use futures::future::BoxFuture;
use futures::FutureExt;
use rmcp::ErrorData;
use rmcp::handler::server::prompt::{GetPromptHandler, PromptContext};
use rmcp::model::{GetPromptResult, PromptMessage, PromptMessageContent, PromptMessageRole};
use crate::golem::AgentMethod;
use crate::mcp_adaptor::{GolemAgentMcpServer};

#[derive(Clone)]
pub struct AgentMcpPrompt {
    pub agent_method: AgentMethod,
}

impl GetPromptHandler<GolemAgentMcpServer, ()> for AgentMcpPrompt {
    fn handle(self, context: PromptContext<'_, GolemAgentMcpServer>) -> BoxFuture<'_, Result<GetPromptResult, ErrorData>> {
        async move {

            let parameters = context.arguments.map(|x| x.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join(", ")).unwrap_or_else(|| "no parameters".to_string());
            
            let result = GetPromptResult {
                description: None,
                messages: vec![
                    PromptMessage {
                        role: PromptMessageRole::User,
                        content: PromptMessageContent::Text {
                            text: format!("{}, call {} with the following parameters: {}",  "developer-given prompt" , self.agent_method.method_name, parameters)
                        } ,
                    }
                ]
            };

            Ok(result)
        }
        .boxed()
    }
}
