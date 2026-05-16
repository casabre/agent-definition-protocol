# Framework Interoperability Guide

Informative (not normative). Guidance for runner implementers.

This guide shows how the ADP flow model maps to popular agent frameworks. Use it as a starting point when building a runner. Refer to [runtime-flow-binding.md](runtime-flow-binding.md) for normative backend selection and condition expression rules.

---

## ADP Flow Model Summary

| Concept | Description |
|---|---|
| `flow.graph.nodes[]` | Typed callables; each has an `id` (unique) and a `kind` (one of 7) |
| `flow.graph.edges[]` | Directed; optional `condition` string triggers conditional routing |
| `start_nodes[]` / `end_nodes[]` | Entry and exit node IDs |
| State | 4-key dict: `inputs` (immutable), `context`, `memory`, `tool_responses` |

State keys by ADR rule:
- `inputs` — populated by `input` node (D1); immutable thereafter
- `context[node.id]` — written by `llm`, `retriever`, `evaluator` nodes (D2, D5, D6)
- `tool_responses[node.id]` — appended (never replaced) by `tool` nodes (D3)

---

## LangGraph Mapping

| ADP concept | LangGraph equivalent |
|---|---|
| `flow.graph.nodes[]` | `StateGraph.add_node(id, callable)` |
| unconditional edge | `StateGraph.add_edge(from, to)` |
| conditional edge | `StateGraph.add_conditional_edges(from, routing_fn, {target: target})` |
| `start_nodes[]` | `StateGraph.set_entry_point(start_nodes[0])` or `START` node |
| `end_nodes[]` | `END` constant in routing functions |
| `state.context` | TypedDict field `context: dict[str, Any]` |
| `state.tool_responses` | TypedDict field `tool_responses: dict[str, list]` |
| `node.model_ref` | Resolve from `runtime.models[]` → `ChatOpenAI(model=...)` or equivalent |
| `node.runtime_ref` | Explicit backend override; see `spec/runtime-flow-binding.md §2` |

### State TypedDict

```python
from typing import Any, TypedDict

class ADPState(TypedDict):
    inputs: dict
    context: dict[str, Any]
    memory: dict
    tool_responses: dict[str, list]
```

### Condition Expression → Routing Function

ADP condition `"context.decide.decision == approved"` maps to a LangGraph routing callable:

```python
import json
import operator as op_module

OPS = {
    "==": op_module.eq,
    "!=": op_module.ne,
    ">":  op_module.gt,
    ">=": op_module.ge,
    "<":  op_module.lt,
    "<=": op_module.le,
}

def make_condition_fn(condition_str: str):
    """Parse ADP condition string into a callable that evaluates against state."""
    key, op_str, value = condition_str.split(" ", 2)
    op_fn = OPS.get(op_str)
    if op_fn is None:
        raise ValueError(f"Unsupported operator '{op_str}' in condition: {condition_str}")
    path = key.split(".")  # e.g. ["context", "decide", "decision"]
    try:
        parsed_value = json.loads(value)
    except json.JSONDecodeError:
        parsed_value = value  # treat as raw string token
    def condition_fn(state: dict) -> bool:
        current = state
        for segment in path:
            current = current[segment]
        return op_fn(current, parsed_value)
    return condition_fn
```

Path format: first segment is the state root key (`inputs`, `context`, `memory`, `tool_responses`); for `context` and `tool_responses`, the second segment is the `node.id` of the node that wrote the value. See `spec/runtime-flow-binding.md §Condition Expression Format`.

### Backend Resolution

```python
from spec_runtime_flow_binding import COMPAT_MATRIX  # see runtime-flow-binding.md

def resolve_backend(node: dict, execution: list[dict]) -> dict:
    """Select the first compatible runtime.execution entry for this node kind."""
    runtime_ref = node.get("runtime_ref")
    if runtime_ref:
        entry = next((e for e in execution if e["id"] == runtime_ref), None)
        if not entry:
            raise ValueError(f"runtime_ref '{runtime_ref}' not found in runtime.execution")
        return entry
    kind = node.get("kind", "")
    for entry in execution:
        if _is_compatible(kind, entry.get("backend", "")):
            return entry
    raise ValueError(f"No compatible backend found for node kind '{kind}'")

def _is_compatible(kind: str, backend: str) -> bool:
    compatible = COMPAT_MATRIX.get(kind, [])
    return backend in compatible or any(backend.startswith(p) for p in compatible if p.endswith(":"))
```

### Hello World: ADP Manifest → LangGraph Graph

Full construction algorithm handling multi-target conditional routing:

