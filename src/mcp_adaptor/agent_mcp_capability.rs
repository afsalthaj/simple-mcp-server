use crate::golem::AgentMethod;
use crate::mcp_adaptor::agent_mcp_resource::AgentMcpResource;
use crate::mcp_adaptor::agent_mcp_tool::AgentMcpTool;

#[derive(Clone)]
pub enum McpAgentCapability {
    Tool(AgentMcpTool),
    Resource(AgentMcpResource),
}



impl McpAgentCapability {

    // Infallible
    pub fn from(method: AgentMethod) -> Self {
        // Based on mapping rules
        if method.input_schema.len() > 0 {
            Self::Tool(AgentMcpTool { tool: method })
        } else {
            Self::Resource(AgentMcpResource { resource: method })
        }
    }
}
