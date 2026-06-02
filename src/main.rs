//! `weather-mcp` — entry point.
//!
//! A thin shell: init tracing, select the data source (fixture-backed vs. real HTTP, via
//! [`weather_mcp::client_from_env`]), build the [`WeatherServer`], and serve it over stdio.
//! Logs go to **stderr** — stdout is the MCP stdio channel (see ARCHITECTURE.md).
//!
//! The transport seam: `serve` takes any transport, so swapping `stdio()` for streamable-HTTP
//! later won't touch the tool implementations.

use rmcp::{transport::stdio, ServiceExt};
use tracing_subscriber::EnvFilter;

use weather_mcp::server::WeatherServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Structured logging to stderr. `with_ansi(false)` keeps the stream clean for capture;
    // `RUST_LOG` (env-filter) controls verbosity, defaulting to info.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let client = weather_mcp::client_from_env();
    tracing::info!("starting weather-mcp server (stdio transport)");

    let service = WeatherServer::new(client)
        .serve(stdio())
        .await
        .inspect_err(|err| tracing::error!(?err, "failed to start MCP service"))?;

    service.waiting().await?;
    Ok(())
}
