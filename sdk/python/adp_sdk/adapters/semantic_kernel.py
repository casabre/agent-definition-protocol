"""Semantic Kernel adapter for ADP v0.3.0.

Converts between ADP manifests and Semantic Kernel configurations.
"""

from typing import Any

from .base import AdapterBase
from .registry import AdapterRegistry
from ..adp_model import ADP


class SemanticKernelAdapter(AdapterBase):
    """Adapter for Semantic Kernel framework.

    Converts ADP plugins/functions to Semantic Kernel configurations.
    """

    framework_id = "semantic_kernel"

    def __init__(self) -> None:
        AdapterRegistry.register(self.__class__)

    def export(self, manifest: ADP) -> dict[str, Any]:
        """Export ADP manifest to Semantic Kernel config."""
        data = manifest.model_dump(by_alias=True, exclude_none=True)

        flow = data.get("flow", {})
        graph = flow.get("graph", {})
        nodes = graph.get("nodes", [])
        edges = graph.get("edges", [])
        tools = data.get("tools", {})
        runtime = data.get("runtime", {})

        # Map nodes to Semantic Kernel steps/agents
        steps: dict[str, dict[str, Any]] = {}

        for node in nodes:
            nid = node.get("id", "")
            kind = node.get("kind", "")

            if kind == "llm":
                steps[nid] = {
                    "type": "LLMService",
                    "model": node.get("model_ref", "gpt-4o"),
                }
            elif kind == "tool":
                steps[nid] = {
                    "type": "Function",
                    "name": node.get("tool_ref", nid),
                }
            elif kind == "retriever":
                steps[nid] = {
                    "type": "Retriever",
                    "memory": node.get("memory_ref"),
                }
            else:
                steps[nid] = {"type": "Node"}

        # Build workflow from edges
        workflow: list[dict[str, Any]] = []
        for edge in edges:
            workflow.append({
                "from": edge.get("from", ""),
                "to": edge.get("to", ""),
            })

        # Map tools to Semantic Kernel plugins
        plugins: dict[str, dict[str, Any]] = {}
        for tool_list_key in ("mcp_servers", "http_apis", "sql_functions"):
            for tool in tools.get(tool_list_key, []):
                plugins[tool.get("id", "")] = {
                    "type": tool_list_key.replace("_", "").title(),
                    "description": tool.get("description", ""),
                    "endpoint": tool.get("endpoint", tool.get("base_url", "")),
                }

        # Map runtime to AI services
        ai_services: dict[str, dict[str, Any]] = {}
        models = runtime.get("models", [])
        for model in models:
            ai_services[model.get("id", "")] = {
                "provider": model.get("provider", "openai"),
                "model": model.get("model", "gpt-4o"),
            }

        # Build Semantic Kernel config
        sk_config: dict[str, Any] = {
            "plugins": plugins,
            "ai_services": ai_services,
            "steps": steps,
            "workflow": workflow,
        }

        # Apply adapter hints
        adapter_hints = (runtime.get("adapter_hints", {})
                         .get("semantic_kernel", {}))

        # Semantic Kernel doesn't have specific adapter hints in v0.3.0
        # but we include any provided
        if adapter_hints:
            sk_config["hints"] = adapter_hints

        return sk_config

    def import_from(self, config: dict[str, Any]) -> ADP:
        """Import Semantic Kernel config to ADP manifest."""
        plugins = config.get("plugins", {})
        ai_services = config.get("ai_services", {})
        steps = config.get("steps", {})
        workflow = config.get("workflow", [])

        flow_nodes = []
        flow_edges = []

        # Convert steps to nodes
        for step_id, step_config in steps.items():
            step_type = step_config.get("type", "")

            kind_map = {
                "LLMService": "llm",
                "Function": "tool",
                "Retriever": "retriever",
                "Router": "router",
            }

            node = {"id": step_id, "kind": kind_map.get(step_type, "tool")}

            if step_type == "LLMService":
                node["model_ref"] = step_config.get("model")
            elif step_type == "Function":
                node["tool_ref"] = step_config.get("name")
            elif step_type == "Retriever":
                node["memory_ref"] = step_config.get("memory")

            flow_nodes.append(node)

        # Convert workflow to edges
        for wf in workflow:
            flow_edges.append({
                "from": wf.get("from", ""),
                "to": wf.get("to", ""),
            })

        # Convert plugins to tools
        http_apis = []
        for plugin_id, plugin_config in plugins.items():
            plugin_type = plugin_config.get("type", "")
            if plugin_type == "HttpApis":
                http_apis.append({
                    "id": plugin_id,
                    "description": plugin_config.get("description", ""),
                    "base_url": plugin_config.get("endpoint", ""),
                })

        # Convert AI services to runtime models
        runtime_models = [
            {
                "id": service_id,
                "provider": service_config.get("provider", "openai"),
                "model": service_config.get("model", "gpt-4o"),
            }
            for service_id, service_config in ai_services.items()
        ] if ai_services else []

        from ..adp_model import (
            RuntimeEntry,
            RuntimeModel,
            GraphModel,
            FlowModel,
            ToolsModel,
            HTTPAPIModel,
        )

        return ADP(
            adp_version="0.3.0",
            id=config.get("id", "imported-from-semantic-kernel"),
            runtime=RuntimeModel(
                execution=[RuntimeEntry(
                    backend="python",
                    id="semantic_kernel",
                    module="semantic_kernel",
                )],
                models=runtime_models,
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
            tools=ToolsModel(http_apis=[
                HTTPAPIModel(
                    id=api.get("id", ""),
                    description=api.get("description", ""),
                    base_url=api.get("base_url", ""),
                )
                for api in http_apis
            ]) if http_apis else None,
            extensions={"source_framework": "semantic_kernel"},
        )

    def roundtrip_fidelity(self) -> dict[str, str]:
        return {
            **super().roundtrip_fidelity(),
            "runtime.models": "faithful",
            "tools": "faithful",
        }