```python
from collections import defaultdict
from typing import Callable
from langgraph.graph import StateGraph, END

BackendFactory = Callable[[dict, dict], Callable[[ADPState], ADPState]]

def build_langgraph_from_adp(
    manifest: dict,
    backend_factory: BackendFactory | None = None,
) -> tuple:
    """Build a compiled LangGraph StateGraph from an ADP manifest.

    Returns (compiled_graph, adp_node_map) where adp_node_map preserves
    ADP node metadata keyed by node.id for round-trip support.
    """
    adp_node_map: dict[str, dict] = {}
    graph = StateGraph(ADPState)
    flow = manifest["flow"]["graph"]
    runtime = manifest["runtime"]

    for node in flow["nodes"]:
        adp_node_map[node["id"]] = node
        entry = resolve_backend(node, runtime["execution"])
        fn = backend_factory(node, entry) if backend_factory else _default_callable(node, entry)
        graph.add_node(node["id"], fn)

    cond_edges: dict[str, list[dict]] = defaultdict(list)
    for edge in flow.get("edges", []):
        if "condition" in edge:
            cond_edges[edge["from"]].append(edge)
        else:
            graph.add_edge(edge["from"], edge["to"])

    for source, edges in cond_edges.items():
        target_map = {e["to"]: make_condition_fn(e["condition"]) for e in edges}
        def _router(state: ADPState, tm=target_map) -> str:
            for target_id, cond_fn in tm.items():
                if cond_fn(state):
                    return target_id
            return END
        graph.add_conditional_edges(
            source, _router, {k: k for k in target_map} | {END: END}
        )

    for start in flow["start_nodes"]:
        graph.set_entry_point(start)

    return graph.compile(), adp_node_map


def adp_from_langgraph(
    graph,
    adp_node_map: dict[str, dict],
    original_manifest: dict,
) -> dict:
    """Reconstruct a partial ADP manifest from a compiled LangGraph graph.

    Structural fidelity only: node IDs, edge connections, start/end nodes.
    Non-structural fields (model_ref, suite_ref, runtime_ref) are recovered
    from adp_node_map, which is returned alongside the graph by build_langgraph_from_adp.
    """
    draw = graph.get_graph()
    nodes = [
        adp_node_map.get(nid, {"id": nid, "kind": "unknown"})
        for nid in draw.nodes
        if nid not in ("__start__", "__end__")
    ]
    edges = [
        {"from": e.source, "to": e.target}
        for e in draw.edges
        if e.source != "__start__" and e.target != "__end__"
    ]
    start_nodes = [e.target for e in draw.edges if e.source == "__start__"]
    end_nodes = [e.source for e in draw.edges if e.target == "__end__"]
    return {
        **original_manifest,
        "flow": {
            **original_manifest.get("flow", {}),
            "graph": {
                "nodes": nodes,
                "edges": edges,
                "start_nodes": start_nodes,
                "end_nodes": end_nodes,
            },
        },
    }
```

**Round-trip limitation**: non-structural fields (`model_ref`, `suite_ref`, `runtime_ref`) are preserved via `adp_node_map`, not via LangGraph introspection — LangGraph does not store ADP metadata on `add_node`. The round-trip is structural (node IDs, edge connections), not a full manifest equality.

---

## AutoGen Mapping

ADP flow nodes map to AutoGen `ConversableAgent` instances; edges model `initiate_chat` sequences. The `router` node kind maps to `GroupChatManager` with speaker selection based on state. ADP state (`context`, `tool_responses`) maps to `agent.chat_messages`. Detailed mapping is deferred to v0.2.0; this section provides directional guidance.

Key differences from LangGraph:
- AutoGen is message-passing, not graph-traversal; explicit `initiate_chat` sequences replace edges
- Shared state requires explicit agent-to-agent message construction
- Conditional routing is implemented via `GroupChatManager.speaker_selection_method`

---

## Semantic Kernel Mapping

| ADP concept | Semantic Kernel equivalent |
|---|---|
| `llm` node | `KernelFunction` registered with a Kernel |
| `tool` node | `KernelPlugin` (native or MCP-based) |
| `flow.graph` | `KernelProcess` (SK Process Framework) |
| `runtime.models[].model` | `OpenAIChatCompletion` registered with the Kernel |
| `state.context[node.id]` | Process step output passed to next step |

SK Process Framework (`KernelProcess`) is the closest mapping to ADP's flow graph. Each ADP node becomes a `KernelProcessStep`; edges become step connections via `OnEvent`.

---

## CrewAI Mapping

| ADP concept | CrewAI equivalent |
|---|---|
| `flow.graph` | `Flow` (CrewAI Flows API) |
| `input` node | `@start` decorated method |
| `llm` / `tool` node | `@listen` decorated method |
| `router` node | `@router` decorated method |
| `end_nodes` | Final `@listen` with no outgoing connections |
| `state.context[node.id]` | Flow state attribute keyed by step name |
| `evaluation` | Task `expected_output` checks |

CrewAI Flows supports conditional branching via `@router`, matching ADP's conditional edge semantics.

---

## Framework-Agnostic Construction Pattern

Regardless of target framework, ADP runner construction follows this order:

1. **Validate** — Call `validate_adp()` + `validate_adp_semantics()` before doing anything else
2. **Build node table** — `{node.id: node}` from `flow.graph.nodes[]`
3. **Resolve backends** — For each node, select `runtime.execution[]` entry per compatibility matrix (see `spec/runtime-flow-binding.md §1`)
4. **Create callables** — Wrap each backend entry in a framework-specific callable that reads/writes ADP state keys
5. **Wire edges** — Unconditional edges first, then group conditional edges by source node; for each group create a routing function
6. **Set entry/exit** — `start_nodes[]` and `end_nodes[]`
7. **Error handling** — If any `runtime_ref` is missing at construction time, fail before execution starts

**State initialization** (before first node fires):
```python
initial_state = {
    "inputs": invocation_payload,
    "context": {},
    "memory": {},
    "tool_responses": {},
}
```

`inputs` is set once from the invocation payload and is not mutated by any node.
