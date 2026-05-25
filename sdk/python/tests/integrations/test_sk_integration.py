"""
adp_sdk.integrations.semantic_kernel — SDK integration tests.

Mirrors examples/runners/semantic-kernel/test_roundtrip.py but imports from the SDK module.
All 4 tests run in mock mode (no semantic_kernel installation required).
"""
from adp_sdk.integrations.semantic_kernel import build_sk_from_adp


def test_process_steps_created_for_all_nodes(simple_manifest):
    """ADP → SK: process_steps contains one entry per manifest node, IDs match."""
    _kernel, process_steps = build_sk_from_adp(simple_manifest)
    manifest_node_ids = {n["id"] for n in simple_manifest["flow"]["graph"]["nodes"]}
    step_ids = {s["id"] for s in process_steps}
    assert len(process_steps) == len(simple_manifest["flow"]["graph"]["nodes"])
    assert step_ids == manifest_node_ids


def test_composition_resolves_before_build(billing_manifest):
    """Composition: resolve_adp → SK build → all billing manifest nodes have steps."""
    billing_nodes = billing_manifest["flow"]["graph"]["nodes"]
    _kernel, process_steps = build_sk_from_adp(billing_manifest)
    manifest_node_ids = {n["id"] for n in billing_nodes}
    step_ids = {s["id"] for s in process_steps}
    assert len(process_steps) == len(billing_nodes)
    assert step_ids == manifest_node_ids


def test_tool_nodes_mapped_to_steps(tool_manifest):
    """ADP → SK: tool node 'lookup' produces a step with kind == 'tool'."""
    _kernel, process_steps = build_sk_from_adp(tool_manifest)
    tool_steps = {s["id"]: s for s in process_steps if s.get("kind") == "tool"}
    assert "lookup" in tool_steps
    assert tool_steps["lookup"]["tool_ref"] == "billing-api"


def test_model_ref_resolved_from_runtime(simple_manifest):
    """ADP → SK: llm node 'chat' resolves model_ref 'gpt4' to provider/model."""
    _kernel, process_steps = build_sk_from_adp(simple_manifest)
    steps_by_id = {s["id"]: s for s in process_steps}
    assert "chat" in steps_by_id
    chat_step = steps_by_id["chat"]
    assert chat_step["kind"] == "llm"
    assert chat_step["model_ref"] == "gpt4"
    assert chat_step.get("provider") == "openai"
    assert chat_step.get("model") == "gpt-4o-mini"


def test_resolve_model_returns_none_for_none_ref():
    """_resolve_model returns None when model_ref is None (line 26 branch)."""
    from adp_sdk.integrations.semantic_kernel import _resolve_model
    result = _resolve_model(None, [{"id": "m1", "provider": "openai", "model": "gpt-4o"}])
    assert result is None


def test_resolve_model_returns_none_for_unknown_ref():
    """_resolve_model returns None when no model matches the ref (line 30 branch)."""
    from adp_sdk.integrations.semantic_kernel import _resolve_model
    result = _resolve_model("nonexistent", [{"id": "m1"}, {"id": "m2"}])
    assert result is None
