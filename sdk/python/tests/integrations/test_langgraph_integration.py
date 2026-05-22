"""
adp_sdk.integrations.langgraph — SDK integration tests.

Mirrors examples/runners/langgraph/test_roundtrip.py but imports from the SDK module.
Skips if langgraph is not installed.
"""
import pytest

pytest.importorskip("langgraph", reason="langgraph required: pip install 'adp-sdk[langgraph]'")

from adp_sdk.integrations.langgraph import ADPState, build_langgraph_from_adp, adp_from_langgraph


def test_graph_has_correct_nodes(simple_manifest, mock_backend_factory):
    """ADP → LangGraph: compiled graph contains exactly the manifest node IDs."""
    graph, node_map = build_langgraph_from_adp(simple_manifest, mock_backend_factory)
    graph_nodes = {n for n in graph.get_graph().nodes if n not in ("__start__", "__end__")}
    manifest_nodes = {n["id"] for n in simple_manifest["flow"]["graph"]["nodes"]}
    assert graph_nodes == manifest_nodes


def test_graph_invocation_populates_state(simple_manifest, mock_backend_factory):
    """ADP → LangGraph → invoke: llm node writes to context per ADR D2."""
    graph, _ = build_langgraph_from_adp(simple_manifest, mock_backend_factory)
    initial: ADPState = {
        "inputs": {"query": "test"},
        "context": {},
        "memory": {},
        "tool_responses": {},
    }
    result = graph.invoke(initial)
    assert "chat" in result["context"], f"llm node 'chat' not in context: {result['context']}"


def test_round_trip_node_ids(simple_manifest, mock_backend_factory):
    """ADP → LangGraph → ADP: node IDs are structurally preserved."""
    graph, node_map = build_langgraph_from_adp(simple_manifest, mock_backend_factory)
    recovered = adp_from_langgraph(graph, node_map, simple_manifest)
    original_ids = {n["id"] for n in simple_manifest["flow"]["graph"]["nodes"]}
    recovered_ids = {n["id"] for n in recovered["flow"]["graph"]["nodes"]}
    assert original_ids == recovered_ids


def test_composition_roundtrip(billing_manifest, mock_backend_factory):
    """Composition: resolve_adp → LangGraph build → all manifest nodes in graph."""
    graph, node_map = build_langgraph_from_adp(billing_manifest, mock_backend_factory)
    graph_nodes = {n for n in graph.get_graph().nodes if n not in ("__start__", "__end__")}
    manifest_nodes = {n["id"] for n in billing_manifest["flow"]["graph"]["nodes"]}
    assert graph_nodes == manifest_nodes, (
        f"Composition round-trip failed: graph={graph_nodes} manifest={manifest_nodes}"
    )
