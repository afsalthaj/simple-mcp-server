#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;
use axum::extract::{Path, Query, State};
use axum::routing::any;
use futures::future::BoxFuture;
use futures::FutureExt;
use rmcp::handler::server::tool::{CallToolHandler, ToolCallContext};
use rmcp::{
    handler::server::router::tool::ToolRouter, model::*, service::RequestContext, task_handler,
    task_manager::OperationProcessor, tool_handler, ErrorData as McpError, RoleServer,
    ServerHandler,
};
use serde_json::json;
use tokio::sync::{Mutex, RwLock};

use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    {self},
};


// This is not an example of rmcp but an example of how it could work with Golem

// This is what I think we should do in Golem and create an instance of CallToolHandler
// Direct use of SDK is much more simpler because these are all "macro" handled,
// but unfortunately we will have these lower level details popped up in golem code base.
#[derive(Clone)]
pub struct AgentMethodMcpBridge {
    method: AgentMethod,
}


// In golem we will have exactly this to a great extent.
// An AgentMethod
impl CallToolHandler<GolemAgentMcpServer, ()> for AgentMethodMcpBridge {
    fn call(
        self,
        context: ToolCallContext<'_, GolemAgentMcpServer>,
    ) -> BoxFuture<'_, Result<CallToolResult, ErrorData>> {
        async move {
            Ok(CallToolResult::structured(
                json!({"result": "example output"})
            ))
        }
            .boxed()
    }
}

const BIND_ADDRESS: &str = "127.0.0.1:8000";

// Not good...........
// Must find ways to avoid it.
// THis is because to form the routing we need to know the agent_id
//  Ok(GolemAgentMcpServer::new(agent_id)))
// but then we cannot build it for all the requests that happened during handshake
type ServiceMap = Arc<RwLock<HashMap<AgentId, StreamableHttpService<
    GolemAgentMcpServer,
    LocalSessionManager
>>>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();
    let ct = tokio_util::sync::CancellationToken::new();

    let services: ServiceMap = Arc::new(RwLock::new(HashMap::new()));


    let router = axum::Router::new().route("/mcp/{agent_id}", any(mcp_entry).with_state(
        services
    ));

    let tcp_listener = tokio::net::TcpListener::bind(BIND_ADDRESS).await?;

    let _ = axum::serve(tcp_listener, router)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.unwrap();
            ct.cancel();
        })
        .await;
    Ok(())
}

async fn mcp_entry(
    State(services): State<ServiceMap>,
    Path(agent_id): Path<String>,
    req: axum::http::Request<axum::body::Body>,
) -> impl axum::response::IntoResponse {

    if let Some(service) = services.read().await.get(&agent_id) {
        return service.handle(req).await;
    }

    let service = StreamableHttpService::new(
        {
            let agent_id = agent_id.clone();
            move || Ok(GolemAgentMcpServer::new(agent_id.clone()))
        },
        LocalSessionManager::default().into(), // This I think needs to be distributed. otherwise handhshake will fail
        StreamableHttpServerConfig::default(),
    );

    services.write().await.insert(agent_id.clone(), service.clone());

    service.handle(req).await
}

type ParameterName = String;
type ElementSchema = String;

type DataSchema = Vec<(ParameterName, ElementSchema)>; // A simple representation of input schema

// This schema mapping should be a task
fn get_schema(input: DataSchema) -> JsonObject {
    let mut properties = serde_json::Map::new();
    for (param_name, element_schema) in input {
        // For simplicity, we treat element_schema as a string describing the type
        // In a real implementation, this would be more complex and handle nested structures
        let schema = match element_schema.as_str() {
            "string" => json!({"type": "string"}), // We will be port this POC soon to Golem where the match on is ElementSchema I guess
            "integer" => json!({"type": "integer"}),
            "boolean" => json!({"type": "boolean"}),
            _ => json!({"type": "string"}), // Default to string for unknown types
        };
        properties.insert(param_name, schema);
    }
    json!({
        "type": "object",
        "properties": properties,
    })
    .as_object()
    .unwrap()
    .clone()
}

#[derive(Clone)]
pub struct AgentMethod {
    method_name: String,
    input_schema: DataSchema,
    output_schema: DataSchema,
}

type AgentId = String;

pub fn get_agent_tool_and_handlers(agent_id: &AgentId) -> Vec<(Tool, AgentMethodMcpBridge)> {
    // just dummy,
    let agent_method = AgentMethod {
        method_name: "example_method".into(),
        input_schema: vec![
            ("param1".into(), "string".into()),
            ("param2".into(), "integer".into()),
        ],
        output_schema: vec![("result".into(), "string".into())],
    };

    let input_schema = get_schema(agent_method.input_schema);
    let output_schema = get_schema(agent_method.output_schema);

    vec![(
        Tool {
            name: "example_method".into(),
            title: None,
            description: Some("An example method".into()),
            input_schema: Arc::new(input_schema),
            output_schema: Some(Arc::new(output_schema)),
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        },
        AgentMethodMcpBridge {
            method: AgentMethod {
                method_name: "example_method".into(),
                input_schema: vec![
                    ("param1".into(), "string".into()),
                    ("param2".into(), "integer".into()),
                ],
                output_schema: vec![("result".into(), "string".into())],
            },
        },
    )]
}


#[derive(Clone)] // required unfortunately
pub struct GolemAgentMcpServer {
    tool_router: ToolRouter<GolemAgentMcpServer>,
    processor: Arc<Mutex<OperationProcessor>>,
}

impl GolemAgentMcpServer {
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            tool_router: Self::tool_router(agent_id),
            processor: Arc::new(Mutex::new(OperationProcessor::new())),
        }
    }

    fn tool_router(agent_id: AgentId) -> ToolRouter<GolemAgentMcpServer> {
        let tool_handlers = get_agent_tool_and_handlers(&agent_id);

        let mut router = ToolRouter::<Self>::new();

        for (tool, method_handler) in tool_handlers {
            router = router.with_route((tool, method_handler));
        }

        router
    }
}

#[tool_handler(meta = Meta(rmcp::object!({"tool_meta_key": "tool_meta_value"})))]
#[task_handler]
impl ServerHandler for GolemAgentMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_prompts()
                .enable_resources()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("This server provides  tools related to agent in golem and prompts. Tools: increment, decrement, get_value, say_hello, echo, sum. Prompts: example_prompt (takes a message), counter_analysis (analyzes counter state with a goal).".to_string()),
        }
    }

    async fn read_resource(
        &self,
        ReadResourceRequestParams { meta: _, uri }: ReadResourceRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        match uri.as_str() {
            "str:////Users/to/some/path/" => {
                let cwd = "/Users/to/some/path/";
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(cwd, uri)],
                })
            }
            "memo://insights" => {
                let memo = "Business Intelligence Memo\n\nAnalysis has revealed 5 key insights ...";
                Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(memo, uri)],
                })
            }
            _ => Err(McpError::resource_not_found(
                "resource_not_found",
                Some(json!({
                    "uri": uri
                })),
            )),
        }
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: Vec::new(),
            meta: None,
        })
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        if let Some(http_request_part) = context.extensions.get::<axum::http::request::Parts>() {
            let initialize_headers = &http_request_part.headers;
            let initialize_uri = &http_request_part.uri;
            dbg!("here are the initialize headers ", initialize_headers);
            tracing::info!(?initialize_headers, %initialize_uri, "initialize from http server");
        }
        Ok(self.get_info())
    }
}
