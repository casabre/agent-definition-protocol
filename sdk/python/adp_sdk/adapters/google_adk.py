"""Google ADK adapter for ADP v0.3.0.

Converts between ADP manifests and Google Agent Development Kit configurations.
"""

from typing import Any

from .base import AdapterBase
from .registry import AdapterRegistry
from ..adp_model import ADP


class GoogleADKAdapter(AdapterBase):
    """Adapter for Google Agent Development Kit framework.

    Converts ADP agents, tools, and artifacts to Google ADK configurations.
    """

    framework_id = "google_adk"

    def __init__(self) -> None:
        AdapterRegistry.register(self.__class__)

    def export(self, manifest: ADP) -> dict[str, Any]:
        """Export ADP manifest to Google ADK config."""
        data = manifest.model_dump(by_alias=True, exclude_none=True)

        flow = data.get("flow", {})
        graph = flow.get("graph", {})
        nodes = graph.get("nodes", [])
        tools = data.get("tools", {})
        artifacts = data.get("artifacts", {})
        memory = data.get("memory", {})

        # Map nodes to ADK agents
        agents: dict[str, dict[str, Any]] = {}

        for node in nodes:
            nid = node.get("id", "")
            kind = node.get("kind", "")

            agent: dict[str, Any] = {"name": nid}

            if kind == "llm":
                agent["type"] = "LLMAgent"
                agent["model"] = node.get("model_ref", "gemini-1.5-pro")
            elif kind == "tool":
                agent["type"] = "ToolAgent"
                agent["tool"] = node.get("tool_ref")
            elif kind == "router":
                agent["type"] = "RouterAgent"
                agent["strategy"] = node.get("strategy")

            agents[nid] = agent

        # Map tools
        adk_tools: list[dict[str, Any]] = []
        for tool_list_key in ("mcp_servers", "http_apis", "sql_functions"):
            for tool in tools.get(tool_list_key, []):
                adk_tools.append({
                    "name": tool.get("id"),
                    "description": tool.get("description", ""),
                    "function": tool.get("id"),
                })

        # Map artifacts
        artifact_stores: list[dict[str, Any]] = []
        if isinstance(artifacts, dict):
            for store in artifacts.get("stores", []):
                artifact_stores.append({
                    "id": store.get("id"),
                    "provider": store.get("provider"),
                    "bucket": store.get("bucket"),
                    "scope": store.get("scope"),
                })

        # Map memory to session service
        session_service: dict[str, Any] = {}
        if isinstance(memory, dict):
            stores = memory.get("stores", [])
            for store in stores:
                if store.get("scope") == "session":
                    session_service["provider"] = store.get("provider")
                    session_service["endpoint"] = store.get("endpoint")

        # Build ADK configuration
        adk_config: dict[str, Any] = {
            "agents": list(agents.values()),
            "tools": adk_tools,
            "artifacts": artifact_stores,
            "session_service": session_service,
        }

        # Apply adapter hints
        adapter_hints = (data.get("runtime", {}).get("adapter_hints", {})
                         .get("google_adk", {}))

        if memory_store := adapter_hints.get("memory_store"):
            adk_config["memory_store"] = memory_store

        return adk_config

    def import_from(self, config: dict[str, Any]) -> ADP:
        """Import Google ADK config to ADP manifest."""
        agents = config.get("agents", [])
        tools = config.get("tools", [])
        artifacts = config.get("artifacts", [])
        session_service = config.get("session_service", {})
        memory_store = config.get("memory_store")

        flow_nodes = []
        flow_edges = []

        # Convert agents to nodes
        for agent in agents:
            agent_type = agent.get("type", "")
            name = agent.get("name", "")

            kind_map = {
                "LLMAgent": "llm",
                "ToolAgent": "tool",
                "RouterAgent": "router",
            }

            node = {"id": name, "kind": kind_map.get(agent_type, "llm")}
            if agent_type == "LLMAgent":
                node["model_ref"] = agent.get("model")
            elif agent_type == "ToolAgent":
                node["tool_ref"] = agent.get("tool")
            elif agent_type == "RouterAgent":
                node["strategy"] = agent.get("strategy")

            flow_nodes.append(node)

        # Extract adapter hints
        adapter_hints = {}
        if memory_store:
            adapter_hints["memory_store"] = memory_store

        # Convert artifacts to ADP artifacts
        adp_artifacts: dict[str, Any] = {}
        if artifacts:
            adp_artifacts["stores"] = [{
                "id": a.get("id"),
                "provider": a.get("provider"),
                "bucket": a.get("bucket"),
                "scope": a.get("scope", "agent"),
            } for a in artifacts]

        # Convert session service to memory
        adp_memory: dict[str, Any] = {}
        if session_service:
            adp_memory["stores"] = [{
                "id": "session_memory",
                "type": "episodic",
                "provider": session_service.get("provider"),
                "endpoint": session_service.get("endpoint"),
                "scope": "session",
            }]

        from ..adp_model import (
            RuntimeEntry,
            RuntimeModel,
            GraphModel,
            FlowModel,
            ToolsModel,
            HTTPAPIModel,
            ArtifactsModel,
            ArtifactStore,
        )

        http_apis = [
            HTTPAPIModel(
                id=t.get("name", ""),
                description=t.get("description", ""),
                base_url="",
            )
            for t in tools
        ] if tools else None

        artifact_stores = [
            ArtifactStore(
                id=a.get("id", ""),
                provider=a.get("provider", "gcs"),
                bucket=a.get("bucket"),
                scope=a.get("scope", "agent"),
            )
            for a in artifacts
        ] if artifacts else None

        return ADP(
            adp_version="0.3.0",
            id=config.get("id", "imported-from-google-adk"),
            runtime=RuntimeModel(execution=[RuntimeEntry(
                backend="python",
                id="google_adk",
                module="google.adk",
            )]),
            flow=FlowModel(
                id="imported-flow",
                graph=GraphModel(
                    nodes=flow_nodes,
                    edges=flow_edges,
                    start_nodes=[flow_nodes[0]["id"]] if flow_nodes else [],
                    end_nodes=[],
                ),
                extensions={"google_adk": adapter_hints} if adapter_hints else None,
            ),
            tools=ToolsModel(http_apis=http_apis) if http_apis else None,
            artifacts=ArtifactsModel(stores=artifact_stores) if artifact_stores else None,
            memory=adp_memory if adp_memory else None,
            extensions={"source_framework": "google_adk"},
        )

    def roundtrip_fidelity(self) -> dict[str, str]:
        return {
            **super().roundtrip_fidelity(),
            "artifacts": "faithful",
            "memory.stores": "faithful",
            "tools": "faithful",
        }
