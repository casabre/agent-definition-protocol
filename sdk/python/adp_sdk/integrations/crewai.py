"""
ADP → CrewAI conversion utilities. Import-only: ADP → CrewAI.
Export (CrewAI → ADP) is deferred to v0.3.0.

Targets CrewAI Flows API (crewai >= 0.63 / crewai >= 1.0).
NOT the legacy Crew/Task API.

See spec/framework-interop.md §CrewAI Mapping for the full mapping guide.
"""
from __future__ import annotations

from typing import Any, Callable

BackendFactory = Callable[[dict, dict], Any] | None

_crewai_available: bool | None = None  # None = not yet probed


def _probe_crewai() -> bool:
    global _crewai_available
    if _crewai_available is None:
        try:
            import crewai  # noqa: F401  # pragma: no cover
            from crewai.flow.flow import Flow  # noqa: F401  # pragma: no cover
            _crewai_available = True  # pragma: no cover
        except ImportError:
            _crewai_available = False
    return _crewai_available


class _MockAgent:
    """Minimal stand-in for crewai.Agent used when crewai is absent."""

    def __init__(self, *, role: str, goal: str, backstory: str, allow_delegation: bool = False):
        self.role = role
        self.goal = goal
        self.backstory = backstory
        self.allow_delegation = allow_delegation

    def __repr__(self) -> str:
        return f"MockAgent(role={self.role!r})"


class _MockFlow:
    """Minimal stand-in for crewai.flow.flow.Flow used when crewai is absent."""

    _start_node_ids: list[str] = []
    _router_node_ids: list[str] = []

    def __repr__(self) -> str:
        return f"MockFlow(start={self._start_node_ids}, routers={self._router_node_ids})"


def build_crewai_from_adp(
    manifest: dict,
    backend_factory: BackendFactory = None,
) -> tuple:
    """Build a CrewAI Flow from an ADP manifest.

    Returns:
        (flow_class: type, agent_map: dict[str, Any])

    ``flow_class`` is a dynamically-created Flow subclass (or a MockFlow
    subclass when crewai is not installed).

    ``agent_map`` maps ADP node IDs to CrewAI Agent instances (or MockAgent
    instances when crewai is not installed).

    The function is always importable. When crewai is absent it operates in
    mock mode so that structural tests can still run. Tests that require a
    real crewai installation should use ``pytest.importorskip("crewai")``.
    """
    flow = manifest["flow"]["graph"]
    nodes: list[dict] = flow["nodes"]
    edges: list[dict] = flow.get("edges", [])
    start_node_ids: list[str] = flow.get("start_nodes", [])
    router_node_ids: list[str] = [n["id"] for n in nodes if n.get("kind") == "router"]

    if _probe_crewai():
        return _build_real(nodes, start_node_ids, router_node_ids, edges, backend_factory)  # pragma: no cover
    else:
        return _build_mock(nodes, start_node_ids, router_node_ids)


def _build_real(  # pragma: no cover
    nodes: list[dict],
    start_node_ids: list[str],
    router_node_ids: list[str],
    edges: list[dict],
    backend_factory: BackendFactory,
) -> tuple:
    from crewai import Agent
    from crewai.flow.flow import Flow, listen, or_, router, start

    # Build predecessor map from flow edges: node_id -> [source_ids]
    predecessors: dict[str, list[str]] = {}
    for edge in edges:
        predecessors.setdefault(edge["to"], []).append(edge["from"])

    agent_map: dict[str, Any] = {}
    for node in nodes:
        node_id = node["id"]
        if backend_factory is not None:
            agent = backend_factory(node, {})
        else:
            agent = Agent(
                role=node_id,
                goal=f"Execute {node_id}",
                backstory="ADP agent node",
                allow_delegation=False,
            )
        agent_map[node_id] = agent

    method_bodies: dict[str, Any] = {}

    for node in nodes:
        node_id = node["id"]
        preds = predecessors.get(node_id, [])
        is_start = node_id in start_node_ids
        is_router = node_id in router_node_ids

        if is_start:
            @start()
            def _method(self, _nid=node_id):  # type: ignore[misc]
                return _nid
        elif is_router and preds:
            # Router: triggered by predecessor(s), returns routing decision string
            trigger = preds[0] if len(preds) == 1 else or_(*preds)
            @router(trigger)
            def _method(self, _nid=node_id):  # type: ignore[misc]
                return _nid
        elif preds:
            # Regular node: listen to predecessor(s)
            trigger = preds[0] if len(preds) == 1 else or_(*preds)
            @listen(trigger)
            def _method(self, _nid=node_id):  # type: ignore[misc]
                return _nid
        else:
            # No predecessors and not a declared start — treat as start
            @start()
            def _method(self, _nid=node_id):  # type: ignore[misc]
                return _nid
        method_bodies[node_id] = _method

    DynamicFlow: type = type(
        "DynamicFlow",
        (Flow,),
        {name: method for name, method in method_bodies.items()},
    )
    DynamicFlow._start_node_ids = start_node_ids  # type: ignore[attr-defined]
    DynamicFlow._router_node_ids = router_node_ids  # type: ignore[attr-defined]

    return DynamicFlow, agent_map


def _build_mock(
    nodes: list[dict],
    start_node_ids: list[str],
    router_node_ids: list[str],
) -> tuple:
    agent_map: dict[str, Any] = {
        node["id"]: _MockAgent(
            role=node["id"],
            goal=f"Execute {node['id']}",
            backstory="ADP agent node",
            allow_delegation=False,
        )
        for node in nodes
    }

    DynamicFlow: type = type(
        "DynamicFlow",
        (_MockFlow,),
        {
            "_start_node_ids": start_node_ids,
            "_router_node_ids": router_node_ids,
        },
    )

    return DynamicFlow, agent_map
