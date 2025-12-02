# Changelog

## 0.2.0 - Execution Semantics Profile (ESP)

- **New**: Execution Semantics Profile (ESP) defines execution semantics for ADP agents
  - Flow graph execution model (node readiness, edge traversal, state passing)
  - State model with core fields (`inputs`, `context`, `memory`, `tool_responses`)
  - Tool binding semantics via `tool_ref` field
  - Model and prompt reference resolution
  - Error and failure semantics (permanent vs transient)
- **New**: `flow.graph.nodes[].tool_ref` field for explicit tool binding
- **New**: `runtime.models[]` array for explicit model configuration
- **Enhanced**: Conformance requirements for ESP-conformant runners
- **Backward compatible**: All ADP v0.1.0 manifests remain valid

## 0.1.0 - Initial draft

- Introduced ADP, ACS, and ADPKG specifications (v0.1).
- Added example agent and container specs.
- Implemented initial `adp` CLI for validate, pack, unpack, and inspect stub.
