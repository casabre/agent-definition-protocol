# Interoperability Mapping (Informative)

This appendix maps ADP concepts to adjacent specifications to ease integration.

## A2A AgentCard (draft)
- **Identity**: `adp.id` ↔ AgentCard `id`; `name`/`description` align directly.
- **Runtime**: `runtime.execution[]` ↔ AgentCard `execution`/`runtime` fields (container/image references map directly).
- **Tools**: `tools.mcp_servers` ↔ AgentCard MCP registry; `tools.http_apis` ↔ AgentCard HTTP connectors.
- **Flow**: `flow.graph` ↔ AgentCard `graph` (node/edge parity, with ADP `ui` mapping to AgentCard UI hints).
- **Evaluation**: `evaluation.suites` ↔ AgentCard `evaluation` (metric ids/thresholds can be shared).
- **Governance**: `governance.*` ↔ AgentCard `governance` (policy set and telemetry endpoints).

## OpenTelemetry metrics
- **Latency**: `evaluation.metrics[].telemetry.metric` SHOULD reference OTel semantic conventions (e.g., `rpc.server.duration`, `http.server.duration`).
- **Cost/tool usage**: `tool.calls`, `llm.tokens` SHOULD map to custom OTel metrics with clear units (tokens, USD).
- **Namespaces**: Use OTel attributes for `agent.id`, `deployment.environment`, and `runtime.backend.id` to correlate telemetry with ADP definitions.

## Required minimums (ADP-Full)
- A2A: If `interop.a2a.ref` is provided, it MUST resolve to a valid AgentCard; ids SHOULD match. Inline `interop.a2a.agentcard` SHOULD include identity and runtime refs.
- OTel: Telemetry metrics SHOULD include attributes `agent.id`, `deployment.environment`, and `runtime.backend.id` to correlate traces/metrics with ADP runtime entries.
- HTTP/MCP/SQL tools SHOULD surface endpoints/connection info as OTel resource attributes where applicable.
- Traces SHOULD include `flow.id` and `flow.node.id` span attributes for flow execution observability.

Status: Informative for mappings; required minimums above apply to ADP-Full implementations.
