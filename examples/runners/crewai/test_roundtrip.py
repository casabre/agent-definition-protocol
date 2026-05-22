"""
ADP → CrewAI conversion tests. Import-only: ADP → CrewAI.
Export (CrewAI → ADP) is deferred to v0.3.0.

Tests:
  1. test_agents_created_for_all_nodes    — agent_map.keys() matches all node IDs
  2. test_composition_resolves_before_build — billing manifest nodes all have agent entries
  3. test_start_nodes_represented         — start node IDs are in agent_map
  4. test_router_node_represented         — router node is in agent_map

Install:
  pip install -r requirements.txt
  pip install -e ../../../sdk/python

Run:
  pytest -v
"""
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent))
from build_adp_graph import build_crewai_from_adp

# Skip the entire module when crewai is not installed.
# Remove this line and use per-test pytest.importorskip if you want to
# run the mock-mode tests unconditionally.
crewai = pytest.importorskip("crewai", reason="crewai >= 0.63 required: pip install crewai>=0.63")


def test_agents_created_for_all_nodes(simple_manifest):
    """ADP → CrewAI: agent_map contains an entry for every node declared in the manifest."""
    _flow_class, agent_map = build_crewai_from_adp(simple_manifest)
    manifest_node_ids = {n["id"] for n in simple_manifest["flow"]["graph"]["nodes"]}
    assert set(agent_map.keys()) == manifest_node_ids, (
        f"agent_map keys {set(agent_map.keys())} != manifest nodes {manifest_node_ids}"
    )


def test_composition_resolves_before_build(billing_manifest):
    """Composition: resolve_adp → CrewAI build → all manifest nodes have agent entries."""
    _flow_class, agent_map = build_crewai_from_adp(billing_manifest)
    manifest_node_ids = {n["id"] for n in billing_manifest["flow"]["graph"]["nodes"]}
    assert set(agent_map.keys()) == manifest_node_ids, (
        f"Composition build failed: agent_map={set(agent_map.keys())} manifest={manifest_node_ids}"
    )


def test_start_nodes_represented(simple_manifest):
    """ADP → CrewAI: all start_nodes from the manifest have entries in agent_map."""
    _flow_class, agent_map = build_crewai_from_adp(simple_manifest)
    start_nodes = simple_manifest["flow"]["graph"]["start_nodes"]
    for node_id in start_nodes:
        assert node_id in agent_map, (
            f"start_node '{node_id}' missing from agent_map (keys: {list(agent_map.keys())})"
        )


def test_router_node_represented(router_manifest):
    """ADP → CrewAI: a node with kind='router' appears in agent_map."""
    _flow_class, agent_map = build_crewai_from_adp(router_manifest)
    router_nodes = [
        n["id"]
        for n in router_manifest["flow"]["graph"]["nodes"]
        if n.get("kind") == "router"
    ]
    assert router_nodes, "router_manifest fixture has no router node — test setup error"
    for node_id in router_nodes:
        assert node_id in agent_map, (
            f"router node '{node_id}' missing from agent_map (keys: {list(agent_map.keys())})"
        )
