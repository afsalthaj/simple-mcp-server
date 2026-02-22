// This is an example of MCP server where URL is not parameterized by agent-id

use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager,
    StreamableHttpServerConfig,
    StreamableHttpService,
};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use poem::{
    listener::TcpListener,
    Route,
    Server,
};
use poem::endpoint::TowerCompatExt;

use mcp_server::mcp_adaptor::GolemAgentMcpServer;

const BIND_ADDRESS: &str = "127.0.0.1:8000";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cancellation_token = tokio_util::sync::CancellationToken::new();
    let shutdown_token = cancellation_token.clone();

    // Base rmcp tower service
    let service = StreamableHttpService::new(
        move || Ok(GolemAgentMcpServer::new(None)),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default(),
    );

    // Convert tower service → Poem endpoint
    let app = Route::new()
        .nest("/mcp", service.compat());

    Server::new(TcpListener::bind(BIND_ADDRESS))
        .run_with_graceful_shutdown(
            app,
            async move {
                tokio::signal::ctrl_c().await.unwrap();
                shutdown_token.cancel();
            },
            None,
        )
        .await?;

    Ok(())
}