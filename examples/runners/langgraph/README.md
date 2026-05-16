# ADP ↔ LangGraph Round-Trip Runner

Demonstrates building a LangGraph `StateGraph` from an ADP manifest and reconstructing the manifest structure from the compiled graph.

## Quick Start

```bash
pip install -r requirements.txt
pip install -e ../../../sdk/python
pytest -v
```

## What Each Test Verifies

| Test | What it checks |
|---|---|
| `test_graph_has_correct_nodes` | Compiled LangGraph graph contains exactly the node IDs declared in the ADP manifest |
| `test_graph_invocation_populates_state` | Invoking the graph populates `context[node.id]` for LLM nodes (ADR D2) and `tool_responses[node.id]` for tool nodes (ADR D3) |
| `test_round_trip_node_ids` | ADP → LangGraph → ADP preserves all node IDs |
| `test_round_trip_edge_connections` | ADP → LangGraph → ADP preserves all edge `from`/`to` pairs |

## Round-Trip Limitations

`adp_from_langgraph` provides **structural fidelity**, not full manifest equality:

- **Preserved**: node IDs, edge connections, start/end nodes
- **Recovered from `adp_node_map`**: `model_ref`, `suite_ref`, `runtime_ref`, `kind`, and all other ADP node fields — LangGraph does not store ADP metadata natively on `add_node`
- **Lost without `adp_node_map`**: non-structural ADP fields cannot be recovered from LangGraph introspection alone

## Architecture

```
build_langgraph_from_adp(manifest, backend_factory)
  → (CompiledStateGraph, adp_node_map: dict[str, dict])

adp_from_langgraph(graph, adp_node_map, original_manifest)
  → partial ADP manifest (structural round-trip)
```

`adp_node_map` is the bridge — always pass it from `build_langgraph_from_adp` to `adp_from_langgraph`.

## Links

- [Framework Interop Guide](../../../spec/framework-interop.md) — full LangGraph/AutoGen/SK/CrewAI mapping
- [Runtime-Flow Binding](../../../spec/runtime-flow-binding.md) — backend compatibility matrix and condition expression format
- [ADP Spec](../../../spec/adp-v0.1.0.md) — full protocol specification
