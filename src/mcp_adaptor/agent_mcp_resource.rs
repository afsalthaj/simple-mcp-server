use futures::future::BoxFuture;
use futures::FutureExt;
use rmcp::ErrorData;
use rmcp::handler::server::tool::{CallToolHandler, ToolCallContext};
use rmcp::model::{CallToolResult, JsonObject};
use serde_json::json;
use crate::golem::AgentMethod;

#[derive(Clone)]
pub struct AgentMcpResource {
    pub resource: AgentMethod,
}

// Handlers and mapper instances to go  in here
