"""
ADP → AutoGen conversion utilities. Import-only: ADP → AutoGen.
Export (AutoGen → ADP) is deferred to v0.3.0.

Targets autogen_agentchat >= 0.4 (pyautogen >= 0.4 / pyautogen 0.10.x new API).
For the legacy pyautogen < 0.4 ConversableAgent API, use adp-sdk <= 0.2.

See spec/framework-interop.md §AutoGen Mapping for the full mapping guide.
"""
from __future__ import annotations
from typing import Any, Callable

try:
    from autogen_agentchat.agents import AssistantAgent  # pragma: no cover
    from autogen_agentchat.teams import RoundRobinGroupChat, SelectorGroupChat  # pragma: no cover
    _AVAILABLE = True  # pragma: no cover
except ImportError:
    AssistantAgent = None  # type: ignore[assignment, misc]
    RoundRobinGroupChat = None  # type: ignore[assignment]
    SelectorGroupChat = None  # type: ignore[assignment]
    _AVAILABLE = False

BackendFactory = Callable[[dict, dict], Any] | None


def build_autogen_from_adp(manifest: dict, backend_factory: BackendFactory = None) -> tuple[dict, list]:  # pragma: no cover
    """Build AutoGen agent map from an ADP manifest.

    Returns:
        (agent_map: dict[str, Any], chat_sequence: list)

    agent_map maps ADP node IDs to AssistantAgent instances.
    For router nodes the map also contains a RoundRobinGroupChat keyed as
    ``{node_id}_team`` to support group-based speaker selection.

    chat_sequence is a list of dicts ``{"from": node_id, "to": next_node_id}``
    derived from the flow graph edges, preserving edge order.

    If autogen_agentchat is not installed, raises ImportError at call time.
    """
    if not _AVAILABLE:
        raise ImportError(
            "autogen_agentchat >= 0.4 required: pip install 'adp-sdk[autogen]'"
        )

    flow = manifest["flow"]["graph"]
    nodes: list[dict] = flow["nodes"]
    edges: list[dict] = flow.get("edges", [])

    agent_map: dict[str, Any] = {}
    for node in nodes:
        node_id = node["id"]
        if backend_factory is not None:
            agent = backend_factory(node, {})
        else:
            agent = AssistantAgent(name=node_id, model_client=None)
        agent_map[node_id] = agent

    for node in nodes:
        if node.get("kind") == "router":
            node_id = node["id"]
            all_agents = [a for k, a in agent_map.items() if not k.endswith("_team")]
            team = RoundRobinGroupChat(participants=all_agents)
            agent_map[f"{node_id}_team"] = team

    chat_sequence: list[dict] = [
        {"from": edge["from"], "to": edge["to"]}
        for edge in edges
    ]

    return agent_map, chat_sequence
