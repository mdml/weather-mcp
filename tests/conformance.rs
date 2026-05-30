//! MCP conformance test — the verifier that can't be gamed with mocks (ADR 0005).
//!
//! Spawns the **built `weather-mcp` binary** as a child process and scripts a real MCP
//! client session over stdio: `initialize` -> `tools/list` -> `tools/call(server_info)`.
//! Also pins the `tools/list` shape and the `server_info` result JSON with `insta`
//! snapshots so accidental schema/output drift trips the verifier.

use rmcp::{
    model::CallToolRequestParams,
    transport::{ConfigureCommandExt, TokioChildProcess},
    ServiceExt,
};
use serde_json::{json, Value};
use tokio::process::Command;

/// Absolute path to the freshly-built test binary, injected by Cargo/nextest.
const SERVER_BIN: &str = env!("CARGO_BIN_EXE_weather-mcp");

/// Spawn the server as a child process and connect a client (this also performs the MCP
/// `initialize` handshake).
async fn connect() -> anyhow::Result<rmcp::service::RunningService<rmcp::RoleClient, ()>> {
    let transport = TokioChildProcess::new(Command::new(SERVER_BIN).configure(|cmd| {
        // Quiet logs; stderr stays out of the stdout MCP channel regardless.
        cmd.env("RUST_LOG", "warn");
    }))?;
    let client = ().serve(transport).await?;
    Ok(client)
}

#[tokio::test]
async fn initialize_reports_server_identity() -> anyhow::Result<()> {
    let client = connect().await?;

    // `peer_info()` is the server's `initialize` response.
    let info = client
        .peer_info()
        .expect("server must report info after initialize");
    assert_eq!(info.server_info.name, "weather-mcp");
    assert!(
        info.capabilities.tools.is_some(),
        "server must advertise the tools capability"
    );

    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn tools_list_contains_server_info() -> anyhow::Result<()> {
    let client = connect().await?;

    let tools = client.list_tools(Default::default()).await?;
    let names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
    assert_eq!(
        names,
        vec!["server_info"],
        "Phase 0 exposes exactly the server_info tool"
    );

    // Snapshot the tool's public shape (name + description + input schema). Pinning this
    // catches accidental changes to the advertised tool surface.
    let tool = &tools.tools[0];
    let shape = json!({
        "name": tool.name,
        "description": tool.description,
        "input_schema": tool.input_schema,
    });
    insta::assert_json_snapshot!("tools_list_server_info", shape);

    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn call_server_info_returns_static_identity() -> anyhow::Result<()> {
    let client = connect().await?;

    let result = client
        .call_tool(CallToolRequestParams::new("server_info"))
        .await?;

    assert_ne!(result.is_error, Some(true), "server_info must not error");

    // The result content is a single JSON block; parse it back and assert the identity,
    // then snapshot the exact JSON shape.
    let content = result
        .content
        .first()
        .expect("server_info returns one content block");
    let text = content
        .as_text()
        .expect("server_info content is text-encoded JSON");
    let parsed: Value = serde_json::from_str(&text.text)?;

    assert_eq!(parsed["name"], "weather-mcp");
    assert_eq!(
        parsed["description"],
        Value::String(
            "MCP server wrapping the Open-Meteo API (forecast + historical trends).".to_string(),
        )
    );
    assert!(parsed["version"].is_string());

    insta::assert_json_snapshot!("server_info_result", parsed, {
        // Version tracks the crate version and will bump over time — redact so the
        // snapshot pins the shape, not the exact number.
        ".version" => "[version]"
    });

    client.cancel().await?;
    Ok(())
}
