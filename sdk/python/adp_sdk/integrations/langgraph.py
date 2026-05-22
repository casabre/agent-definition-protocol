"""
ADP → LangGraph and LangGraph → ADP conversion utilities.

See spec/framework-interop.md §LangGraph Mapping for the full mapping guide.
See spec/runtime-flow-binding.md for backend selection and condition expression rules.
"""
from __future__ import annotations

import json
import operator as op_module
from collections import defaultdict
from typing import Any, Callable, TypedDict

try:
    from langgraph.graph import END, StateGraph
    _AVAILABLE = True
except ImportError:
    END = None  # type: ignore[assignment]
    StateGraph = None  # type: ignore[assignment]
    _AVAILABLE = False

COMPAT_MATRIX: dict[str, list[str]] = {
    "llm":       ["openai", "anthropic", "vllm", "bedrock", "langchain", "litellm", "llm:"],
    "tool":      ["python", "docker", "http", "mcp", "wasm", "tool:"],
    "retriever": ["pinecone", "weaviate", "chroma", "redis", "pgvector", "memory:"],
    "evaluator": [],
    "router":    [],
    "input":     [],
    "output":    [],
}

_OPS: dict[str, Any] = {
    "==": op_module.eq,
    "!=": op_module.ne,
    ">":  op_module.gt,
    ">=": op_module.ge,
    "<":  op_module.lt,
    "<=": op_module.le,
}


class ADPState(TypedDict):
    inputs: dict
    context: dict[str, Any]
    memory: dict
    tool_responses: dict[str, list]


BackendFactory = Callable[[dict, dict], Callable[[ADPState], ADPState]]


def make_condition_fn(condition_str: str) -> Callable[[dict], bool]:
    """Parse an ADP condition string into a callable that evaluates against state."""
    key, op_str, value = condition_str.split(" ", 2)
    op_fn = _OPS.get(op_str)
    if op_fn is None:
        raise ValueError(f"Unsupported operator '{op_str}' in condition: {condition_str}")
    path = key.split(".")
    try:
        parsed_value = json.loads(value)
    except json.JSONDecodeError:
        parsed_value = value
    def condition_fn(state: dict) -> bool:
        current: Any = state
        for segment in path:
            current = current[segment]
        return op_fn(current, parsed_value)
    return condition_fn


def resolve_backend(node: dict, execution: list[dict]) -> dict:
    """Select the first compatible runtime.execution entry for this node kind."""
    runtime_ref = node.get("runtime_ref")
    if runtime_ref:
        entry = next((e for e in execution if e["id"] == runtime_ref), None)
        if entry is None:
            raise ValueError(f"runtime_ref '{runtime_ref}' not found in runtime.execution")
        return entry
    kind = node.get("kind", "")
    compatible = COMPAT_MATRIX.get(kind, [])
    for entry in execution:
        backend = entry.get("backend", "")
        if backend in compatible or any(backend.startswith(p) for p in compatible if p.endswith(":")):
            return entry
    return execution[0] if execution else {}


def resolve_callable(
    node: dict,
    manifest: dict,
    backend_factory: BackendFactory | None,
    entry: dict,
) -> Callable:
    """Resolve a callable for a node, handling tool_ref lookup."""
    tool_ref = node.get("tool_ref")
    if tool_ref:
        tools = manifest.get("tools", {})
        tool_entry = None
        for tool_list_key in ("mcp_servers", "http_apis", "sql_functions"):
            for tool in tools.get(tool_list_key, []):
                if tool.get("id") == tool_ref:
                    tool_entry = tool
                    break
            if tool_entry:
                break
        if tool_entry is None:
            raise ValueError(f"tool_ref '{tool_ref}' not found in tools")
        if backend_factory:
            return backend_factory(node, {"tool_entry": tool_entry, **entry})
        return _default_callable(node, entry)
    if backend_factory:
        return backend_factory(node, entry)
    return _default_callable(node, entry)


def _default_callable(node: dict, _entry: dict) -> Callable[[ADPState], ADPState]:
    """Minimal no-op callable used when no backend_factory is provided."""
    node_id = node["id"]
    kind = node.get("kind", "")
    def _fn(state: ADPState) -> ADPState:
        new_state = dict(state)
        if kind in ("llm", "retriever", "evaluator"):
            new_state["context"] = {**state.get("context", {}), node_id: {}}
        elif kind == "tool":
            responses = list(state.get("tool_responses", {}).get(node_id, []))
            new_state["tool_responses"] = {**state.get("tool_responses", {}), node_id: responses}
        return new_state  # type: ignore[return-value]
    return _fn


def build_langgraph_from_adp(
    manifest: dict,
    backend_factory: BackendFactory | None = None,
) -> tuple:
    """Build a compiled LangGraph StateGraph from an ADP manifest.

    Returns (compiled_graph, adp_node_map) where adp_node_map is a
    dict[str, dict] preserving ADP node metadata for round-trip support.
    """
    if not _AVAILABLE:
        raise ImportError("langgraph required: pip install 'adp-sdk[langgraph]'")
    adp_node_map: dict[str, dict] = {}
    graph: Any = StateGraph(ADPState)
    flow = manifest["flow"]["graph"]
    runtime = manifest["runtime"]
    execution = runtime.get("execution", [])

    for node in flow["nodes"]:
        adp_node_map[node["id"]] = node
        entry = resolve_backend(node, execution)
        fn = resolve_callable(node, manifest, backend_factory, entry)
        graph.add_node(node["id"], fn)

    cond_edges: dict[str, list[dict]] = defaultdict(list)
    for edge in flow.get("edges", []):
        if "condition" in edge:
            cond_edges[edge["from"]].append(edge)
        else:
            graph.add_edge(edge["from"], edge["to"])

    for source, edges in cond_edges.items():
        target_map = {e["to"]: make_condition_fn(e["condition"]) for e in edges}
        def _router(state: ADPState, tm: dict = target_map) -> str:
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
    graph: Any,
    adp_node_map: dict[str, dict],
    original_manifest: dict,
) -> dict:
    """Reconstruct a partial ADP manifest from a compiled LangGraph graph.

    Structural fidelity only: node IDs, edge connections, start/end nodes.
    Non-structural ADP fields (model_ref, suite_ref, runtime_ref) are recovered
    from adp_node_map — they are not stored natively in LangGraph.
    """
    if not _AVAILABLE:
        raise ImportError("langgraph required: pip install 'adp-sdk[langgraph]'")
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
