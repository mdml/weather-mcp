//! MCP conformance — the verifier that can't be gamed with mocks (ADR 0005, test-plan §3.5).
//!
//! Spawns the **built `weather-mcp` binary** as a child process, with the fixture-backed client
//! selected via `WEATHER_MCP_FIXTURES`, and scripts a real MCP client session over stdio.
//!
//! `initialize` and `tools/list` (names + schemas) register the three tools and pin their request
//! schemas by snapshot. The `tools/call` paths drive each tool end-to-end against the
//! fixture-backed client and assert the result snapshots. This also covers the §3.4 handler
//! snapshots.

use rmcp::{
    model::CallToolRequestParams,
    transport::{ConfigureCommandExt, TokioChildProcess},
    ServiceExt,
};
use serde_json::{json, Map, Value};
use tokio::process::Command;

const SERVER_BIN: &str = env!("CARGO_BIN_EXE_weather-mcp");

/// Absolute path to the committed fixtures, handed to the child via env.
fn fixtures_dir() -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .to_string_lossy()
        .into_owned()
}

/// Spawn the server (fixture-backed) and connect a client (performs the `initialize` handshake).
async fn connect() -> anyhow::Result<rmcp::service::RunningService<rmcp::RoleClient, ()>> {
    let fixtures = fixtures_dir();
    let transport = TokioChildProcess::new(Command::new(SERVER_BIN).configure(|cmd| {
        cmd.env("RUST_LOG", "warn");
        cmd.env("WEATHER_MCP_FIXTURES", &fixtures);
    }))?;
    Ok(().serve(transport).await?)
}

fn args(value: Value) -> Map<String, Value> {
    value.as_object().expect("args object").clone()
}

#[tokio::test]
async fn initialize_reports_server_identity() -> anyhow::Result<()> {
    let client = connect().await?;
    let info = client
        .peer_info()
        .expect("server must report info after initialize");
    assert_eq!(info.server_info.name, "weather-mcp");
    assert!(
        info.capabilities.tools.is_some(),
        "tools capability advertised"
    );
    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn tools_list_is_exactly_the_three_weather_tools() -> anyhow::Result<()> {
    let client = connect().await?;
    let tools = client.list_tools(Default::default()).await?;

    let mut names: Vec<&str> = tools.tools.iter().map(|t| t.name.as_ref()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        vec!["compare_period", "get_forecast", "get_historical"],
        "exactly the three weather tools"
    );

    // Regression guard: every tool's input schema must be self-contained — no `$ref`/`$defs`
    // indirection. Many MCP clients / LLM tool-callers don't dereference `$ref`, so a schema that
    // hides a struct/enum behind `$defs` makes them mis-serialize the param (the `compare_period`
    // string-vs-object bug). Inlining (`#[schemars(inline)]` + flat params) keeps this empty; a
    // future change that reintroduces indirection fails loudly here.
    for tool in &tools.tools {
        let schema =
            serde_json::to_string(&tool.input_schema).expect("input schema serializes to a string");
        assert!(
            !schema.contains("\"$ref\""),
            "tool `{}` input schema must not contain `$ref` (MCP clients don't deref it): {schema}",
            tool.name,
        );
        assert!(
            !schema.contains("\"$defs\""),
            "tool `{}` input schema must not contain `$defs` (inline the subschemas): {schema}",
            tool.name,
        );
    }

    // Snapshot the published shape (name + description + input schema) of all three, sorted for
    // determinism. Pinning this catches accidental drift of the advertised tool contract.
    let mut shapes: Vec<Value> = tools
        .tools
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.input_schema,
            })
        })
        .collect();
    shapes.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    insta::assert_json_snapshot!("tools_list", shapes);

    client.cancel().await?;
    Ok(())
}

// ---- tools/call each tool end-to-end against the fixture client -------------------------------

#[tokio::test]
async fn call_get_forecast_returns_success() -> anyhow::Result<()> {
    let client = connect().await?;
    let result = client
        .call_tool(
            CallToolRequestParams::new("get_forecast")
                .with_arguments(args(json!({ "location": "Boston", "forecast_days": 7 }))),
        )
        .await?;
    assert_ne!(result.is_error, Some(true), "get_forecast should succeed");
    insta::assert_json_snapshot!("call_get_forecast", result.structured_content);
    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn call_get_historical_returns_success() -> anyhow::Result<()> {
    let client = connect().await?;
    let result = client
        .call_tool(
            CallToolRequestParams::new("get_historical").with_arguments(args(json!({
                "location": "Boston",
                "start_date": "2020-01-01",
                "end_date": "2020-12-31"
            }))),
        )
        .await?;
    assert_ne!(result.is_error, Some(true), "get_historical should succeed");
    insta::assert_json_snapshot!("call_get_historical", result.structured_content);
    client.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn call_compare_period_returns_success() -> anyhow::Result<()> {
    let client = connect().await?;
    let result = client
        .call_tool(
            CallToolRequestParams::new("compare_period").with_arguments(args(json!({
                "location": "Boston",
                "period_start": "2026-01-01",
                "period_end": "2026-05-25"
            }))),
        )
        .await?;
    assert_ne!(result.is_error, Some(true), "compare_period should succeed");
    insta::assert_json_snapshot!("call_compare_period", result.structured_content);
    client.cancel().await?;
    Ok(())
}
