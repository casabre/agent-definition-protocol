"""
Tests for pure utility functions in adp_sdk.integrations.langgraph
that do NOT require the langgraph package to be installed.
"""
import pytest
from adp_sdk.integrations.langgraph import (
    make_condition_fn,
    resolve_backend,
    resolve_callable,
    _default_callable,
)


# ---------------------------------------------------------------------------
# make_condition_fn
# ---------------------------------------------------------------------------

def test_make_condition_fn_eq_true():
    fn = make_condition_fn("inputs.x == 1")
    assert fn({"inputs": {"x": 1}}) is True


def test_make_condition_fn_eq_false():
    fn = make_condition_fn("inputs.x == 1")
    assert fn({"inputs": {"x": 2}}) is False


def test_make_condition_fn_ne():
    fn = make_condition_fn("inputs.flag != true")
    assert fn({"inputs": {"flag": False}}) is True
    assert fn({"inputs": {"flag": True}}) is False


def test_make_condition_fn_gt():
    fn = make_condition_fn("inputs.score > 0.5")
    assert fn({"inputs": {"score": 0.9}}) is True
    assert fn({"inputs": {"score": 0.1}}) is False


def test_make_condition_fn_ge():
    fn = make_condition_fn("inputs.score >= 0.5")
    assert fn({"inputs": {"score": 0.5}}) is True


def test_make_condition_fn_lt():
    fn = make_condition_fn("inputs.score < 0.5")
    assert fn({"inputs": {"score": 0.3}}) is True


def test_make_condition_fn_le():
    fn = make_condition_fn("inputs.score <= 0.5")
    assert fn({"inputs": {"score": 0.5}}) is True


def test_make_condition_fn_string_value():
    """Condition value that is not valid JSON is kept as string."""
    fn = make_condition_fn("inputs.status == active")
    assert fn({"inputs": {"status": "active"}}) is True


def test_make_condition_fn_unsupported_operator():
    with pytest.raises(ValueError, match="Unsupported operator"):
        make_condition_fn("inputs.x ~= 1")


# ---------------------------------------------------------------------------
# resolve_backend
# ---------------------------------------------------------------------------

EXECUTION = [
    {"id": "py", "backend": "python"},
    {"id": "oai", "backend": "openai"},
    {"id": "pg", "backend": "pgvector"},
]


def test_resolve_backend_by_runtime_ref():
    node = {"id": "n", "kind": "llm", "runtime_ref": "oai"}
    entry = resolve_backend(node, EXECUTION)
    assert entry["id"] == "oai"


def test_resolve_backend_runtime_ref_not_found_raises():
    node = {"id": "n", "kind": "llm", "runtime_ref": "missing"}
    with pytest.raises(ValueError, match="not found"):
        resolve_backend(node, EXECUTION)


def test_resolve_backend_by_kind_llm():
    """llm kind is compatible with openai backend."""
    node = {"id": "n", "kind": "llm"}
    entry = resolve_backend(node, EXECUTION)
    assert entry["backend"] == "openai"


def test_resolve_backend_by_kind_retriever():
    """retriever kind is compatible with pgvector backend."""
    node = {"id": "n", "kind": "retriever"}
    entry = resolve_backend(node, EXECUTION)
    assert entry["backend"] == "pgvector"


def test_resolve_backend_no_match_returns_first():
    """When no compatible backend found, returns execution[0]."""
    node = {"id": "n", "kind": "unknown_kind"}
    entry = resolve_backend(node, EXECUTION)
    assert entry == EXECUTION[0]


def test_resolve_backend_empty_execution():
    """Empty execution list returns empty dict."""
    node = {"id": "n", "kind": "llm"}
    entry = resolve_backend(node, [])
    assert entry == {}


# ---------------------------------------------------------------------------
# resolve_callable / _default_callable
# ---------------------------------------------------------------------------

def test_default_callable_llm_node():
    """_default_callable for an llm node writes to context."""
    fn = _default_callable({"id": "chat", "kind": "llm"}, {})
    state = {"inputs": {}, "context": {}, "memory": {}, "tool_responses": {}}
    new_state = fn(state)
    assert "chat" in new_state["context"]


