// This is an example of MCP server per agent-id. Altogether I felt a little awkward
// to implement (it's implementation details not conceptual issues).
// Implementation is complex rmcp unlike a global MCP server per domain which can route
// to various agent instances, in which case the tools embed the information of how
// the agent should be constructed (similar to code first routes). So this particular POC code
// is second priority compared to `rmcp_default_server.rs`. If you haven't read that, please read
// before reading this one.

use std::collections::HashMap;
use std::sync::Arc;
use axum::extract::{Path, State};
use axum::routing::any;
use tokio::sync::{RwLock};

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

/*
This service-map is pretty much RMCP specific. I believe we can avoid this complexity though.
There is a way out to this although introducing more code, but its mechanical. (We will detail this)

Wiring in AgentId from the MCP URL was not a great experience due
to rmcp specific details where anything starts of with implemnnting that eagerly computed list of tools.
This is not efficiently use of handshake of MCP where `initialize` can decide the list of tools to be exposed.
Anyway, we can get over this limitation by making a few changes.
*/

pub type ServiceMap = Arc<RwLock<HashMap<AgentId, StreamableHttpService<
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
            move || Ok(GolemAgentMcpServer::new(Some(agent_id.clone())))
        },
        LocalSessionManager::default().into(), // This I think needs to be distributed. otherwise handhshake will fail
        StreamableHttpServerConfig::default(),
    );

    services.write().await.insert(agent_id.clone(), service.clone());

    service.handle(req).await
}
