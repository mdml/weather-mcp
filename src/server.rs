//! The MCP server handler.
//!
//! Phase 0 exposes exactly one trivial, deterministic, network-free tool:
//! [`WeatherServer::server_info`]. The real weather tools (`get_forecast`,
//! `get_historical`, `compare_period`) arrive in Phase 1 — see ARCHITECTURE.md.
//!
//! The server type is deliberately **independent of the transport** (the transport
//! seam): `serve(...)` in `main.rs` decides stdio vs. HTTP.

use rmcp::{
    handler::server::router::tool::ToolRouter,
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use serde::{Deserialize, Serialize};

/// Static identity for the server, surfaced both in `initialize` (via [`get_info`]) and
/// from the `server_info` tool. Kept as constants so the conformance/snapshot tests are
/// fully deterministic and don't depend on build-env quirks.
///
/// [`get_info`]: WeatherServer::get_info
pub const SERVER_NAME: &str = "weather-mcp";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SERVER_DESCRIPTION: &str =
    "MCP server wrapping the Open-Meteo API (forecast + historical trends).";

/// The static payload returned by the `server_info` tool.
///
/// Derives `JsonSchema` so the (empty) output shape is well-formed; `Serialize` for the
/// tool result and `Deserialize` so tests can parse it back if needed.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ServerInfoResult {
    /// Server name (matches the MCP `initialize` server name).
    pub name: String,
    /// Server version (the crate version).
    pub version: String,
    /// Human-readable description of what the server does.
    pub description: String,
}

impl ServerInfoResult {
    fn current() -> Self {
        Self {
            name: SERVER_NAME.to_string(),
            version: SERVER_VERSION.to_string(),
            description: SERVER_DESCRIPTION.to_string(),
        }
    }
}

/// The weather MCP server. Phase 0 holds only the tool router; Phase 1 adds the
/// Open-Meteo client alongside it.
#[derive(Clone)]
pub struct WeatherServer {
    // Read by the `#[tool_handler]`-generated `call_tool`/`list_tools` impl; dead-code
    // analysis doesn't see through the macro, hence the allow.
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl Default for WeatherServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl WeatherServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// Return static identity (name/version/description) for this server as JSON.
    ///
    /// Deterministic and network-free — the canary that proves the MCP tool path works
    /// end-to-end before any real weather logic exists.
    #[tool(description = "Return this server's name, version, and description as JSON.")]
    async fn server_info(&self) -> Result<CallToolResult, McpError> {
        let info = ServerInfoResult::current();
        Ok(CallToolResult::success(vec![Content::json(info)?]))
    }
}

#[tool_handler]
impl ServerHandler for WeatherServer {
    fn get_info(&self) -> ServerInfo {
        // `ServerInfo` / `Implementation` are `#[non_exhaustive]` (defined in rmcp), so
        // we can't use struct-literal syntax — start from `default()` and assign fields.
        let mut server_info = Implementation::default();
        server_info.name = SERVER_NAME.to_string();
        server_info.version = SERVER_VERSION.to_string();

        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info = server_info;
        info.instructions = Some(SERVER_DESCRIPTION.to_string());
        info
    }
}
