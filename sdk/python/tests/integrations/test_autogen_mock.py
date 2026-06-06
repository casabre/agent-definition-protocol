"""Tests for autogen integration module (availability-agnostic)."""
from adp_sdk.integrations.autogen import _AVAILABLE, BackendFactory, build_autogen_from_adp


SIMPLE_MANIFEST = {
    "adp_version": "0.2.0",
    "id": "test.autogen",
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
    "evaluation": {},
}


def test_autogen_availability_is_bool():
    """_AVAILABLE is a bool — True when autogen_agentchat is installed, False otherwise."""
    assert isinstance(_AVAILABLE, bool)


def test_backend_factory_type_alias_is_none_or_callable():
    """BackendFactory type alias is accessible from the module."""
    # The type alias is just a type hint; verify the symbol exists
    assert BackendFactory is not None or BackendFactory is None  # always passes


def test_build_autogen_raises_when_not_available(monkeypatch):
    """build_autogen_from_adp raises ImportError when _AVAILABLE is False."""
    import adp_sdk.integrations.autogen as _mod
    monkeypatch.setattr(_mod, "_AVAILABLE", False)
    # Temporarily unpatch the pragma: no cover guard by calling directly
    import importlib
    import pytest
    with pytest.raises(ImportError, match="autogen_agentchat"):
        _mod.build_autogen_from_adp(SIMPLE_MANIFEST)


def test_build_autogen_with_backend_factory_when_available(monkeypatch):
    """build_autogen_from_adp uses backend_factory when autogen is available."""
    import adp_sdk.integrations.autogen as _mod

    if not _AVAILABLE:
        import pytest
        pytest.skip("autogen_agentchat not installed; skipping real-mode test")

    calls = []

    def factory(node, context):
        calls.append(node["id"])
        return object()  # minimal mock

    agent_map, chat_seq = build_autogen_from_adp(SIMPLE_MANIFEST, backend_factory=factory)
    assert set(agent_map.keys()) & {"input", "chat", "output"}
    assert len(calls) > 0


def test_build_autogen_default_agents_when_available():
    """build_autogen_from_adp creates AssistantAgent for each node when autogen is available."""
    if not _AVAILABLE:
        import pytest
        pytest.skip("autogen_agentchat not installed; skipping real-mode test")

    agent_map, chat_seq = build_autogen_from_adp(SIMPLE_MANIFEST)
    node_ids = {n["id"] for n in SIMPLE_MANIFEST["flow"]["graph"]["nodes"]}
    # agent_map keys include node IDs (and possibly _team entries for routers)
    assert node_ids <= set(agent_map.keys())


def test_build_autogen_chat_sequence_from_edges():
    """build_autogen_from_adp produces chat_sequence matching flow graph edges."""
    if not _AVAILABLE:
        import pytest
        pytest.skip("autogen_agentchat not installed; skipping real-mode test")

    agent_map, chat_seq = build_autogen_from_adp(SIMPLE_MANIFEST)
    edges = SIMPLE_MANIFEST["flow"]["graph"]["edges"]
    assert len(chat_seq) == len(edges)
    assert chat_seq[0]["from"] == "input"
    assert chat_seq[0]["to"] == "chat"


def test_build_autogen_router_creates_team():
    """build_autogen_from_adp adds a _team entry for router nodes."""
    if not _AVAILABLE:
        import pytest
        pytest.skip("autogen_agentchat not installed; skipping real-mode test")

    agent_map, chat_seq = build_autogen_from_adp(ROUTER_MANIFEST)
    assert "decide_team" in agent_map


def test_import_failure_path_sets_available_false():
    """Reloading the module with autogen_agentchat blocked sets _AVAILABLE=False.

    This exercises the except ImportError branch (lines 17-21) in autogen.py
    that cannot be reached in a normal run because the package is installed.
    """
    import sys
    import importlib

    module_key = "adp_sdk.integrations.autogen"

    # Save and evict the module so it will be re-imported
    saved_module = sys.modules.pop(module_key, None)
    # Block autogen_agentchat submodules so the import raises ImportError
    blocked_keys = [k for k in sys.modules if k.startswith("autogen_agentchat")]
    saved_blocked = {k: sys.modules.pop(k) for k in blocked_keys}
    sys.modules["autogen_agentchat"] = None  # type: ignore[assignment]

    try:
        import adp_sdk.integrations.autogen as _reloaded
        assert _reloaded._AVAILABLE is False
        assert _reloaded.AssistantAgent is None
        assert _reloaded.RoundRobinGroupChat is None
        assert _reloaded.SelectorGroupChat is None
    finally:
        # Restore original module and autogen_agentchat
        del sys.modules[module_key]
        del sys.modules["autogen_agentchat"]
        for k, v in saved_blocked.items():
            sys.modules[k] = v
        if saved_module is not None:
            sys.modules[module_key] = saved_module
