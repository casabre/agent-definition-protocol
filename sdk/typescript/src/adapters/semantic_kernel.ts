/**
 * Semantic Kernel adapter for ADP v0.3.0.
 *
 * Converts between ADP manifests and Semantic Kernel configurations.
 */

import { AdapterBase } from "./base";
import { AdapterRegistry } from "./registry";
import { ADP } from "../adp";

export class SemanticKernelAdapter extends AdapterBase {
  readonly frameworkId = "semantic_kernel";

  constructor() {
    super();
    AdapterRegistry.register(SemanticKernelAdapter);
  }

  export(manifest: ADP): Record<string, unknown> {
    const data = manifest as Record<string, unknown>;

    const flow = (data.flow as Record<string, unknown>) ?? {};
    const graph = (flow.graph as Record<string, unknown>) ?? {};
    const nodes = (graph.nodes as Array<Record<string, unknown>>) ?? [];
    const edges = (graph.edges as Array<Record<string, unknown>>) ?? [];
    const tools = (data.tools as Record<string, unknown>) ?? {};
    const runtime = (data.runtime as Record<string, unknown>) ?? {};

    // Map nodes to Semantic Kernel steps/agents
    const steps: Record<string, Record<string, unknown>> = {};

    for (const node of nodes) {
      const nid = (node.id as string) ?? "";
      const kind = (node.kind as string) ?? "";

      if (kind === "llm") {
        steps[nid] = {
          type: "LLMService",
          model: node.model_ref as string ?? "gpt-4o",
        };
      } else if (kind === "tool") {
        steps[nid] = {
          type: "Function",
          name: node.tool_ref as string ?? nid,
        };
      } else if (kind === "retriever") {
        steps[nid] = {
          type: "Retriever",
          memory: node.memory_ref as string,
        };
      } else {
        steps[nid] = { type: "Node" };
      }
    }

    // Build workflow from edges
    const workflow: Array<Record<string, unknown>> = [];
    for (const edge of edges) {
      workflow.push({
        from: edge.from as string,
        to: edge.to as string,
      });
    }

    // Map tools to Semantic Kernel plugins
    const plugins: Record<string, Record<string, unknown>> = {};
    const toolListKeys: string[] = ["mcp_servers", "http_apis", "sql_functions"];
    for (const toolListKey of toolListKeys) {
      const toolList = (tools[toolListKey] as Array<Record<string, unknown>>) ?? [];
      for (const tool of toolList) {
        plugins[tool.id as string] = {
          type: toolListKey.replace(/_/g, "").replace(/^(\w)/, (m) => m.toUpperCase()),
          description: (tool.description as string) ?? "",
          endpoint: tool.endpoint as string ?? tool.base_url as string ?? "",
        };
      }
    }

    // Map runtime to AI services
    const aiServices: Record<string, Record<string, unknown>> = {};
    const models = (runtime.models as Array<Record<string, unknown>>) ?? [];
    for (const model of models) {
      aiServices[model.id as string] = {
        provider: model.provider as string ?? "openai",
        model: model.model as string ?? "gpt-4o",
      };
    }

    // Build Semantic Kernel config
    const skConfig: Record<string, unknown> = {
      plugins: plugins,
      ai_services: aiServices,
      steps: steps,
      workflow: workflow,
    };

    // Apply adapter hints
    const adapterHints = (
      (runtime.adapter_hints as Record<string, unknown>)?.semantic_kernel as Record<string, unknown>
    ) ?? {};

    // Semantic Kernel doesn't have specific adapter hints in v0.3.0
    // but we include any provided
    if (Object.keys(adapterHints).length > 0) {
      skConfig.hints = adapterHints;
    }

    return skConfig;
  }

  importFrom(config: Record<string, unknown>): ADP {
    const plugins = (config.plugins as Record<string, Record<string, unknown>>) ?? {};
    const aiServices = (config.ai_services as Record<string, Record<string, unknown>>) ?? {};
    const steps = (config.steps as Record<string, Record<string, unknown>>) ?? {};
    const workflow = (config.workflow as Array<Record<string, unknown>>) ?? [];

    const flowNodes: Array<Record<string, unknown>> = [];
    const flowEdges: Array<Record<string, unknown>> = [];

    // Convert steps to nodes
    const kindMap: Record<string, string> = {
      LLMService: "llm",
      Function: "tool",
      Retriever: "retriever",
      Router: "router",
    };

    for (const [stepId, stepConfig] of Object.entries(steps)) {
      const stepType = (stepConfig.type as string) ?? "";
      const kind = kindMap[stepType] ?? "tool";
      const node: Record<string, unknown> = { id: stepId, kind };

      if (stepType === "LLMService") {
        node.model_ref = stepConfig.model as string;
      } else if (stepType === "Function") {
        node.tool_ref = stepConfig.name as string;
      } else if (stepType === "Retriever") {
        node.memory_ref = stepConfig.memory as string;
      }

      flowNodes.push(node);
    }

    // Convert workflow to edges
    for (const wf of workflow) {
      flowEdges.push({
        from: wf.from as string,
        to: wf.to as string,
      });
    }

    // Convert plugins to tools
    const httpApis: Array<Record<string, unknown>> = [];
    for (const [pluginId, pluginConfig] of Object.entries(plugins)) {
      const pluginType = (pluginConfig.type as string) ?? "";
      if (pluginType === "HttpApis") {
        httpApis.push({
          id: pluginId,
          description: (pluginConfig.description as string) ?? "",
          base_url: (pluginConfig.endpoint as string) ?? "",
        });
      }
    }

    // Convert AI services to runtime models
    const runtimeModels: Array<Record<string, unknown>> = [];
    for (const [serviceId, serviceConfig] of Object.entries(aiServices)) {
      runtimeModels.push({
        id: serviceId,
        provider: (serviceConfig.provider as string) ?? "openai",
        model: (serviceConfig.model as string) ?? "gpt-4o",
      });
    }

    const startNodes = flowNodes.length > 0 ? [flowNodes[0].id as string] : [];

    return {
      adp_version: "0.3.0",
      id: (config.id as string) ?? "imported-from-semantic-kernel",
      runtime: {
        execution: [
          {
            backend: "python",
            id: "semantic_kernel",
            module: "semantic_kernel",
          },
        ],
        models: runtimeModels,
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
      tools: httpApis.length > 0 ? { http_apis: httpApis } : undefined,
      evaluation: { suites: [] },
      extensions: {
        source_framework: "semantic_kernel",
      },
    } as unknown as ADP;
  }

  roundtripFidelity(): Record<string, string> {
    return {
      ...super.roundtripFidelity(),
      "runtime.models": "faithful",
      "tools": "faithful",
    };
  }
}