def test_default_callable_tool_node():
    """_default_callable for a tool node writes to tool_responses."""
    fn = _default_callable({"id": "fetch", "kind": "tool"}, {})
    state = {"inputs": {}, "context": {}, "memory": {}, "tool_responses": {}}
    new_state = fn(state)
    assert "fetch" in new_state["tool_responses"]


def test_default_callable_other_node():
    """_default_callable for other node kinds returns unchanged state keys."""
    fn = _default_callable({"id": "route", "kind": "router"}, {})
    state = {"inputs": {"q": "hi"}, "context": {}, "memory": {}, "tool_responses": {}}
    new_state = fn(state)
    assert new_state["inputs"] == {"q": "hi"}


def test_resolve_callable_no_tool_ref_no_factory():
    """resolve_callable returns _default_callable when no tool_ref and no backend_factory."""
    node = {"id": "n", "kind": "llm"}
    fn = resolve_callable(node, {}, None, {})
    state = {"inputs": {}, "context": {}, "memory": {}, "tool_responses": {}}
    result = fn(state)
    assert isinstance(result, dict)


def test_resolve_callable_with_factory_no_tool_ref():
    """resolve_callable calls backend_factory when no tool_ref."""
    called_with = {}
    def factory(n, e):
        called_with["node"] = n
        called_with["entry"] = e
        return lambda s: s
    node = {"id": "n", "kind": "llm"}
    entry = {"backend": "openai"}
    resolve_callable(node, {}, factory, entry)
    assert called_with["node"] == node
    assert called_with["entry"] == entry


def test_resolve_callable_tool_ref_found_no_factory():
    """resolve_callable finds tool entry and falls back to _default_callable."""
    manifest = {
        "tools": {
            "http_apis": [{"id": "billing-api", "base_url": "https://billing.example"}]
        }
    }
    node = {"id": "n", "kind": "tool", "tool_ref": "billing-api"}
    fn = resolve_callable(node, manifest, None, {})
    state = {"inputs": {}, "context": {}, "memory": {}, "tool_responses": {}}
    result = fn(state)
    assert isinstance(result, dict)


def test_resolve_callable_tool_ref_found_with_factory():
    """resolve_callable passes tool_entry to backend_factory when tool_ref is found."""
    factory_kwargs = {}
    def factory(n, e):
        factory_kwargs.update(e)
        return lambda s: s
    manifest = {
        "tools": {
            "mcp_servers": [{"id": "mcp-1", "url": "https://mcp.example"}]
        }
    }
    node = {"id": "n", "kind": "tool", "tool_ref": "mcp-1"}
    resolve_callable(node, manifest, factory, {"backend": "mcp"})
    assert "tool_entry" in factory_kwargs


def test_resolve_callable_tool_ref_not_found_raises():
    """resolve_callable raises ValueError when tool_ref not found in tools."""
    node = {"id": "n", "kind": "tool", "tool_ref": "nonexistent"}
    with pytest.raises(ValueError, match="not found in tools"):
        resolve_callable(node, {}, None, {})


# ---------------------------------------------------------------------------
# Import-failure path (exercises the except ImportError branch at module level)
# ---------------------------------------------------------------------------

def test_import_failure_path_sets_available_false():
    """Reloading langgraph module with langgraph blocked sets _AVAILABLE=False.

    This exercises lines 17-20 of langgraph.py (the except ImportError block)
    that cannot be reached during normal testing because langgraph IS installed.
    """
    import sys

    module_key = "adp_sdk.integrations.langgraph"

    # Save and evict the module
    saved_module = sys.modules.pop(module_key, None)
    # Block langgraph submodules
    blocked_keys = [k for k in sys.modules if k.startswith("langgraph")]
    saved_blocked = {k: sys.modules.pop(k) for k in blocked_keys}
    sys.modules["langgraph"] = None  # type: ignore[assignment]

    try:
        import adp_sdk.integrations.langgraph as _reloaded
        assert _reloaded._AVAILABLE is False
        assert _reloaded.END is None
        assert _reloaded.StateGraph is None
    finally:
        del sys.modules[module_key]
        del sys.modules["langgraph"]
        for k, v in saved_blocked.items():
            sys.modules[k] = v
        if saved_module is not None:
            sys.modules[module_key] = saved_module
