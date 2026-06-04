/**
 * AutoGen adapter for ADP v0.3.0.
 *
 * Converts between ADP manifests and AutoGen configurations.
 */

import { AdapterBase } from "./base";
import { AdapterRegistry } from "./registry";
import { ADP } from "../adp";

export class AutoGenAdapter extends AdapterBase {
  readonly frameworkId = "autogen";

  constructor() {
    super();
    AdapterRegistry.register(AutoGenAdapter);
  }

  export(manifest: ADP): Record<string, unknown> {
    const data = manifest as Record<string, unknown>;

    const flow = (data.flow as Record<string, unknown>) ?? {};
    const graph = (flow.graph as Record<string, unknown>) ?? {};
    const nodes = (graph.nodes as Array<Record<string, unknown>>) ?? [];

    // Map ADP nodes to AutoGen agents/tools
    const agents: Record<string, Record<string, unknown>> = {};
    const toolsList: Array<Record<string, unknown>> = [];

    for (const node of nodes) {
      const nid = (node.id as string) ?? "";
      const kind = (node.kind as string) ?? "";

      if (kind === "llm") {
        agents[nid] = {
          type: "AssistantAgent",
          model: node.model_ref as string ?? "gpt-4",
        };
      } else if (kind === "tool") {
        const toolRef = node.tool_ref as string ?? "";
        toolsList.push({
          name: nid,
          function: toolRef,
        });
        agents[nid] = {
          type: "ToolAgent",
          name: nid,
        };
      } else if (kind === "router") {
        agents[nid] = {
          type: "RouterAgent",
          strategy: node.strategy as string ?? "round_robin",
        };
      }
    }

    // Build group chat from loop nodes
    const loopPolicy = (flow.loop_policy as Record<string, unknown>) ?? {};
    const groupChat: Record<string, unknown> = {
      agents: Object.values(agents),
      tools: toolsList,
    };

    if (loopPolicy.default_max_iterations !== undefined) {
      groupChat.max_turns = loopPolicy.default_max_iterations;
    }

    // Apply adapter hints
    const adapterHints = (
      (data.runtime as Record<string, unknown>)?.adapter_hints as Record<string, unknown>
    )?.autogen as Record<string, unknown> | undefined;

    if (adapterHints) {
      if (adapterHints.human_input_mode !== undefined) {
        groupChat.human_input_mode = adapterHints.human_input_mode;
      }
      if (adapterHints.max_turns !== undefined) {
        groupChat.max_turns = adapterHints.max_turns;
      }
    }

    return groupChat;
  }

  importFrom(config: Record<string, unknown>): ADP {
    const agents = (config.agents as Array<Record<string, unknown>>) ?? [];
    const tools = (config.tools as Array<Record<string, unknown>>) ?? [];
    const maxTurns = config.max_turns as number | undefined;

    const flowNodes: Array<Record<string, unknown>> = [];
    const flowEdges: Array<Record<string, unknown>> = [];

    // Convert agents to nodes
    for (const agent of agents) {
      const agentType = (agent.type as string) ?? "";
      const name = (agent.name as string) ?? (agent.model as string) ?? "agent";

      if (agentType === "AssistantAgent") {
        flowNodes.push({
          id: name,
          kind: "llm",
          model_ref: agent.model as string | undefined,
        });
      } else if (agentType === "ToolAgent") {
        flowNodes.push({
          id: name,
          kind: "tool",
        });
      } else {
        flowNodes.push({
          id: name,
          kind: "router",
        });
      }
    }

    // Connect tools to agents
    for (const tool of tools) {
      const toolName = (tool.name as string) ?? "";
      const functionName = (tool.function as string) ?? "";
      flowNodes.push({
        id: `tool_${toolName}`,
        kind: "tool",
        tool_ref: functionName,
      });
      // Connect last agent to tool
      if (flowNodes.length > 0) {
        flowEdges.push({
          from: flowNodes[flowNodes.length - 1].id,
          to: `tool_${toolName}`,
        });
      }
    }

    // Extract adapter hints
    const adapterHints: Record<string, unknown> = {};
    if (config.human_input_mode !== undefined) {
      adapterHints.human_input_mode = config.human_input_mode;
    }
    if (maxTurns !== undefined) {
      adapterHints.max_turns = maxTurns;
    }

    const startNodes = flowNodes.length > 0 ? [flowNodes[0].id as string] : [];

    return {
      adp_version: "0.3.0",
      id: (config.id as string) ?? "imported-from-autogen",
      runtime: {
        execution: [
          {
            backend: "python",
            id: "autogen",
            module: "autogen",
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
        loop_policy: maxTurns !== undefined ? { default_max_iterations: maxTurns } : undefined,
        extensions: adapterHints && Object.keys(adapterHints).length > 0 ? { autogen: adapterHints } : undefined,
      },
      evaluation: { suites: [] },
      extensions: {
        source_framework: "autogen",
      },
    } as unknown as ADP;
  }

  roundtripFidelity(): Record<string, string> {
    return {
      ...super.roundtripFidelity(),
      "loop.termination": "faithful", // via max_turns
    };
  }
}
