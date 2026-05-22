# ADP → CrewAI Runner

Demonstrates building a CrewAI Flow from an ADP manifest.

**Import direction:** ADP → CrewAI only.
Export (CrewAI → ADP) is deferred to v0.3.0.

**Target API:** CrewAI Flows (`crewai >= 0.63`). This runner does **not** use the legacy `Crew`/`Task` API.

## Quick Start

```bash
pip install -r requirements.txt
pip install -e ../../../sdk/python
pytest -v
```

If `crewai` is not installed, all tests skip gracefully via `pytest.importorskip`.

## What Each Test Verifies

| Test | What it checks |
|---|---|
| `test_agents_created_for_all_nodes` | `agent_map` contains a CrewAI Agent for every node ID declared in the manifest |
| `test_composition_resolves_before_build` | Composition via `resolve_adp` + CrewAI build yields agents for all resolved nodes |
| `test_start_nodes_represented` | All `start_nodes` from the manifest have entries in `agent_map` |
| `test_router_node_represented` | A node with `kind: router` appears in `agent_map` and receives `@router` treatment |

## Architecture

```
build_crewai_from_adp(manifest, backend_factory=None)
  → (DynamicFlow: type, agent_map: dict[str, Agent])
```

`DynamicFlow` is a dynamically-created `Flow` subclass. Each ADP node becomes a
method on the class:

- **start nodes** (`start_nodes[]`) → decorated with `@start`
- **router nodes** (`kind: router`) → decorated with `@router`
- **all other nodes** → decorated with `@listen`

`agent_map` maps ADP node IDs to `crewai.Agent` instances. Pass a
`backend_factory(node, runtime_entry) -> Agent` to inject custom agents.

## Lazy Import

`build_adp_graph.py` is always importable even without `crewai` installed.
When `crewai` is absent the module operates in mock mode (returning `MockAgent`
and `MockFlow` stubs). Tests use `pytest.importorskip("crewai")` so they skip
rather than fail in environments without `crewai`.

## Links

- [Framework Interop Guide](../../../spec/framework-interop.md) — full CrewAI mapping
- [ADP Spec](../../../spec/adp-v0.1.0.md) — full protocol specification
