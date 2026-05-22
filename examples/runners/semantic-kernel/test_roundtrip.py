"""
ADP → Semantic Kernel import tests (mock mode).

These tests verify the ADP → SK conversion in mock mode and do NOT require
semantic_kernel to be installed. All four tests always run.

Tests:
  1. process_steps created for all manifest nodes (IDs match)
  2. composition resolves before build (billing manifest nodes all have steps)
  3. tool nodes are mapped to steps with kind == "tool"
  4. model_ref is resolved from runtime.models (provider + model name)

Install:
  pip install -r requirements.txt
  pip install -e ../../../sdk/python

Run:
  pytest -v
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from build_adp_graph import build_sk_from_adp


def test_process_steps_created_for_all_nodes(simple_manifest):
    """ADP → SK: process_steps contains one entry per manifest node, IDs match."""
    _kernel, process_steps = build_sk_from_adp(simple_manifest)
    manifest_node_ids = {n["id"] for n in simple_manifest["flow"]["graph"]["nodes"]}
    step_ids = {s["id"] for s in process_steps}
    assert len(process_steps) == len(simple_manifest["flow"]["graph"]["nodes"]), (
        f"Expected {len(simple_manifest['flow']['graph']['nodes'])} steps, got {len(process_steps)}"
    )
    assert step_ids == manifest_node_ids, (
        f"Step IDs {step_ids} do not match manifest node IDs {manifest_node_ids}"
    )


def test_composition_resolves_before_build(billing_manifest):
    """Composition: resolve_adp → SK build → all billing manifest nodes have steps."""
    billing_nodes = billing_manifest["flow"]["graph"]["nodes"]
    _kernel, process_steps = build_sk_from_adp(billing_manifest)
    manifest_node_ids = {n["id"] for n in billing_nodes}
    step_ids = {s["id"] for s in process_steps}
    assert len(process_steps) == len(billing_nodes), (
        f"Expected {len(billing_nodes)} steps, got {len(process_steps)}"
    )
    assert step_ids == manifest_node_ids, (
        f"Step IDs {step_ids} do not match manifest node IDs {manifest_node_ids}"
    )


def test_tool_nodes_mapped_to_steps(tool_manifest):
    """ADP → SK: tool node 'lookup' produces a step with kind == 'tool'."""
    _kernel, process_steps = build_sk_from_adp(tool_manifest)
    tool_steps = {s["id"]: s for s in process_steps if s.get("kind") == "tool"}
    assert "lookup" in tool_steps, (
        f"Expected tool step 'lookup', got steps: {[s['id'] for s in process_steps]}"
    )
    assert tool_steps["lookup"]["tool_ref"] == "billing-api"


def test_model_ref_resolved_from_runtime(simple_manifest):
    """ADP → SK: llm node 'chat' resolves model_ref 'gpt4' to provider/model from runtime.models."""
    _kernel, process_steps = build_sk_from_adp(simple_manifest)
    steps_by_id = {s["id"]: s for s in process_steps}
    assert "chat" in steps_by_id, f"Expected step 'chat', got: {list(steps_by_id)}"
    chat_step = steps_by_id["chat"]
    assert chat_step["kind"] == "llm"
    assert chat_step["model_ref"] == "gpt4"
    assert chat_step.get("provider") == "openai", (
        f"Expected provider 'openai', got: {chat_step.get('provider')}"
    )
    assert chat_step.get("model") == "gpt-4o-mini", (
        f"Expected model 'gpt-4o-mini', got: {chat_step.get('model')}"
    )
