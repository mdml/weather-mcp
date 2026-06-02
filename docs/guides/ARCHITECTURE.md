# Architecture (planned)

The intended shape, not yet built. The one constraint that must be designed in from the build
(Phase 2 on): a **transport seam** so stdio (now) → HTTP (Phase 5) doesn't force a rewrite
([0006](../decisions/0006-phased-delivery.md)).

## Crate layout

A single small crate to start (split later only if a real need appears):

```
weather-mcp/
├── Cargo.toml
├── rust-toolchain.toml
├── src/
│   ├── main.rs          # entry: tracing to stderr, build server, serve(stdio())
│   ├── server.rs        # the rmcp ServerHandler + #[tool_router] (the three tools)
│   ├── openmeteo/        # the Open-Meteo client (outermost I/O boundary)
│   │   ├── mod.rs
│   │   ├── forecast.rs  # Forecast API
│   │   └── archive.rs   # Historical (ERA5) Archive API
│   └── compare.rs       # compare_period aggregation (pure, deterministic, fixture-tested)
├── tests/
│   ├── conformance.rs   # spawn over stdio: initialize → tools/list → tools/call
│   └── fixtures/         # recorded Open-Meteo responses
└── ...
```

The **aggregation logic (`compare.rs`) is pure and deterministic** — it takes parsed archive
data and produces the comparison, so it's fully fixture-testable offline. The
**`openmeteo/` client is the outermost boundary** — the only thing the live smoke test
(`just test-live`) exercises against the real API. See
[DEVELOPMENT.md](DEVELOPMENT.md) for the testing strategy.

## MCP server with `rmcp` 1.x

The 1.x macro API (differs from 0.x). Illustrative sketch — *not* the implementation:

```rust
#[derive(Clone)]
pub struct WeatherServer { tool_router: ToolRouter<Self>, client: OpenMeteoClient }

#[tool_router]
impl WeatherServer {
    #[tool(description = "Current conditions + N-day forecast for a location")]
    async fn get_forecast(&self, Parameters(req): Parameters<ForecastRequestStub>)
        -> Result<CallToolResult, McpError> { /* ... */ }
    // get_historical, compare_period ...
}

#[tool_handler]
impl ServerHandler for WeatherServer { fn get_info(&self) -> ServerInfo { /* ... */ } }
```

Tool argument structs derive `serde::Deserialize` + `schemars::JsonSchema` (the schema
surfaces in `tools/list`). Tool/parameter names and shapes are cribbed from
`cmer81/open-meteo-mcp` ([0004](../decisions/0004-minimal-tool-surface.md)).

## The transport seam

The server serves over stdio (since the Phase 0 skeleton):

```rust
let service = WeatherServer::new().serve(stdio()).await?;
service.waiting().await?;
```

Keep the server type (`WeatherServer` + its `#[tool_router]`) **independent of the transport**.
`rmcp`'s transport is pluggable (`serve(...)` takes any transport), so Phase 5 swaps
`stdio()` for the streamable-HTTP transport and adds the OAuth/JWT layer **without touching the
tool implementations**. The skeleton (Phase 0) should already prove the stdio path end-to-end
via the conformance test.

## Logging

Logs go to **stderr** (stdout is the MCP stdio channel). `tracing` + `tracing_subscriber` with
an env filter, `with_writer(std::io::stderr)`.

## References

- `rmcp` 1.7.0 — official Rust MCP SDK (`modelcontextprotocol/rust-sdk`).
- `cmer81/open-meteo-mcp` — tool-shape reference (TypeScript).
- `gbrigandi/mcp-server-openmeteo` — lean 4-tool surface reference (Rust skeleton).
