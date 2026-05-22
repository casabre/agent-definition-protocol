# ADP → AutoGen Runner

Demonstrates building AutoGen `ConversableAgent` instances from an ADP manifest.

## Direction

**Import-only (ADP → AutoGen).** Export (AutoGen → ADP) is deferred to v0.3.0.

AutoGen does not expose a standardized introspection API for recovering flow structure from a set of agents. Until that gap is addressed in the spec, only the ADP-to-AutoGen direction is supported here.

## Target API Version

Targets **pyautogen >= 0.2, < 0.4** (the `ConversableAgent` legacy API).

pyautogen 0.4+ rebranded to `autogen-agentchat` and introduced a new API (`AssistantAgent`, `RoundRobinGroupChat`, etc.). That mapping is planned for v0.3.0.

## Mapping Summary

| ADP concept | AutoGen equivalent |
|---|---|
| `flow.graph.nodes[]` | `ConversableAgent` (one per node, `llm_config=False` in tests) |
| `router` node | `GroupChat` + `GroupChatManager` (keyed as `{node_id}_manager`) |
| `flow.graph.edges[]` | `chat_sequence` list of `{"from": …, "to": …}` dicts |
| `start_nodes[]` | First `initiator` in `chat_sequence` |

AutoGen is message-passing, not graph-traversal. Explicit `initiate_chat` sequences replace LangGraph-style edge wiring. Conditional routing is implemented via `GroupChatManager.speaker_selection_method`.

## Quick Start

```bash
pip install -r requirements.txt
pip install -e ../../../sdk/python
pytest -v
```

If pyautogen is not installed, all tests are skipped gracefully via `pytest.importorskip`.

## What Each Test Verifies

| Test | What it checks |
|---|---|
| `test_agents_created_for_all_nodes` | `agent_map` keys match every node ID declared in the manifest |
| `test_composition_resolves_before_build` | Composition-resolved billing manifest (`billing-variant.yaml`) drives agent creation |
| `test_router_uses_group_chat_manager` | A `router` node produces a `GroupChatManager` keyed as `{node_id}_manager` |
| `test_tool_node_creates_agent` | A `tool` node gets its own `ConversableAgent` entry |

## Architecture

```
build_autogen_from_adp(manifest, backend_factory=None)
  → (agent_map: dict[str, ConversableAgent], chat_sequence: list[dict])
```

`agent_map` maps ADP node IDs to `ConversableAgent` instances. Router nodes additionally produce a `GroupChatManager` under the key `{node_id}_manager`.

`chat_sequence` is a list of `{"from": node_id, "to": node_id}` dicts derived from `flow.graph.edges[]`, preserving edge order.

## Links

- [Framework Interop Guide](../../../spec/framework-interop.md) — full AutoGen/LangGraph/SK/CrewAI mapping
- [ADP Spec](../../../spec/adp-v0.2.0.md) — full protocol specification
