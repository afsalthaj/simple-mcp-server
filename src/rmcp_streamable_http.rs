// This is an example of MCP server per agent-id. Altogether I felt a little awkward
// to implement (it's implementation details not conceptual issues).
// Implementation is complex rmcp unlike a global MCP server per domain which can route
// to various agent instances, in which case the tools embed the information of how
// the agent should be constructed (similar to code first routes). So this particular POC code
// is second priority compared to `rmcp_default_server.rs`. If you haven't read that, please read
// before reading this one.

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


// This is what I think we should do in Golem and create an instance of CallToolHandler
// Direct use of SDK is much more simpler because these are all "macro" handled,
// but unfortunately we will have these lower level details popped up in golem code base.
#[derive(Clone)]
pub struct AgentMethodMcpBridge {
    method: AgentMethod,
}


// While `CallToolHandler` is auto implemented by `tool_handler` macro usually
// but in our case this is manually
// in SDK given a tool annotated function
// but in our case there is no tool or resource annotation

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

/*
This service-map is pretty much RMCP specific. I believe we can avoid this complexity though.
There is a way out to this although introducing more code, but its mechanical. (We will detail this)

Wiring in AgentId from the MCP URL was not a great experience due
to rmcp specific details where anything starts of with implemnnting that eagerly computed list of tools.
This is not efficiently use of handshake of MCP where `initialize` can decide the list of tools to be exposed.
Anyway, we can get over this limitation by making a few changes.
*/

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


    println!("Method       : {:?}", req.method());
    println!("Version      : {:?}", req.version());
    println!("URI (full)   : {:?}", req.uri());
    println!("Path         : {:?}", req.uri().path());
    println!("Query        : {:?}", req.uri().query());
    println!("Headers      : {:#?}", req.headers());
    println!("Agent ID     : {:?}", agent_id);

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

type ParameterName = String; // parameter name in the method signature
type ElementSchema = String; // name of th etype

type DataSchema = Vec<(ParameterName, ElementSchema)>; // A simple representation of input schema


trait McpSchemaMapper {
    fn get_json_object_schema(&self) -> JsonObject;
}

struct DataSchemaMcpBridge {
    schema: DataSchema, // mny be we need separate bridge for input and output because of the parmeter name in result type is not really needed
}

impl McpSchemaMapper for DataSchemaMcpBridge {
    fn get_json_object_schema(&self) -> JsonObject {
        let mut properties = serde_json::Map::new();
        for (param_name, element_schema) in self.schema.iter() {
            // For simplicity, we treat element_schema as a string describing the type
            // In a real implementation, this would be more complex and handle nested structures
            let schema = match element_schema.as_str() {
                "string" => json!({"type": "string"}), // We will be port this POC soon to Golem where the match on is ElementSchema I guess
                "integer" => json!({"type": "integer"}),
                "boolean" => json!({"type": "boolean"}),
                _ => json!({"type": "string"}), // Default to string for unknown types
            };
            properties.insert(param_name.clone(), schema);
        }
        json!({
        "type": "object",
        "properties": properties,
    })
            .as_object()
            .unwrap()
            .clone()
    }
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

    let mcp_data_schema_input = DataSchemaMcpBridge {
        schema: agent_method.input_schema.clone(),
    };

        let mcp_data_schema_output = DataSchemaMcpBridge {
            schema: agent_method.output_schema.clone(),
        };

    let input_schema =  mcp_data_schema_input.get_json_object_schema();
    let output_schema = mcp_data_schema_output.get_json_object_schema();

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

// Almost all macros in rmcp was useless for us (and that's expected - and we are not using it for these helpers anyway).
// We avoided it for the most part except for the below one, and that should also disappear.
// It works for now. However, we should copy the relevant implenetations such as list-tools
// and then avoid design flaws such as expecting listing to be pre-computed in GolemAgentMcpServer rather
// than mcp-initialize phase deciding it. This will make the code far better in architecture and MCP lifecycle
// although obviously verbose
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
            println!("\n================== MCP INITIALIZE ==================");

            if let Some(http_parts) = context.extensions.get::<axum::http::request::Parts>() {
                println!("Version      : {:?}", http_parts.version);
                println!("URI          : {:?}", http_parts.uri);
                println!("Path         : {:?}", http_parts.uri.path());
                println!("Query        : {:?}", http_parts.uri.query());
                println!("Headers      : {:#?}", http_parts.headers);
            } else {
                println!("No HTTP parts found in extensions");
            }

            println!("=====================================================\n");

            let initialize_headers = &http_request_part.headers;
            let initialize_uri = &http_request_part.uri;
            dbg!("here are the initialize headers ", initialize_headers);
            tracing::info!(?initialize_headers, %initialize_uri, "initialize from http server");
        }
        Ok(self.get_info())
    }
}
