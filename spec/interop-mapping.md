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

## Recommended practices (informative)

These are guidance for implementers, not normative requirements:

- A2A: If `interop.a2a.ref` is provided, implementers are encouraged to ensure it resolves to a valid AgentCard and that ids match. Inline `interop.a2a.agentcard` is encouraged to include identity and runtime refs.
- OTel: Implementers are encouraged to include `agent.id`, `deployment.environment`, and `runtime.backend.id` as OTel attributes to correlate traces with ADP runtime entries.
- HTTP/MCP/SQL tools: Implementers are encouraged to surface endpoint and connection info as OTel resource attributes where applicable.
- Traces: Implementers are encouraged to include `flow.id` and `flow.node.id` span attributes for flow execution observability.
