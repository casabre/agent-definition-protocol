"""
Tests for crewai integration module that run in mock mode (no crewai installation required).
"""
from adp_sdk.integrations.crewai import (
    _probe_crewai,
    _build_mock,
    _MockAgent,
    _MockFlow,
    build_crewai_from_adp,
)


SIMPLE_MANIFEST = {
    "adp_version": "0.2.0",
    "id": "test.crewai",
    "runtime": {
        "execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}],
    },
    "flow": {
        "id": "test.flow",
        "graph": {
            "nodes": [
                {"id": "input", "kind": "input"},
                {"id": "chat", "kind": "llm"},
                {"id": "output", "kind": "output"},
            ],
            "edges": [
                {"from": "input", "to": "chat"},
                {"from": "chat", "to": "output"},
            ],
            "start_nodes": ["input"],
            "end_nodes": ["output"],
        },
    },
    "evaluation": {"suites": [{"id": "basic", "metrics": [{"id": "m", "type": "deterministic", "threshold": True}]}]},
}

ROUTER_MANIFEST = {
    "adp_version": "0.2.0",
    "id": "test.router",
    "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
    "flow": {
        "id": "router.flow",
        "graph": {
            "nodes": [
                {"id": "input", "kind": "input"},
                {"id": "decide", "kind": "router"},
                {"id": "path_a", "kind": "llm"},
            ],
            "edges": [
                {"from": "input", "to": "decide"},
                {"from": "decide", "to": "path_a"},
            ],
            "start_nodes": ["input"],
            "end_nodes": ["path_a"],
        },
    },
    "evaluation": {"suites": [{"id": "basic", "metrics": [{"id": "m", "type": "deterministic", "threshold": True}]}]},
}


def test_probe_crewai_returns_bool():
    """_probe_crewai returns a bool (False when crewai not installed)."""
    result = _probe_crewai()
    assert isinstance(result, bool)


def test_mock_agent_repr():
    """_MockAgent repr includes role."""
    agent = _MockAgent(role="my-role", goal="do things", backstory="history")
    assert "my-role" in repr(agent)


def test_mock_agent_attributes():
    """_MockAgent stores all constructor attributes."""
    agent = _MockAgent(role="r", goal="g", backstory="b", allow_delegation=True)
    assert agent.role == "r"
    assert agent.goal == "g"
    assert agent.backstory == "b"
    assert agent.allow_delegation is True


def test_mock_flow_repr():
    """_MockFlow repr includes start and router node lists."""
    flow_cls = type("MyFlow", (_MockFlow,), {"_start_node_ids": ["a"], "_router_node_ids": []})
    f = flow_cls()
    r = repr(f)
    assert "start" in r or "MockFlow" in r


def test_build_mock_creates_agents_for_all_nodes():
    """_build_mock creates a MockAgent for every node."""
    nodes = [{"id": "n1"}, {"id": "n2"}]
    DynamicFlow, agent_map = _build_mock(nodes, ["n1"], [])
    assert set(agent_map.keys()) == {"n1", "n2"}
    assert all(isinstance(a, _MockAgent) for a in agent_map.values())


def test_build_mock_sets_start_and_router_ids():
    """_build_mock sets _start_node_ids and _router_node_ids on the flow class."""
    nodes = [{"id": "a"}, {"id": "b"}]
    DynamicFlow, _ = _build_mock(nodes, start_node_ids=["a"], router_node_ids=["b"])
    assert DynamicFlow._start_node_ids == ["a"]
    assert DynamicFlow._router_node_ids == ["b"]


def test_build_crewai_from_adp_returns_something():
    """build_crewai_from_adp returns (flow_cls, agent_map) with expected structure."""
    flow_cls, agent_map = build_crewai_from_adp(SIMPLE_MANIFEST)
    node_ids = {n["id"] for n in SIMPLE_MANIFEST["flow"]["graph"]["nodes"]}
    assert set(agent_map.keys()) == node_ids
    # start_node_ids should be set regardless of mock/real mode
    assert flow_cls._start_node_ids == ["input"]


def test_build_crewai_from_adp_router_in_mock():
    """build_crewai_from_adp identifies router nodes in mock mode."""
    flow_cls, agent_map = build_crewai_from_adp(ROUTER_MANIFEST)
    assert "decide" in flow_cls._router_node_ids


def test_probe_crewai_caches_result():
    """_probe_crewai returns the same result on repeated calls (cached value)."""
    import adp_sdk.integrations.crewai as _mod
    # Reset the cached state to None to force a fresh probe
    original = _mod._crewai_available
    _mod._crewai_available = None
    try:
        r1 = _probe_crewai()
        r2 = _probe_crewai()
        assert r1 == r2
        assert isinstance(r1, bool)
    finally:
        _mod._crewai_available = original


def test_build_crewai_in_mock_mode_forced(monkeypatch):
    """build_crewai_from_adp uses _build_mock when crewai probe returns False."""
    import adp_sdk.integrations.crewai as _mod
    monkeypatch.setattr(_mod, "_crewai_available", None)
    monkeypatch.setattr(_mod, "_probe_crewai", lambda: False)
    # Ensure _probe_crewai=False => _build_mock is called
    flow_cls, agent_map = _mod.build_crewai_from_adp(SIMPLE_MANIFEST)
    node_ids = {n["id"] for n in SIMPLE_MANIFEST["flow"]["graph"]["nodes"]}
    assert set(agent_map.keys()) == node_ids
    assert all(isinstance(a, _MockAgent) for a in agent_map.values())


def test_build_crewai_from_adp_no_edges():
    """build_crewai_from_adp handles manifest with no edges."""
    manifest = {
        "adp_version": "0.2.0",
        "id": "test.no-edges",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {
            "id": "f",
            "graph": {
                "nodes": [{"id": "solo", "kind": "input"}],
                "start_nodes": ["solo"],
                "end_nodes": ["solo"],
            },
        },
        "evaluation": {},
    }
    flow_cls, agent_map = build_crewai_from_adp(manifest)
    assert "solo" in agent_map


def test_probe_crewai_import_failure_path():
    """Blocking crewai in sys.modules exercises the except ImportError branch in _probe_crewai.

    This covers lines 26-27 of crewai.py (the except block) that are unreachable
    in a normal run when crewai IS installed.
    """
    import sys
    import adp_sdk.integrations.crewai as _mod

    # Save current probe state
    original_available = _mod._crewai_available

    try:
        # Reset probe state to force a fresh probe
        _mod._crewai_available = None

        # Block crewai import
        blocked_keys = [k for k in sys.modules if k.startswith("crewai")]
        saved_blocked = {k: sys.modules.pop(k) for k in blocked_keys}
        sys.modules["crewai"] = None  # type: ignore[assignment]

        try:
            result = _mod._probe_crewai()
            assert result is False
            assert _mod._crewai_available is False
        finally:
            del sys.modules["crewai"]
            for k, v in saved_blocked.items():
                sys.modules[k] = v
    finally:
        # Restore original state
        _mod._crewai_available = original_available
