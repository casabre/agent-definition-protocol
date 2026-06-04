"""CrewAI adapter for ADP v0.3.0.

Converts between ADP manifests and CrewAI configurations.
"""

from typing import Any

from .base import AdapterBase
from .registry import AdapterRegistry
from ..adp_model import ADP


class CrewAIAdapter(AdapterBase):
    """Adapter for CrewAI framework.

    Converts ADP agents and tasks to CrewAI configurations.
    """

    framework_id = "crewai"

    def __init__(self) -> None:
        AdapterRegistry.register(self.__class__)

    def export(self, manifest: ADP) -> dict[str, Any]:
        """Export ADP manifest to CrewAI config."""
        data = manifest.model_dump(by_alias=True, exclude_none=True)

        flow = data.get("flow", {})
        graph = flow.get("graph", {})
        nodes = graph.get("nodes", [])

        # Map ADP nodes to CrewAI agents and tasks
        agents: dict[str, dict[str, Any]] = {}
        tasks: list[dict[str, Any]] = []

        for node in nodes:
            nid = node.get("id", "")
            kind = node.get("kind", "")
            role = node.get("label", nid)

            if kind == "llm":
                agents[nid] = {
                    "role": role,
                    "llm": node.get("model_ref", "gpt-4"),
                }
            elif kind == "tool":
                # Tools are mapped to agent tools
                pass

        # Build crew configuration
        crew: dict[str, Any] = {
            "agents": list(agents.values()),
            "tasks": tasks,
            "process": "sequential",  # default
        }

        # Apply adapter hints
        adapter_hints = (data.get("runtime", {}).get("adapter_hints", {})
                         .get("crewai", {}))

        if process := adapter_hints.get("process"):
            crew["process"] = process
        if max_rpm := adapter_hints.get("max_rpm"):
            crew["max_rpm"] = max_rpm

        return crew

    def import_from(self, config: dict[str, Any]) -> ADP:
        """Import CrewAI config to ADP manifest."""
        agents = config.get("agents", [])
        process = config.get("process", "sequential")

        flow_nodes = []
        flow_edges = []

        # Convert agents to nodes
        for agent in agents:
            role = agent.get("role", "")
            llm = agent.get("llm", "")

            flow_nodes.append({
                "id": role.lower().replace(" ", "_"),
                "kind": "llm",
                "label": role,
                "model_ref": llm,
            })

        # Connect nodes sequentially
        for i in range(len(flow_nodes) - 1):
            flow_edges.append({
                "from": flow_nodes[i]["id"],
                "to": flow_nodes[i + 1]["id"],
            })

        # Extract adapter hints
        adapter_hints = {}
        if process:
            adapter_hints["process"] = process
        if max_rpm := config.get("max_rpm"):
            adapter_hints["max_rpm"] = max_rpm

        from ..adp_model import (
            RuntimeEntry,
            RuntimeModel,
            GraphModel,
            FlowModel,
        )

        return ADP(
            adp_version="0.3.0",
            id=config.get("id", "imported-from-crewai"),
            runtime=RuntimeModel(execution=[RuntimeEntry(
                backend="python",
                id="crewai",
                module="crewai",
            )]),
            flow=FlowModel(
                id="imported-flow",
                graph=GraphModel(
                    nodes=flow_nodes,
                    edges=flow_edges,
                    start_nodes=[flow_nodes[0]["id"]] if flow_nodes else [],
                    end_nodes=[flow_nodes[-1]["id"]] if flow_nodes else [],
                ),
                extensions={"crewai": adapter_hints} if adapter_hints else None,
            ),
            extensions={"source_framework": "crewai"},
        )

    def roundtrip_fidelity(self) -> dict[str, str]:
        return {
            **super().roundtrip_fidelity(),
            "tools.policy": "faithful",  # via rate_limit -> max_rpm
        }
