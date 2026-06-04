"""OpenAI Agents SDK adapter for ADP v0.3.0.

Converts between ADP manifests and OpenAI Agents SDK configurations.
"""

from typing import Any

from .base import AdapterBase
from .registry import AdapterRegistry
from ..adp_model import ADP


class OpenAIAgentsAdapter(AdapterBase):
    """Adapter for OpenAI Agents SDK framework.

    Converts ADP agents, handoffs, and guardrails to OpenAI Agents SDK configurations.
    """

    framework_id = "openai_agents"

    def __init__(self) -> None:
        AdapterRegistry.register(self.__class__)

    def export(self, manifest: ADP) -> dict[str, Any]:
        """Export ADP manifest to OpenAI Agents SDK config."""
        data = manifest.model_dump(by_alias=True, exclude_none=True)

        flow = data.get("flow", {})
        graph = flow.get("graph", {})
        nodes = graph.get("nodes", [])
        edges = graph.get("edges", [])
        tools = data.get("tools", {})
        guardrails = data.get("guardrails", {})
        observability = data.get("observability", {})

        # Map nodes to OpenAI agents
        agents: dict[str, dict[str, Any]] = {}
        handoffs: list[dict[str, Any]] = []

        for node in nodes:
            nid = node.get("id", "")
            kind = node.get("kind", "")

            if kind == "llm":
                agents[nid] = {
                    "name": nid,
                    "model": node.get("model_ref", "gpt-4o"),
                }
            elif kind == "tool":
                agents[nid] = {
                    "name": nid,
                    "tools": [node.get("tool_ref", "")],
                }

        # Build handoffs from edges
        for edge in edges:
            frm = edge.get("from", "")
            to = edge.get("to", "")
            condition = edge.get("condition")
            handoffs.append({
                "from": frm,
                "to": to,
                "condition": condition,
            })

        # Map tools
        oai_tools: list[dict[str, Any]] = []
        for tool_list_key in ("mcp_servers", "http_apis", "sql_functions"):
            for tool in tools.get(tool_list_key, []):
                oai_tools.append({
                    "name": tool.get("id"),
                    "description": tool.get("description", ""),
                })

        # Map guardrails interrupts to OpenAI guardrails
        oai_guardrails: dict[str, Any] = {}
        interrupts = guardrails.get("interrupts", [])
        for interrupt in interrupts:
            if isinstance(interrupt, dict):
                trigger = interrupt.get("trigger", "")
                if trigger == "cost_threshold":
                    oai_guardrails["cost_limit"] = {
                        "threshold": interrupt.get("threshold_usd", 10.0),
                        "action": interrupt.get("mode", "block"),
                    }

        # Map observability
        oai_observability: dict[str, Any] = {}
        if isinstance(observability, dict):
            tracing = observability.get("tracing", {})
            if isinstance(tracing, dict):
                oai_observability["tracing"] = {
                    "backend": tracing.get("backend", "stdout"),
                    "events": tracing.get("trace_events", []),
                }

        # Build OpenAI Agents config
        oai_config: dict[str, Any] = {
            "agents": list(agents.values()),
            "handoffs": handoffs,
            "tools": oai_tools,
            "guardrails": oai_guardrails,
            "observability": oai_observability,
        }

        return oai_config

    def import_from(self, config: dict[str, Any]) -> ADP:
        """Import OpenAI Agents SDK config to ADP manifest."""
        agents = config.get("agents", [])
        handoffs = config.get("handoffs", [])
        tools = config.get("tools", [])
        observability = config.get("observability", {})

        flow_nodes = []
        flow_edges = []

        # Convert agents to nodes
        for agent in agents:
            name = agent.get("name", "")
            model = agent.get("model", "")
            agent_tools = agent.get("tools", [])

            flow_nodes.append({
                "id": name,
                "kind": "llm",
                "model_ref": model,
            })

            # Add tool nodes
            for tool_name in agent_tools:
                flow_nodes.append({
                    "id": f"{name}_tool_{tool_name}",
                    "kind": "tool",
                    "tool_ref": tool_name,
                })
                flow_edges.append({
                    "from": name,
                    "to": f"{name}_tool_{tool_name}",
                })

        # Convert handoffs to edges
        for handoff in handoffs:
            flow_edges.append({
                "from": handoff.get("from", ""),
                "to": handoff.get("to", ""),
                "condition": handoff.get("condition"),
            })

        # Convert observability
        adp_observability: dict[str, Any] = {}
        if tracing := observability.get("tracing", {}):
            adp_observability["tracing"] = {
                "backend": tracing.get("backend"),
                "trace_events": tracing.get("events", []),
            }

        from ..adp_model import (
            RuntimeEntry,
            RuntimeModel,
            GraphModel,
            FlowModel,
            ToolsModel,
            HTTPAPIModel,
            ObservabilityModel,
            TracingModel,
        )

        http_apis = [
            HTTPAPIModel(
                id=t.get("name", ""),
                description=t.get("description", ""),
                base_url="",
            )
            for t in tools
        ] if tools else None

        tracing_config = None
        if adp_observability.get("tracing"):
            tracing_config = TracingModel(
                backend=adp_observability["tracing"].get("backend"),
                trace_events=adp_observability["tracing"].get("trace_events"),
            )

        return ADP(
            adp_version="0.3.0",
            id=config.get("id", "imported-from-openai-agents"),
            runtime=RuntimeModel(execution=[RuntimeEntry(
                backend="python",
                id="openai_agents",
                module="openai.agents",
            )]),
            flow=FlowModel(
                id="imported-flow",
                graph=GraphModel(
                    nodes=flow_nodes,
                    edges=flow_edges,
                    start_nodes=[flow_nodes[0]["id"]] if flow_nodes else [],
                    end_nodes=[],
                ),
            ),
            tools=ToolsModel(http_apis=http_apis) if http_apis else None,
            observability=ObservabilityModel(tracing=tracing_config) if tracing_config else None,
            extensions={"source_framework": "openai_agents"},
        )

    def roundtrip_fidelity(self) -> dict[str, str]:
        return {
            **super().roundtrip_fidelity(),
            "guardrails.interrupts": "faithful",
            "observability": "faithful",
        }
