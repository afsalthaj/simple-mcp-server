use std::borrow::Cow;
use std::sync::Arc;
use axum::extract::{Path, Query, State};
use headers::Age;
use rmcp::{
    handler::server::router::tool::ToolRouter, model::*, service::RequestContext, task_handler,
    task_manager::OperationProcessor, tool_handler, ErrorData as McpError, RoleServer,
    ServerHandler,
};
use rmcp::handler::server::router::prompt::PromptRouter;
use serde_json::{json};
use tokio::sync::{Mutex};

use crate::golem::{AgentId, AgentMethod, AgentType, ElementSchema};
use crate::mcp_adaptor::{AgentMcpTool, McpAgentCapability, McpToolSchema, McpToolSchemaMapper};
use crate::mcp_adaptor::agent_mcp_prompt::AgentMcpPrompt;

#[derive(Clone)]
pub struct GolemAgentMcpServer {
    pub tool_router: ToolRouter<GolemAgentMcpServer>,
    pub processor: Arc<Mutex<OperationProcessor>>,
}

impl GolemAgentMcpServer {
    // Supporting per agent-id or fully global with no agent information at all
    pub fn new(agent_id: Option<AgentId>) -> Self {
        Self {
            tool_router: Self::tool_router(agent_id),
            processor: Arc::new(Mutex::new(OperationProcessor::new())),
        }
    }

    fn tool_router(agent_id: Option<AgentId>) -> ToolRouter<GolemAgentMcpServer> {
        let tool_handlers = get_agent_tool_and_handlers(agent_id);

        let mut router = ToolRouter::<Self>::new();

        for (tool, method_handler) in tool_handlers {
            router = router.with_route((tool, method_handler));
        }

        router
    }

    fn prompt_router(agent_id: Option<AgentId>) -> PromptRouter<GolemAgentMcpServer> {
        let prompt_handlers = get_agent_prompt_and_handlers(agent_id);

        let mut router = PromptRouter::<Self>::new();

        for (prompt, prompt_handler) in prompt_handlers {
            router = router.with_route((prompt, prompt_handler));
        }

        router
    }
}

pub fn get_agent_prompt_and_handlers(agent_id: Option<AgentId>) -> Vec<(Prompt, AgentMcpPrompt)> {
    // similar to get_agent_tool_and_handlers, but for prompts
    // prompt name is `get_${method_name}_prompt`
    vec![]
}

pub fn get_agent_tool_and_handlers(agent_id: Option<AgentId>) -> Vec<(Tool, AgentMcpTool)> {

    match agent_id {
        Some(agent) => {
            // just dummy,
            let agent_method = get_agent_methods(&agent);

            let mut tools = vec![];

            for method in agent_method.into_iter() {
                let agent_method_mcp = McpAgentCapability::from(method);

                match agent_method_mcp {
                    McpAgentCapability::Tool(agent_mcp_tool) => {
                        let McpToolSchema {input_schema, output_schema} = agent_mcp_tool.get_schema();
                        let tool = Tool {
                            name: Cow::from(agent_mcp_tool.tool.method_name.clone()),
                            title: None,
                            description: Some("An increment method that takes a number and increment it".into()),
                            input_schema: Arc::new(input_schema),
                            output_schema: output_schema.map(Arc::new),
                            annotations: None,
                            execution: None,
                            icons: None,
                            meta: None,
                        };


                        tools.push((tool, agent_mcp_tool));
                    }
                    McpAgentCapability::Resource(_) => {}
                }
            }

            tools
        },
        None => {
            let agent_types = vec!["agent_type_1".to_string(), "agent_type_2".to_string()];

            for agent_type in &agent_types {
               let agent_method  = get_agent_methods(agent_type);
               let agent_constructor = format!("Constructor for {}", agent_type);

                // and append consturctor schema into the agent_method


           }

            vec![]
        }
    }

}


pub fn get_agent_methods(_agent_id: &AgentType) -> Vec<AgentMethod> {
    vec![
        AgentMethod {
            method_name: "increment".into(),
            input_schema: vec![
                ("number".into(), ElementSchema::U32),
            ],
            output_schema: vec![("result".into(), ElementSchema::U32)],
        }
    ]
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