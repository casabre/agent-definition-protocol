"""AutoGen adapter for ADP v0.3.0.

Converts between ADP manifests and AutoGen configurations.
"""

from typing import Any

from .base import AdapterBase
from .registry import AdapterRegistry
from ..adp_model import ADP


class AutoGenAdapter(AdapterBase):
    """Adapter for AutoGen framework.

    Converts ADP flow graphs to AutoGen GroupChat/Assistants configurations.
    """

    framework_id = "autogen"

    def __init__(self) -> None:
        AdapterRegistry.register(self.__class__)

    def export(self, manifest: ADP) -> dict[str, Any]:
        """Export ADP manifest to AutoGen config."""
        data = manifest.model_dump(by_alias=True, exclude_none=True)

        flow = data.get("flow", {})
        graph = flow.get("graph", {})
        nodes = graph.get("nodes", [])

        # Map ADP nodes to AutoGen agents/tools
        agents: dict[str, dict[str, Any]] = {}
        tools_list: list[dict[str, Any]] = []

        for node in nodes:
            nid = node.get("id", "")
            kind = node.get("kind", "")

            if kind == "llm":
                agents[nid] = {
                    "type": "AssistantAgent",
                    "model": node.get("model_ref", "gpt-4"),
                }
            elif kind == "tool":
                tool_ref = node.get("tool_ref", "")
                tools_list.append({
                    "name": nid,
                    "function": tool_ref,
                })
                agents[nid] = {
                    "type": "ToolAgent",
                    "name": nid,
                }
            elif kind == "router":
                agents[nid] = {
                    "type": "RouterAgent",
                    "strategy": node.get("strategy", "round_robin"),
                }

        # Build group chat from loop nodes
        loop_policy = flow.get("loop_policy", {})
        group_chat: dict[str, Any] = {
            "agents": list(agents.values()),
            "tools": tools_list,
        }

        if max_iterations := loop_policy.get("default_max_iterations"):
            group_chat["max_turns"] = max_iterations

        # Apply adapter hints
        adapter_hints = (data.get("runtime", {}).get("adapter_hints", {})
                         .get("autogen", {}))

        if human_input_mode := adapter_hints.get("human_input_mode"):
            group_chat["human_input_mode"] = human_input_mode
        if max_turns := adapter_hints.get("max_turns"):
            group_chat["max_turns"] = max_turns

        return group_chat

    def import_from(self, config: dict[str, Any]) -> ADP:
        """Import AutoGen config to ADP manifest."""
        agents = config.get("agents", [])
        tools = config.get("tools", [])
        max_turns = config.get("max_turns")

        flow_nodes = []
        flow_edges = []

        # Convert agents to nodes
        for agent in agents:
            agent_type = agent.get("type", "")
            name = agent.get("name", agent.get("model", "agent"))

            if agent_type == "AssistantAgent":
                flow_nodes.append({
                    "id": name,
                    "kind": "llm",
                    "model_ref": agent.get("model"),
                })
            elif agent_type == "ToolAgent":
                flow_nodes.append({
                    "id": name,
                    "kind": "tool",
                })
            else:
                flow_nodes.append({
                    "id": name,
                    "kind": "router",
                })

        # Connect tools to agents
        for tool in tools:
            tool_name = tool.get("name", "")
            function = tool.get("function", "")
            flow_nodes.append({
                "id": f"tool_{tool_name}",
                "kind": "tool",
                "tool_ref": function,
            })
            # Connect last agent to tool
            if flow_nodes:
                flow_edges.append({
                    "from": flow_nodes[-1]["id"],
                    "to": f"tool_{tool_name}",
                })

        # Extract adapter hints
        adapter_hints = {}
        if human_input_mode := config.get("human_input_mode"):
            adapter_hints["human_input_mode"] = human_input_mode
        if max_turns:
            adapter_hints["max_turns"] = max_turns

        from ..adp_model import (
            RuntimeEntry,
            RuntimeModel,
            GraphModel,
            FlowModel,
            LoopPolicyModel,
        )

        return ADP(
            adp_version="0.3.0",
            id=config.get("id", "imported-from-autogen"),
            runtime=RuntimeModel(execution=[RuntimeEntry(
                backend="python",
                id="autogen",
                module="autogen",
            )]),
            flow=FlowModel(
                id="imported-flow",
                graph=GraphModel(
                    nodes=flow_nodes,
                    edges=flow_edges,
                    start_nodes=[flow_nodes[0]["id"]] if flow_nodes else [],
                    end_nodes=[],
                ),
                loop_policy=LoopPolicyModel(
                    default_max_iterations=max_turns,
                ) if max_turns else None,
                extensions={"autogen": adapter_hints} if adapter_hints else None,
            ),
            extensions={"source_framework": "autogen"},
        )

    def roundtrip_fidelity(self) -> dict[str, str]:
        return {
            **super().roundtrip_fidelity(),
            "loop.termination": "faithful",  # via max_turns
        }
