"""LlamaIndex adapter for ADP v0.3.0.

Converts between ADP manifests and LlamaIndex configurations.
"""

from typing import Any

from .base import AdapterBase
from .registry import AdapterRegistry
from ..adp_model import ADP


class LlamaIndexAdapter(AdapterBase):
    """Adapter for LlamaIndex framework.

    Converts ADP tools and memory to LlamaIndex QueryEngine configurations.
    """

    framework_id = "llamaindex"

    def __init__(self) -> None:
        AdapterRegistry.register(self.__class__)

    def export(self, manifest: ADP) -> dict[str, Any]:
        """Export ADP manifest to LlamaIndex config."""
        data = manifest.model_dump(by_alias=True, exclude_none=True)

        tools = data.get("tools", {})
        memory = data.get("memory", {})

        # Map tools to LlamaIndex tools
        index_tools: list[dict[str, Any]] = []

        for tool_list_key in ("mcp_servers", "http_apis", "sql_functions"):
            for tool in tools.get(tool_list_key, []):
                index_tools.append({
                    "name": tool.get("id"),
                    "description": tool.get("description", ""),
                    "function": tool.get("id"),
                })

        # Map memory stores to LlamaIndex memory
        memory_config: dict[str, Any] = {}
        if isinstance(memory, dict):
            stores = memory.get("stores", [])
            for store in stores:
                store_type = store.get("type", "semantic")
                if store_type == "semantic":
                    memory_config["vector_store"] = {
                        "provider": store.get("provider"),
                        "index": store.get("index"),
                    }
                elif store_type == "episodic":
                    memory_config["chat_memory"] = {
                        "provider": store.get("provider"),
                    }

        # Build QueryEngine configuration
        query_engine: dict[str, Any] = {
            "tools": index_tools,
            "memory": memory_config,
        }

        # Apply adapter hints
        adapter_hints = (data.get("runtime", {}).get("adapter_hints", {})
                         .get("llamaindex", {}))

        if embedder_config := adapter_hints.get("embedder_config"):
            query_engine["embedder"] = embedder_config

        return query_engine

    def import_from(self, config: dict[str, Any]) -> ADP:
        """Import LlamaIndex config to ADP manifest."""
        tools = config.get("tools", [])
        memory_config = config.get("memory", {})
        embedder = config.get("embedder")

        # Convert tools to ADP tools
        adp_tools = []
        for tool in tools:
            adp_tools.append({
                "id": tool.get("name", ""),
                "description": tool.get("description", ""),
                "base_url": tool.get("base_url", ""),
            })

        # Convert memory to ADP memory
        adp_memory: dict[str, Any] = {}
        if vector_store := memory_config.get("vector_store"):
            adp_memory["stores"] = [{
                "id": "vector_store",
                "type": "semantic",
                "provider": vector_store.get("provider"),
                "index": vector_store.get("index"),
            }]
        if chat_memory := memory_config.get("chat_memory"):
            if "stores" not in adp_memory:
                adp_memory["stores"] = []
            adp_memory["stores"].append({
                "id": "chat_memory",
                "type": "episodic",
                "provider": chat_memory.get("provider"),
            })

        # Extract adapter hints
        adapter_hints = {}
        if embedder:
            adapter_hints["embedder_config"] = embedder

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
                base_url=t.get("base_url", ""),
            )
            for t in tools
        ] if tools else None

        return ADP(
            adp_version="0.3.0",
            id=config.get("id", "imported-from-llamaindex"),
            runtime=RuntimeModel(execution=[RuntimeEntry(
                backend="python",
                id="llamaindex",
                module="llama_index",
            )]),
            flow=FlowModel(
                id="imported-flow",
                graph=GraphModel(
                    nodes=[{
                        "id": "query",
                        "kind": "llm",
                    }],
                    edges=[],
                    start_nodes=["query"],
                    end_nodes=["query"],
                ),
                extensions={"llamaindex": adapter_hints} if adapter_hints else None,
            ),
            tools=ToolsModel(http_apis=http_apis) if http_apis else None,
            memory=adp_memory if adp_memory else None,
            extensions={"source_framework": "llamaindex"},
        )

    def roundtrip_fidelity(self) -> dict[str, str]:
        return {
            **super().roundtrip_fidelity(),
            "memory.stores": "faithful",
            "tools": "faithful",
        }
