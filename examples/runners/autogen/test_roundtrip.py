"""
ADP → AutoGen import tests.

Import-only (ADP → AutoGen). Export (AutoGen → ADP) is deferred to v0.3.0.

Tests:
  1. test_agents_created_for_all_nodes       — agent_map keys match all manifest node IDs
  2. test_composition_resolves_before_build  — composition-resolved manifest drives agent creation
  3. test_router_creates_team               — router node produces a *_team entry (RoundRobinGroupChat)
  4. test_tool_node_creates_agent            — tool node gets its own AssistantAgent entry

Install:
  pip install pyautogen>=0.4
  pip install -e ../../../sdk/python

Run:
  pytest -v
"""
import sys
from pathlib import Path
import copy

import pytest

autogen_agentchat = pytest.importorskip("autogen_agentchat")

sys.path.insert(0, str(Path(__file__).parent))
from build_adp_graph import build_autogen_from_adp


def test_agents_created_for_all_nodes(simple_manifest):
    """ADP → AutoGen: agent_map contains exactly the node IDs declared in the manifest."""
    agent_map, _ = build_autogen_from_adp(simple_manifest)
    manifest_node_ids = {n["id"] for n in simple_manifest["flow"]["graph"]["nodes"]}
    base_agent_ids = {k for k in agent_map if not k.endswith("_team")}
    assert base_agent_ids == manifest_node_ids, (
        f"Expected agent_map keys {manifest_node_ids}, got {base_agent_ids}"
    )


def test_composition_resolves_before_build(billing_manifest):
    """Composition-resolved manifest (billing-variant) drives AssistantAgent creation."""
    agent_map, _ = build_autogen_from_adp(billing_manifest)
    manifest_node_ids = {n["id"] for n in billing_manifest["flow"]["graph"]["nodes"]}
    base_agent_ids = {k for k in agent_map if not k.endswith("_team")}
    assert base_agent_ids == manifest_node_ids, (
        f"Composition round-trip failed: agent_map={base_agent_ids} manifest={manifest_node_ids}"
    )


def test_router_creates_team(router_manifest):
    """Router node kind produces a RoundRobinGroupChat keyed as {node_id}_team."""
    from autogen_agentchat.teams import RoundRobinGroupChat

    agent_map, _ = build_autogen_from_adp(router_manifest)
    router_nodes = [
        n["id"]
        for n in router_manifest["flow"]["graph"]["nodes"]
        if n.get("kind") == "router"
    ]
    assert router_nodes, "Test fixture must contain at least one router node"
    for router_id in router_nodes:
        team_key = f"{router_id}_team"
        assert team_key in agent_map, (
            f"Expected '{team_key}' in agent_map for router node '{router_id}'. "
            f"Keys present: {list(agent_map.keys())}"
        )
        assert isinstance(agent_map[team_key], RoundRobinGroupChat), (
            f"Expected RoundRobinGroupChat for '{team_key}', "
            f"got {type(agent_map[team_key])}"
        )


def test_tool_node_creates_agent(simple_manifest):
    """Tool node added to the manifest gets its own AssistantAgent entry."""
    from autogen_agentchat.agents import AssistantAgent

    manifest = copy.deepcopy(simple_manifest)
    tool_node = {"id": "fetch_data", "kind": "tool", "tool_ref": "billing-api"}
    manifest["flow"]["graph"]["nodes"].append(tool_node)
    manifest["flow"]["graph"]["edges"].append({"from": "llm", "to": "fetch_data"})

    agent_map, _ = build_autogen_from_adp(manifest)

    assert "fetch_data" in agent_map, (
        f"Expected 'fetch_data' in agent_map. Keys: {list(agent_map.keys())}"
    )
    assert isinstance(agent_map["fetch_data"], AssistantAgent), (
        f"Expected AssistantAgent for 'fetch_data', got {type(agent_map['fetch_data'])}"
    )
