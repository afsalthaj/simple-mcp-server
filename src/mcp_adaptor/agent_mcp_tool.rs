use futures::future::BoxFuture;
use futures::FutureExt;
use rmcp::ErrorData;
use rmcp::handler::server::tool::{CallToolHandler, ToolCallContext};
use rmcp::model::{CallToolResult, JsonObject};
use serde_json::json;
use crate::golem::{AgentMethod, ElementSchema};
use crate::mcp_adaptor::agent_mcp_server::GolemAgentMcpServer;
use crate::mcp_adaptor::mcp_schema::{McpToolSchema, McpToolSchemaMapper};

#[derive(Clone)]
pub struct AgentMcpTool {
    pub tool: AgentMethod,
}

// While `CallToolHandler` is auto implemented by `tool_handler` macro usually
// but in our case this is manually
// in SDK given a tool annotated function
// but in our case there is no tool or resource annotation

impl CallToolHandler<GolemAgentMcpServer, ()> for AgentMcpTool {
    fn call(
        self,
        context: ToolCallContext<'_, GolemAgentMcpServer>,
    ) -> BoxFuture<'_, Result<CallToolResult, ErrorData>> {
        let _arguments: Option<JsonObject> = context.arguments;

        async move {
            Ok(CallToolResult::structured(
                json!({"result": "example output"})
            ))
        }
            .boxed()
    }
}


impl McpToolSchemaMapper for AgentMcpTool {
    fn get_schema(&self) -> McpToolSchema {
        let mut properties = serde_json::Map::new();
        for (param_name, element_schema) in self.tool.input_schema.iter() {
            // For simplicity, we treat element_schema as a string describing the type
            // In a real implementation, this would be more complex and handle nested structures
            let schema = match element_schema {
                ElementSchema::String => json!({"type": "string"}), // We will be port this POC soon to Golem where the match on is ElementSchema I guess
                ElementSchema::U32 => json!({"type": "integer"}),
                ElementSchema::Bool => json!({"type": "boolean"}),
                _ => json!({"type": "string"}), // Default to string for unknown types
            };
            properties.insert(param_name.clone(), schema);
        }
        let input_schema: JsonObject = json!({
            "type": "object",
            "properties": properties,
        })
            .as_object()
            .unwrap()
            .clone();


        let mut properties = serde_json::Map::new();

        if (self.tool.output_schema.is_empty()) {
            return McpToolSchema {
                input_schema,
                output_schema: None,
            };
        }

        for (param_name, element_schema) in self.tool.input_schema.iter() {
            // For simplicity, we treat element_schema as a string describing the type
            // In a real implementation, this would be more complex and handle nested structures
            let schema = match element_schema {
                ElementSchema::String => json!({"type": "string"}), // We will be port this POC soon to Golem where the match on is ElementSchema I guess
                ElementSchema::U32 => json!({"type": "integer"}),
                ElementSchema::Bool => json!({"type": "boolean"}),
                _ => json!({"type": "string"}), // Default to string for unknown types
            };
            properties.insert(param_name.clone(), schema);
        }
        let output_schema: JsonObject = json!({
           "type": "object",
           "properties": properties,
       })
            .as_object()
            .unwrap()
            .clone();

        McpToolSchema {
            input_schema,
            output_schema: Some(output_schema),
        }
    }
}