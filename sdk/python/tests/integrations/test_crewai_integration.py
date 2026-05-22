"""
adp_sdk.integrations.crewai — SDK integration tests.

Mirrors examples/runners/crewai/test_roundtrip.py but imports from the SDK module.
Skips if crewai is not installed.
"""
import pytest

pytest.importorskip("crewai", reason="crewai required: pip install 'adp-sdk[crewai]'")

from adp_sdk.integrations.crewai import build_crewai_from_adp


def test_agents_created_for_all_nodes(simple_manifest):
    """ADP → CrewAI: agent_map contains an entry for every node."""
    _flow_class, agent_map = build_crewai_from_adp(simple_manifest)
    manifest_node_ids = {n["id"] for n in simple_manifest["flow"]["graph"]["nodes"]}
    assert set(agent_map.keys()) == manifest_node_ids


def test_composition_resolves_before_build(billing_manifest):
    """Composition: resolve_adp → CrewAI build → all manifest nodes have agent entries."""
    _flow_class, agent_map = build_crewai_from_adp(billing_manifest)
    manifest_node_ids = {n["id"] for n in billing_manifest["flow"]["graph"]["nodes"]}
    assert set(agent_map.keys()) == manifest_node_ids


def test_start_nodes_represented(simple_manifest):
    """All start_nodes from the manifest have entries in agent_map."""
    _flow_class, agent_map = build_crewai_from_adp(simple_manifest)
    start_nodes = simple_manifest["flow"]["graph"]["start_nodes"]
    for node_id in start_nodes:
        assert node_id in agent_map


def test_router_node_represented(router_manifest):
    """A node with kind='router' appears in agent_map."""
    _flow_class, agent_map = build_crewai_from_adp(router_manifest)
    router_nodes = [
        n["id"] for n in router_manifest["flow"]["graph"]["nodes"]
        if n.get("kind") == "router"
    ]
    for node_id in router_nodes:
        assert node_id in agent_map
