"""Pydantic AI adapter for ADP v0.3.0.

Converts between ADP manifests and Pydantic AI configurations.
"""

from typing import Any

from .base import AdapterBase
from .registry import AdapterRegistry
from ..adp_model import ADP


class PydanticAIAdapter(AdapterBase):
    """Adapter for Pydantic AI framework.

    Converts ADP agents and tools to Pydantic AI configurations.
    """

    framework_id = "pydantic_ai"

    def __init__(self) -> None:
        AdapterRegistry.register(self.__class__)

    def export(self, manifest: ADP) -> dict[str, Any]:
        """Export ADP manifest to Pydantic AI config."""
        data = manifest.model_dump(by_alias=True, exclude_none=True)

        flow = data.get("flow", {})
        graph = flow.get("graph", {})
        nodes = graph.get("nodes", [])
        edges = graph.get("edges", [])
        tools = data.get("tools", {})
        runtime = data.get("runtime", {})

        # Map nodes to Pydantic AI agents
        agents: dict[str, dict[str, Any]] = {}

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
                    "deps": {"type": "Tool"},
                }

        # Build adjacency list from edges
        adjacency: dict[str, list[str]] = {}
        for edge in edges:
            frm = edge.get("from", "")
            to = edge.get("to", "")
            if frm not in adjacency:
                adjacency[frm] = []
            adjacency[frm].append(to)

        # Map tools to Pydantic AI tool types
        pydantic_tools: list[dict[str, Any]] = []
        for tool_list_key in ("mcp_servers", "http_apis", "sql_functions"):
            for tool in tools.get(tool_list_key, []):
                pydantic_tools.append({
                    "name": tool.get("id"),
                    "description": tool.get("description", ""),
                    "type": tool_list_key.replace("_", "").title(),
                })

        # Map runtime models
        models = runtime.get("models", [])
        pydantic_models: dict[str, dict[str, Any]] = {}
        for model in models:
            pydantic_models[model.get("id", "")] = {
                "provider": model.get("provider", "openai"),
                "model": model.get("model", "gpt-4o"),
            }

        # Build Pydantic AI config
        pydantic_config: dict[str, Any] = {
            "agents": agents,
            "adjacency": adjacency,
            "tools": pydantic_tools,
            "models": pydantic_models,
        }

        # Apply adapter hints
        adapter_hints = (runtime.get("adapter_hints", {})
                         .get("pydantic_ai", {}))

        if embedder_config := adapter_hints.get("embedder_config"):
            pydantic_config["embedder"] = embedder_config

        return pydantic_config

    def import_from(self, config: dict[str, Any]) -> ADP:
        """Import Pydantic AI config to ADP manifest."""
        agents = config.get("agents", {})
        adjacency = config.get("adjacency", {})
        tools = config.get("tools", [])
        models = config.get("models", {})
        embedder = config.get("embedder")

        flow_nodes = []
        flow_edges = []

        # Convert agents to nodes
        for agent_id, agent_config in agents.items():
            name = agent_config.get("name", agent_id)
            model = agent_config.get("model", "")
            deps = agent_config.get("deps", {})
            deps_type = deps.get("type", "")

            if deps_type == "Tool":
                flow_nodes.append({
                    "id": name,
                    "kind": "tool",
                })
            else:
                flow_nodes.append({
                    "id": name,
                    "kind": "llm",
                    "model_ref": model,
                })

        # Convert adjacency to edges
        for frm, tos in adjacency.items():
            for to in tos:
                flow_edges.append({
                    "from": frm,
                    "to": to,
                })

        # Extract adapter hints
        adapter_hints = {}
        if embedder:
            adapter_hints["embedder_config"] = embedder

        # Convert models to runtime models
        runtime_models = [
            {
                "id": model_id,
                "provider": model_config.get("provider", "openai"),
                "model": model_config.get("model", "gpt-4o"),
            }
            for model_id, model_config in models.items()
        ] if models else []

        from ..adp_model import (
            RuntimeEntry,
            RuntimeModel,
            GraphModel,
            FlowModel,
            ToolsModel,
            HTTPAPIModel,
        )

        http_apis = [
            HTTPAPIModel(
                id=t.get("name", ""),
                description=t.get("description", ""),
                base_url="",
            )
            for t in tools
            if t.get("type") == "HttpApis"
        ] if tools else None

        return ADP(
            adp_version="0.3.0",
            id=config.get("id", "imported-from-pydantic-ai"),
            runtime=RuntimeModel(
                execution=[RuntimeEntry(
                    backend="python",
                    id="pydantic_ai",
                    module="pydantic_ai",
                )],
                models=runtime_models,
                adapter_hints={"pydantic_ai": adapter_hints} if adapter_hints else None,
            ),
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
            extensions={"source_framework": "pydantic_ai"},
        )

    def roundtrip_fidelity(self) -> dict[str, str]:
        return {
            **super().roundtrip_fidelity(),
            "runtime.models": "faithful",
        }
