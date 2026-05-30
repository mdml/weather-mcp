//! `weather-mcp` — entry point.
//!
//! Phase 0 skeleton: build the [`server::WeatherServer`] and serve it over stdio.
//! Logs go to **stderr** — stdout is the MCP stdio channel (see ARCHITECTURE.md).

mod compare;
mod openmeteo;
mod server;

use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::EnvFilter;

use crate::server::WeatherServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Structured logging to stderr. `with_ansi(false)` keeps the stream clean for
    // capture; `RUST_LOG` (env-filter) controls verbosity, defaulting to info.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("starting weather-mcp server (stdio transport)");

    // The transport seam (ARCHITECTURE.md): `serve` takes any transport, so swapping
    // `stdio()` for streamable-HTTP in Phase 3 won't touch the tool implementations.
    let service = WeatherServer::new()
        .serve(stdio())
        .await
        .inspect_err(|err| tracing::error!(?err, "failed to start MCP service"))?;

    service.waiting().await?;
    Ok(())
}
