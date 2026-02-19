mod rmcp_streamable_http;

use axum::{
    extract::Json,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::post,
    Router,
};
use serde_json::{json, Value};
use uuid::Uuid;

const BIND_ADDRESS: &str = "127.0.0.1:8000";


#[tokio::main]
async fn main() -> anyhow::Result<()>  {
    let tcp_listener = tokio::net::TcpListener::bind(BIND_ADDRESS).await?;

    let app = Router::new().route("/mcp", post(mcp_handler));


    println!("MCP server running on http://0.0.0.0:8080/mcp");

    axum::serve(tcp_listener, app.into_make_service())
        .await?;

    Ok(())
}

async fn mcp_handler(
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let id = payload.get("id").cloned();
    let method = payload
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // ========================================
    // INITIALIZE
    // ========================================
    if method == "initialize" || method == "initialise" {
        let session_id = Uuid::new_v4().to_string();

        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "tools": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "golem-mcp-dev",
                    "version": "0.1.0"
                }
            }
        });

        let mut response_headers = HeaderMap::new();
        response_headers.insert(
            "Mcp-Session-Id",
            HeaderValue::from_str(&session_id).unwrap(),
        );

        return (StatusCode::OK, response_headers, Json(body));
    }


    if method == "notifications/initialized" {
        return (StatusCode::ACCEPTED, HeaderMap::new(), Json(json!({})));
    }

    if headers.get("Mcp-Session-Id").is_none() {
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32000,
                "message": "Missing Mcp-Session-Id"
            }
        });

        return (StatusCode::BAD_REQUEST, HeaderMap::new(), Json(body));
    }

    if method == "tools/list" {
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [
                    {
                        "name": "counter",
                        "title": "Counter",
                        "description": "Increments a given number",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "number": { "type": "integer" }
                            },
                            "required": ["number"]
                        }
                    }
                ]
            }
        });

        return (StatusCode::OK, HeaderMap::new(), Json(body));
    }

    if method == "tools/call" {
        let params = payload.get("params").cloned().unwrap_or(json!({}));
        let tool_name = params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if tool_name != "counter" {
            let body = json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": "Tool not found"
                }
            });

            return (StatusCode::BAD_REQUEST, HeaderMap::new(), Json(body));
        }

        let args = params
            .get("arguments")
            .cloned()
            .unwrap_or(json!({}));

        let number = args
            .get("number")
            .and_then(|v| v.as_u64())
            .unwrap_or(1);

        let incremented = number + 1;

        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": format!("Hello, {}!", incremented)
                    }
                ],
                "isError": false
            }
        });

        return (StatusCode::OK, HeaderMap::new(), Json(body));
    }

    let body = json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": -32601,
            "message": "Method not found"
        }
    });

    (StatusCode::BAD_REQUEST, HeaderMap::new(), Json(body))
}
