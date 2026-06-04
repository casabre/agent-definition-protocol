/**
 * OpenAI Agents SDK adapter for ADP v0.3.0.
 *
 * Converts between ADP manifests and OpenAI Agents SDK configurations.
 */

import { AdapterBase } from "./base";
import { AdapterRegistry } from "./registry";
import { ADP } from "../adp";

export class OpenAIAgentsAdapter extends AdapterBase {
  readonly frameworkId = "openai_agents";

  constructor() {
    super();
    AdapterRegistry.register(OpenAIAgentsAdapter);
  }

  export(manifest: ADP): Record<string, unknown> {
    const data = manifest as Record<string, unknown>;

    const flow = (data.flow as Record<string, unknown>) ?? {};
    const graph = (flow.graph as Record<string, unknown>) ?? {};
    const nodes = (graph.nodes as Array<Record<string, unknown>>) ?? [];
    const edges = (graph.edges as Array<Record<string, unknown>>) ?? [];
    const tools = (data.tools as Record<string, unknown>) ?? {};
    const guardrails = (data.guardrails as Record<string, unknown>) ?? {};
    const observability = (data.observability as Record<string, unknown>) ?? {};

    // Map nodes to OpenAI agents
    const agents: Record<string, Record<string, unknown>> = {};
    const handoffs: Array<Record<string, unknown>> = [];

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
          tools: [node.tool_ref as string ?? ""],
        };
      }
    }

    // Build handoffs from edges
    for (const edge of edges) {
      const frm = (edge.from as string) ?? "";
      const to = (edge.to as string) ?? "";
      const condition = edge.condition as string | undefined;
      handoffs.push({
        from: frm,
        to: to,
        condition: condition,
      });
    }

    // Map tools
    const oaiTools: Array<Record<string, unknown>> = [];
    const toolListKeys: string[] = ["mcp_servers", "http_apis", "sql_functions"];
    for (const toolListKey of toolListKeys) {
      const toolList = (tools[toolListKey] as Array<Record<string, unknown>>) ?? [];
      for (const tool of toolList) {
        oaiTools.push({
          name: tool.id as string,
          description: (tool.description as string) ?? "",
        });
      }
    }

    // Map guardrails interrupts to OpenAI guardrails
    const oaiGuardrails: Record<string, unknown> = {};
    const interrupts = (guardrails.interrupts as Array<Record<string, unknown>>) ?? [];
    for (const interrupt of interrupts) {
      if (interrupt && typeof interrupt === "object") {
        const trigger = (interrupt.trigger as string) ?? "";
        if (trigger === "cost_threshold") {
          oaiGuardrails.cost_limit = {
            threshold: interrupt.threshold_usd as number ?? 10.0,
            action: interrupt.mode as string ?? "block",
          };
        }
      }
    }

    // Map observability
    const oaiObservability: Record<string, unknown> = {};
    if (observability && typeof observability === "object") {
      const tracing = (observability.tracing as Record<string, unknown>) ?? {};
      if (typeof tracing === "object") {
        oaiObservability.tracing = {
          backend: tracing.backend as string ?? "stdout",
          events: tracing.trace_events as string[] ?? [],
        };
      }
    }

    // Build OpenAI Agents config
    const oaiConfig: Record<string, unknown> = {
      agents: Object.values(agents),
      handoffs: handoffs,
      tools: oaiTools,
      guardrails: oaiGuardrails,
      observability: oaiObservability,
    };

    return oaiConfig;
  }

  importFrom(config: Record<string, unknown>): ADP {
    const agents = (config.agents as Array<Record<string, unknown>>) ?? [];
    const handoffs = (config.handoffs as Array<Record<string, unknown>>) ?? [];
    const tools = (config.tools as Array<Record<string, unknown>>) ?? [];
    const guardrails = (config.guardrails as Record<string, unknown>) ?? {};
    const observability = (config.observability as Record<string, unknown>) ?? {};

    const flowNodes: Array<Record<string, unknown>> = [];
    const flowEdges: Array<Record<string, unknown>> = [];

    // Convert agents to nodes
    for (const agent of agents) {
      const name = (agent.name as string) ?? "";
      const model = (agent.model as string) ?? "";
      const agentTools = (agent.tools as string[]) ?? [];

      flowNodes.push({
        id: name,
        kind: "llm",
        model_ref: model,
      });

      // Add tool nodes
      for (const toolName of agentTools) {
        flowNodes.push({
          id: `${name}_tool_${toolName}`,
          kind: "tool",
          tool_ref: toolName,
        });
        flowEdges.push({
          from: name,
          to: `${name}_tool_${toolName}`,
        });
      }
    }

    // Convert handoffs to edges
    for (const handoff of handoffs) {
      flowEdges.push({
        from: handoff.from as string,
        to: handoff.to as string,
        condition: handoff.condition as string,
      });
    }

    // Convert observability
    const adpObservability: Record<string, unknown> = {};
    if (observability.tracing !== undefined) {
      const tracing = observability.tracing as Record<string, unknown>;
      adpObservability.tracing = {
        backend: tracing.backend,
        trace_events: tracing.events as string[],
      };
    }

    const startNodes = flowNodes.length > 0 ? [flowNodes[0].id as string] : [];

    return {
      adp_version: "0.3.0",
      id: (config.id as string) ?? "imported-from-openai-agents",
      runtime: {
        execution: [
          {
            backend: "python",
            id: "openai_agents",
            module: "openai.agents",
          },
        ],
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
      tools: tools.length > 0 ? { http_apis: tools.map((t) => ({
        id: t.name as string,
        description: (t.description as string) ?? "",
        base_url: "",
      })) } : undefined,
      observability: adpObservability && Object.keys(adpObservability).length > 0 ? adpObservability : undefined,
      evaluation: { suites: [] },
      extensions: {
        source_framework: "openai_agents",
      },
    } as unknown as ADP;
  }

  roundtripFidelity(): Record<string, string> {
    return {
      ...super.roundtripFidelity(),
      "guardrails.interrupts": "faithful",
      "observability": "faithful",
    };
  }
}
