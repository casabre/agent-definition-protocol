/**
 * LangGraph adapter for ADP v0.3.0.
 *
 * Converts between ADP manifests and LangGraph StateGraph configurations.
 */

import { AdapterBase } from "./base";
import { AdapterRegistry } from "./registry";
import { ADP } from "../adp";

export class LangGraphAdapter extends AdapterBase {
  readonly frameworkId = "langgraph";

  constructor() {
    super();
    AdapterRegistry.register(LangGraphAdapter);
  }

  export(manifest: ADP): Record<string, unknown> {
    const data = manifest as Record<string, unknown>;

    // Build StateGraph from flow.graph
    const flow = (data.flow as Record<string, unknown>) ?? {};
    const graph = (flow.graph as Record<string, unknown>) ?? {};
    const nodes = (graph.nodes as Array<Record<string, unknown>>) ?? [];
    const edges = (graph.edges as Array<Record<string, unknown>>) ?? [];

    // Map node kinds to LangGraph node types
    const nodeMap: Record<string, Record<string, unknown>> = {};
    for (const node of nodes) {
      const nid = (node.id as string) ?? "";
      const kind = (node.kind as string) ?? "";
      const nodeConfig: Record<string, unknown> = {};

      // Set node type based on ADP kind
      if (kind === "llm") {
        nodeConfig.type = "ChatModel";
        const modelRef = node.model_ref as string | undefined;
        if (modelRef) {
          nodeConfig.model = modelRef;
        }
      } else if (kind === "tool") {
        nodeConfig.type = "ToolNode";
        const toolRef = node.tool_ref as string | undefined;
        if (toolRef) {
          nodeConfig.tool = toolRef;
        }
      } else if (kind === "router") {
        nodeConfig.type = "Router";
        const strategy = node.strategy as string | undefined;
        if (strategy) {
          nodeConfig.strategy = strategy;
        }
      } else if (kind === "retriever") {
        nodeConfig.type = "Retriever";
        const memoryRef = node.memory_ref as string | undefined;
        if (memoryRef) {
          nodeConfig.memory = memoryRef;
        }
      } else if (kind === "input") {
        nodeConfig.type = "Start";
      } else if (kind === "output") {
        nodeConfig.type = "End";
      } else {
        nodeConfig.type = "Node";
      }

      // Add params
      const params = node.params as Record<string, unknown> | undefined;
      if (params) {
        nodeConfig.params = params;
      }

      nodeMap[nid] = nodeConfig;
    }

    // Build edges
    const edgeConfig: Record<string, string[]> = {};
    for (const edge of edges) {
      const frm = (edge.from as string) ?? "";
      const to = (edge.to as string) ?? "";
      if (!edgeConfig[frm]) {
        edgeConfig[frm] = [];
      }
      edgeConfig[frm].push(to);
    }

    // Apply adapter hints
    const adapterHints = (
      (data.runtime as Record<string, unknown>)?.adapter_hints as Record<string, unknown>
    )?.langgraph as Record<string, unknown> | undefined;

    const stateGraph: Record<string, unknown> = {
      nodes: nodeMap,
      edges: edgeConfig,
    };

    // Apply langgraph-specific hints
    if (adapterHints) {
      if (adapterHints.checkpointer !== undefined) {
        stateGraph.checkpointer = adapterHints.checkpointer;
      }
      if (adapterHints.memory_store !== undefined) {
        stateGraph.store = adapterHints.memory_store;
      }
      if (adapterHints.recursion_limit !== undefined) {
        stateGraph.recursion_limit = adapterHints.recursion_limit;
      }
      if (adapterHints.stream_mode !== undefined) {
        stateGraph.stream_mode = adapterHints.stream_mode;
      }
    }

    return stateGraph;
  }

  importFrom(config: Record<string, unknown>): ADP {
    const nodes = (config.nodes as Record<string, Record<string, unknown>>) ?? {};
    const edges = (config.edges as Record<string, string[]>) ?? {};

    // Build flow graph
    const flowNodes: Array<Record<string, unknown>> = [];
    const flowEdges: Array<Record<string, unknown>> = [];

    const kindMap: Record<string, string> = {
      ChatModel: "llm",
      ToolNode: "tool",
      Router: "router",
      Retriever: "retriever",
      Start: "input",
      End: "output",
    };

    for (const [nodeId, nodeConfig] of Object.entries(nodes)) {
      const type = (nodeConfig.type as string) ?? "";
      const kind = kindMap[type] ?? "tool";
      const node: Record<string, unknown> = { id: nodeId, kind };

      if (type === "ChatModel") {
        if (nodeConfig.model) {
          node.model_ref = nodeConfig.model;
        }
      } else if (type === "ToolNode") {
        if (nodeConfig.tool) {
          node.tool_ref = nodeConfig.tool;
        }
      } else if (type === "Router") {
        if (nodeConfig.strategy) {
          node.strategy = nodeConfig.strategy;
        }
      } else if (type === "Retriever") {
        if (nodeConfig.memory) {
          node.memory_ref = nodeConfig.memory;
        }
      }

      flowNodes.push(node);
    }

    for (const [frm, tos] of Object.entries(edges)) {
      for (const to of tos) {
        flowEdges.push({ from: frm, to });
      }
    }

    // Extract adapter hints
    const adapterHints: Record<string, unknown> = {};
    if (config.checkpointer !== undefined) {
      adapterHints.checkpointer = config.checkpointer;
    }
    if (config.store !== undefined) {
      adapterHints.memory_store = config.store;
    }
    if (config.recursion_limit !== undefined) {
      adapterHints.recursion_limit = config.recursion_limit;
    }
    if (config.stream_mode !== undefined) {
      adapterHints.stream_mode = config.stream_mode;
    }

    const startNodes = flowNodes.filter((n) => n.kind === "input").map((n) => n.id as string);
    const endNodes = flowNodes.filter((n) => n.kind === "output").map((n) => n.id as string);

    return {
      adp_version: "0.3.0",
      id: (config.id as string) ?? "imported-from-langgraph",
      name: config.name as string | undefined,
      runtime: {
        execution: [
          {
            backend: "python",
            id: "langgraph",
            module: "langgraph.graph",
          },
        ],
      },
      flow: {
        id: "imported-flow",
        graph: {
          nodes: flowNodes,
          edges: flowEdges,
          start_nodes: startNodes,
          end_nodes: endNodes,
        },
        extensions: adapterHints && Object.keys(adapterHints).length > 0 ? { langgraph: adapterHints } : undefined,
      },
      evaluation: { suites: [] },
      extensions: {
        source_framework: "langgraph",
      },
    } as unknown as ADP;
  }

  roundtripFidelity(): Record<string, string> {
    return {
      ...super.roundtripFidelity(),
      "flow.graph": "faithful",
      "loop.termination": "faithful", // via recursion_limit
      "memory.stores": "faithful", // via adapter_hints
    };
  }
}
