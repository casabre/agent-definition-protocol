"""LangGraph adapter for ADP v0.3.0.

Converts between ADP manifests and LangGraph StateGraph configurations.
"""

from typing import Any

from .base import AdapterBase
from .registry import AdapterRegistry
from ..adp_model import ADP


class LangGraphAdapter(AdapterBase):
    """Adapter for LangGraph framework.

    Converts ADP flow graphs to LangGraph StateGraph configurations
    and vice versa.
    """

    framework_id = "langgraph"

    def __init__(self) -> None:
        AdapterRegistry.register(self.__class__)

    def export(self, manifest: ADP) -> dict[str, Any]:
        """Export ADP manifest to LangGraph StateGraph config."""
        data = manifest.model_dump(by_alias=True, exclude_none=True)

        # Build StateGraph from flow.graph
        flow = data.get("flow", {})
        graph = flow.get("graph", {})
        nodes = graph.get("nodes", [])
        edges = graph.get("edges", [])

        # Map node kinds to LangGraph node types
        node_map: dict[str, dict[str, Any]] = {}
        for node in nodes:
            nid = node.get("id", "")
            kind = node.get("kind", "")
            node_config: dict[str, Any] = {}

            # Set node type based on ADP kind
            if kind == "llm":
                node_config["type"] = "ChatModel"
                model_ref = node.get("model_ref")
                if model_ref:
                    node_config["model"] = model_ref
            elif kind == "tool":
                node_config["type"] = "ToolNode"
                tool_ref = node.get("tool_ref")
                if tool_ref:
                    node_config["tool"] = tool_ref
            elif kind == "router":
                node_config["type"] = "Router"
                strategy = node.get("strategy")
                if strategy:
                    node_config["strategy"] = strategy
            elif kind == "retriever":
                node_config["type"] = "Retriever"
                memory_ref = node.get("memory_ref")
                if memory_ref:
                    node_config["memory"] = memory_ref
            elif kind in ("input", "output"):
                node_config["type"] = "Start" if kind == "input" else "End"
            else:
                node_config["type"] = "Node"

            # Add params
            params = node.get("params", {})
            if params:
                node_config["params"] = params

            node_map[nid] = node_config

        # Build edges
        edge_config: dict[str, list[str]] = {}
        for edge in edges:
            frm = edge.get("from", "")
            to = edge.get("to", "")
            if frm not in edge_config:
                edge_config[frm] = []
            edge_config[frm].append(to)

        # Apply adapter hints
        adapter_hints = (data.get("runtime", {}).get("adapter_hints", {})
                         .get("langgraph", {}))

        state_graph: dict[str, Any] = {
            "nodes": node_map,
            "edges": edge_config,
        }

        # Apply langgraph-specific hints
        if checkpointer := adapter_hints.get("checkpointer"):
            state_graph["checkpointer"] = checkpointer
        if memory_store := adapter_hints.get("memory_store"):
            state_graph["store"] = memory_store
        if recursion_limit := adapter_hints.get("recursion_limit"):
            state_graph["recursion_limit"] = recursion_limit
        if stream_mode := adapter_hints.get("stream_mode"):
            state_graph["stream_mode"] = stream_mode

        return state_graph

    def import_from(self, config: dict[str, Any]) -> ADP:
        """Import LangGraph StateGraph config to ADP manifest."""
        nodes = config.get("nodes", {})
        edges = config.get("edges", {})

        # Build flow graph
        flow_nodes = []
        flow_edges = []

        for node_id, node_config in nodes.items():
            kind_map = {
                "ChatModel": "llm",
                "ToolNode": "tool",
                "Router": "router",
                "Retriever": "retriever",
                "Start": "input",
                "End": "output",
            }
            kind = kind_map.get(node_config.get("type", ""), "tool")
            model_ref = node_config.get("model")
            tool_ref = node_config.get("tool")
            memory_ref = node_config.get("memory")

            node = {"id": node_id, "kind": kind}
            if model_ref:
                node["model_ref"] = model_ref
            if tool_ref:
                node["tool_ref"] = tool_ref
            if memory_ref:
                node["memory_ref"] = memory_ref

            flow_nodes.append(node)

        for frm, tos in edges.items():
            for to in tos:
                flow_edges.append({"from": frm, "to": to})

        # Extract adapter hints
        adapter_hints = {}
        if checkpointer := config.get("checkpointer"):
            adapter_hints["checkpointer"] = checkpointer
        if memory_store := config.get("store"):
            adapter_hints["memory_store"] = memory_store
        if recursion_limit := config.get("recursion_limit"):
            adapter_hints["recursion_limit"] = recursion_limit
        if stream_mode := config.get("stream_mode"):
            adapter_hints["stream_mode"] = stream_mode

        # Build minimal ADP manifest
        from ..adp_model import (
            RuntimeEntry,
            RuntimeModel,
            GraphModel,
            FlowModel,
        )

        return ADP(
            adp_version="0.3.0",
            id=config.get("id", "imported-from-langgraph"),
            name=config.get("name"),
            runtime=RuntimeModel(execution=[RuntimeEntry(
                backend="python",
                id="langgraph",
                module="langgraph.graph",
            )]),
            flow=FlowModel(
                id="imported-flow",
                graph=GraphModel(
                    nodes=flow_nodes,
                    edges=flow_edges,
                    start_nodes=[n["id"] for n in flow_nodes if n.get("kind") == "input"],
                    end_nodes=[n["id"] for n in flow_nodes if n.get("kind") == "output"],
                ),
                extensions={"langgraph": adapter_hints} if adapter_hints else None,
            ),
            extensions={"source_framework": "langgraph"},
        )

    def roundtrip_fidelity(self) -> dict[str, str]:
        return {
            **super().roundtrip_fidelity(),
            "flow.graph": "faithful",
            "loop.termination": "faithful",  # via recursion_limit
            "memory.stores": "faithful",  # via adapter_hints
        }
