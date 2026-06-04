/**
 * Pydantic AI adapter for ADP v0.3.0.
 *
 * Converts between ADP manifests and Pydantic AI configurations.
 */

import { AdapterBase } from "./base";
import { AdapterRegistry } from "./registry";
import { ADP } from "../adp";

export class PydanticAIAdapter extends AdapterBase {
  readonly frameworkId = "pydantic_ai";

  constructor() {
    super();
    AdapterRegistry.register(PydanticAIAdapter);
  }

  export(manifest: ADP): Record<string, unknown> {
    const data = manifest as Record<string, unknown>;

    const flow = (data.flow as Record<string, unknown>) ?? {};
    const graph = (flow.graph as Record<string, unknown>) ?? {};
    const nodes = (graph.nodes as Array<Record<string, unknown>>) ?? [];
    const edges = (graph.edges as Array<Record<string, unknown>>) ?? [];
    const tools = (data.tools as Record<string, unknown>) ?? {};
    const runtime = (data.runtime as Record<string, unknown>) ?? {};

    // Map nodes to Pydantic AI agents
    const agents: Record<string, Record<string, unknown>> = {};

    for (const node of nodes) {
      const nid = (node.id as string) ?? "";
      const kind = (node.kind as string) ?? "";

      if (kind === "llm") {
        agents[nid] = {
          name: nid,
          model: node.model_ref as string ?? "gpt-4o",
        };
      } else if (kind === "tool") {
        agents[nid] = {
          name: nid,
          deps: { type: "Tool" },
        };
      }
    }

    // Build adjacency list from edges
    const adjacency: Record<string, string[]> = {};
    for (const edge of edges) {
      const frm = (edge.from as string) ?? "";
      const to = (edge.to as string) ?? "";
      if (!adjacency[frm]) {
        adjacency[frm] = [];
      }
      adjacency[frm].push(to);
    }

    // Map tools to Pydantic AI tool types
    const pydanticTools: Array<Record<string, unknown>> = [];
    const toolListKeys: string[] = ["mcp_servers", "http_apis", "sql_functions"];
    for (const toolListKey of toolListKeys) {
      const toolList = (tools[toolListKey] as Array<Record<string, unknown>>) ?? [];
      for (const tool of toolList) {
        pydanticTools.push({
          name: tool.id as string,
          description: (tool.description as string) ?? "",
          type: toolListKey.replace(/_/g, "").replace(/^(\w)/, (m) => m.toUpperCase()),
        });
      }
    }

    // Map runtime models
    const models = (runtime.models as Array<Record<string, unknown>>) ?? [];
    const pydanticModels: Record<string, Record<string, unknown>> = {};
    for (const model of models) {
      const modelId = (model.id as string) ?? "";
      pydanticModels[modelId] = {
        provider: model.provider as string ?? "openai",
        model: model.model as string ?? "gpt-4o",
      };
    }

    // Build Pydantic AI config
    const pydanticConfig: Record<string, unknown> = {
      agents: agents,
      adjacency: adjacency,
      tools: pydanticTools,
      models: pydanticModels,
    };

    // Apply adapter hints
    const adapterHints = (
      (runtime.adapter_hints as Record<string, unknown>)?.pydantic_ai as Record<string, unknown>
    ) ?? {};

    if (adapterHints.embedder_config !== undefined) {
      pydanticConfig.embedder = adapterHints.embedder_config;
    }

    return pydanticConfig;
  }

  importFrom(config: Record<string, unknown>): ADP {
    const agents = (config.agents as Record<string, Record<string, unknown>>) ?? {};
    const adjacency = (config.adjacency as Record<string, string[]>) ?? {};
    const tools = (config.tools as Array<Record<string, unknown>>) ?? [];
    const models = (config.models as Record<string, Record<string, unknown>>) ?? {};
    const embedder = config.embedder as Record<string, unknown> | undefined;

    const flowNodes: Array<Record<string, unknown>> = [];
    const flowEdges: Array<Record<string, unknown>> = [];

    // Convert agents to nodes
    for (const [agentId, agentConfig] of Object.entries(agents)) {
      const name = (agentConfig.name as string) ?? agentId;
      const model = (agentConfig.model as string) ?? "";
      const deps = (agentConfig.deps as Record<string, unknown>) ?? {};
      const depsType = (deps.type as string) ?? "";

      if (depsType === "Tool") {
        flowNodes.push({
          id: name,
          kind: "tool",
        });
      } else {
        flowNodes.push({
          id: name,
          kind: "llm",
          model_ref: model,
        });
      }
    }

    // Convert adjacency to edges
    for (const [frm, tos] of Object.entries(adjacency)) {
      for (const to of tos) {
        flowEdges.push({
          from: frm,
          to: to,
        });
      }
    }

    // Extract adapter hints
    const adapterHints: Record<string, unknown> = {};
    if (embedder) {
      adapterHints.embedder_config = embedder;
    }

    // Convert models to runtime models
    const runtimeModels: Array<Record<string, unknown>> = [];
    for (const [modelId, modelConfig] of Object.entries(models)) {
      runtimeModels.push({
        id: modelId,
        provider: (modelConfig.provider as string) ?? "openai",
        model: (modelConfig.model as string) ?? "gpt-4o",
      });
    }

    const startNodes = flowNodes.length > 0 ? [flowNodes[0].id as string] : [];

    return {
      adp_version: "0.3.0",
      id: (config.id as string) ?? "imported-from-pydantic-ai",
      runtime: {
        execution: [
          {
            backend: "python",
            id: "pydantic_ai",
            module: "pydantic_ai",
          },
        ],
        models: runtimeModels,
        adapter_hints: adapterHints && Object.keys(adapterHints).length > 0 ? { pydantic_ai: adapterHints } : undefined,
      },
      flow: {
        id: "imported-flow",
        graph: {
          nodes: flowNodes,
          edges: flowEdges,
          start_nodes: startNodes,
          end_nodes: [],
        },
      },
      tools: tools.length > 0 ? { http_apis: tools
        .filter((t) => t.type === "HttpApis")
        .map((t) => ({
          id: t.name as string,
          description: (t.description as string) ?? "",
          base_url: "",
        })) } : undefined,
      evaluation: { suites: [] },
      extensions: {
        source_framework: "pydantic_ai",
      },
    } as unknown as ADP;
  }

  roundtripFidelity(): Record<string, string> {
    return {
      ...super.roundtripFidelity(),
      "runtime.models": "faithful",
    };
  }
}
