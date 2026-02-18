use axum::{
    extract::Json,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::post,
    Router, Server,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/mcp", post(mcp_handler));

    println!("MCP server running on http://0.0.0.0:8080/mcp");

    Server::bind(&"0.0.0.0:8080".parse::<SocketAddr>().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
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

    // ========================================
    // All other calls require session header
    // ========================================
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

    // ========================================
    // tools/list
    // ========================================
    if method == "tools/list" {
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [
                    {
                        "name": "hello_agent",
                        "title": "Hello Agent",
                        "description": "Returns a greeting",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" }
                            },
                            "required": ["name"]
                        }
                    }
                ]
            }
        });

        return (StatusCode::OK, HeaderMap::new(), Json(body));
    }

    // ========================================
    // tools/call
    // ========================================
    if method == "tools/call" {
        let params = payload.get("params").cloned().unwrap_or(json!({}));
        let tool_name = params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if tool_name != "hello_agent" {
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

        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("World");

        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": format!("Hello, {}!", name)
                    }
                ],
                "isError": false
            }
        });

        return (StatusCode::OK, HeaderMap::new(), Json(body));
    }

    // ========================================
    // Unknown method
    // ========================================
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
