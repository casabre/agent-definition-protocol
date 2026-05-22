"""
adp_sdk.integrations.autogen — SDK integration tests.

Targets autogen_agentchat >= 0.4 (pyautogen 0.10.x new API).
Skips if autogen_agentchat is not installed.
"""
import copy
import pytest

pytest.importorskip("autogen_agentchat", reason="autogen_agentchat required: pip install 'adp-sdk[autogen]'")

from adp_sdk.integrations.autogen import build_autogen_from_adp


def test_agents_created_for_all_nodes(simple_manifest):
    """ADP → AutoGen: agent_map contains exactly the node IDs from the manifest."""
    agent_map, _ = build_autogen_from_adp(simple_manifest)
    manifest_node_ids = {n["id"] for n in simple_manifest["flow"]["graph"]["nodes"]}
    base_agent_ids = {k for k in agent_map if not k.endswith("_team")}
    assert base_agent_ids == manifest_node_ids


def test_composition_resolves_before_build(billing_manifest):
    """Composition-resolved manifest drives AssistantAgent creation."""
    agent_map, _ = build_autogen_from_adp(billing_manifest)
    manifest_node_ids = {n["id"] for n in billing_manifest["flow"]["graph"]["nodes"]}
    base_agent_ids = {k for k in agent_map if not k.endswith("_team")}
    assert base_agent_ids == manifest_node_ids


def test_router_creates_team(router_manifest):
    """Router node produces a RoundRobinGroupChat keyed as {node_id}_team."""
    from autogen_agentchat.teams import RoundRobinGroupChat

    agent_map, _ = build_autogen_from_adp(router_manifest)
    router_nodes = [
        n["id"] for n in router_manifest["flow"]["graph"]["nodes"]
        if n.get("kind") == "router"
    ]
    for router_id in router_nodes:
        team_key = f"{router_id}_team"
        assert team_key in agent_map
        assert isinstance(agent_map[team_key], RoundRobinGroupChat)


def test_tool_node_creates_agent(simple_manifest):
    """Tool node gets its own AssistantAgent entry."""
    from autogen_agentchat.agents import AssistantAgent

    manifest = copy.deepcopy(simple_manifest)
    manifest["flow"]["graph"]["nodes"].append({"id": "fetch_data", "kind": "tool"})
    manifest["flow"]["graph"]["edges"].append({"from": "chat", "to": "fetch_data"})

    agent_map, _ = build_autogen_from_adp(manifest)
    assert "fetch_data" in agent_map
    assert isinstance(agent_map["fetch_data"], AssistantAgent)
