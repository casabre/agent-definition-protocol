"""
ADP ↔ LangGraph round-trip tests.

Tests:
  1. ADP → LangGraph: graph contains all manifest node IDs
  2. ADP → LangGraph → invoke: context and tool_responses are populated per ADR D2/D3
  3. ADP → LangGraph → ADP: node IDs are preserved (structural round-trip)
  4. ADP → LangGraph → ADP: edge from/to pairs are preserved

Install:
  pip install -r requirements.txt
  pip install -e ../../../sdk/python

Run:
  pytest -v
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from build_adp_graph import ADPState, build_langgraph_from_adp, adp_from_langgraph


def test_graph_has_correct_nodes(acme_manifest, mock_backend_factory):
    """ADP → LangGraph: compiled graph contains exactly the manifest node IDs."""
    graph, node_map = build_langgraph_from_adp(acme_manifest, mock_backend_factory)
    graph_nodes = {n for n in graph.get_graph().nodes if n not in ("__start__", "__end__")}
    manifest_nodes = {n["id"] for n in acme_manifest["flow"]["graph"]["nodes"]}
    assert graph_nodes == manifest_nodes


def test_graph_invocation_populates_state(acme_manifest, mock_backend_factory):
    """ADP → LangGraph → invoke: planner/executor/synthesizer write to state per ADR D2/D3."""
    graph, _ = build_langgraph_from_adp(acme_manifest, mock_backend_factory)
    initial: ADPState = {
        "inputs": {"query": "test"},
        "context": {},
        "memory": {},
        "tool_responses": {},
    }
    result = graph.invoke(initial)
    assert "planner" in result["context"], f"planner not in context: {result['context']}"
    assert "executor" in result["tool_responses"], f"executor not in tool_responses: {result['tool_responses']}"
    assert "synthesizer" in result["context"], f"synthesizer not in context: {result['context']}"


def test_round_trip_node_ids(acme_manifest, mock_backend_factory):
    """ADP → LangGraph → ADP: node IDs are structurally preserved."""
    graph, node_map = build_langgraph_from_adp(acme_manifest, mock_backend_factory)
    recovered = adp_from_langgraph(graph, node_map, acme_manifest)
    original_ids = {n["id"] for n in acme_manifest["flow"]["graph"]["nodes"]}
    recovered_ids = {n["id"] for n in recovered["flow"]["graph"]["nodes"]}
    assert original_ids == recovered_ids


def test_round_trip_edge_connections(acme_manifest, mock_backend_factory):
    """ADP → LangGraph → ADP: edge from/to pairs are structurally preserved."""
    graph, node_map = build_langgraph_from_adp(acme_manifest, mock_backend_factory)
    recovered = adp_from_langgraph(graph, node_map, acme_manifest)
    original_edges = {(e["from"], e["to"]) for e in acme_manifest["flow"]["graph"]["edges"]}
    recovered_edges = {(e["from"], e["to"]) for e in recovered["flow"]["graph"]["edges"]}
    assert original_edges == recovered_edges
