// This is an example of MCP server  where URL is not parmeter by agent-id
// In this case, the tool listing will ensure to include agent_type_name within the tool name,
// and constructor signature within the tool schema

use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    {self},
};
use mcp_server::golem::AgentId;
use mcp_server::mcp_adaptor::GolemAgentMcpServer;

const BIND_ADDRESS: &str = "127.0.0.1:8000";

// structure of flattened version
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();
    let ct = tokio_util::sync::CancellationToken::new();

    let service = StreamableHttpService::new(
        {
            move || Ok(GolemAgentMcpServer::new(None))
        },
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default(),
    );

   
    let router = axum::Router::new().nest_service("/mcp", service);
    
    let tcp_listener = tokio::net::TcpListener::bind(BIND_ADDRESS).await?;

    let _ = axum::serve(tcp_listener, router)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.unwrap();
            ct.cancel();
        })
        .await;
    Ok(())
}
