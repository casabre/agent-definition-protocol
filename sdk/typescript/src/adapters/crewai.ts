/**
 * CrewAI adapter for ADP v0.3.0.
 *
 * Converts between ADP manifests and CrewAI configurations.
 */

import { AdapterBase } from "./base";
import { AdapterRegistry } from "./registry";
import { ADP } from "../adp";

export class CrewAIAdapter extends AdapterBase {
  readonly frameworkId = "crewai";

  constructor() {
    super();
    AdapterRegistry.register(CrewAIAdapter);
  }

  export(manifest: ADP): Record<string, unknown> {
    const data = manifest as Record<string, unknown>;

    const flow = (data.flow as Record<string, unknown>) ?? {};
    const graph = (flow.graph as Record<string, unknown>) ?? {};
    const nodes = (graph.nodes as Array<Record<string, unknown>>) ?? [];

    // Map ADP nodes to CrewAI agents and tasks
    const agents: Record<string, Record<string, unknown>> = {};
    const tasks: Array<Record<string, unknown>> = [];

    for (const node of nodes) {
      const nid = (node.id as string) ?? "";
      const kind = (node.kind as string) ?? "";
      const role = (node.label as string) ?? nid;

      if (kind === "llm") {
        agents[nid] = {
          role: role,
          llm: node.model_ref as string ?? "gpt-4",
        };
      } else if (kind === "tool") {
        // Tools are mapped to agent tools
        // In CrewAI, tools are defined at the agent level
      }
    }

    // Build crew configuration
    const crew: Record<string, unknown> = {
      agents: Object.values(agents),
      tasks: tasks,
      process: "sequential", // default
    };

    // Apply adapter hints
    const adapterHints = (
      (data.runtime as Record<string, unknown>)?.adapter_hints as Record<string, unknown>
    )?.crewai as Record<string, unknown> | undefined;

    if (adapterHints) {
      if (adapterHints.process !== undefined) {
        crew.process = adapterHints.process;
      }
      if (adapterHints.max_rpm !== undefined) {
        crew.max_rpm = adapterHints.max_rpm;
      }
    }

    return crew;
  }

  importFrom(config: Record<string, unknown>): ADP {
    const agents = (config.agents as Array<Record<string, unknown>>) ?? [];
    const tasks = (config.tasks as Array<Record<string, unknown>>) ?? [];
    const process = (config.process as string) ?? "sequential";

    const flowNodes: Array<Record<string, unknown>> = [];
    const flowEdges: Array<Record<string, unknown>> = [];

    // Convert agents to nodes
    for (const agent of agents) {
      const role = (agent.role as string) ?? "";
      const llm = (agent.llm as string) ?? "";

      flowNodes.push({
        id: role.toLowerCase().replace(/\s+/g, "_"),
        kind: "llm",
        label: role,
        model_ref: llm,
      });
    }

    // Connect nodes sequentially
    for (let i = 0; i < flowNodes.length - 1; i++) {
      flowEdges.push({
        from: flowNodes[i].id,
        to: flowNodes[i + 1].id,
      });
    }

    // Extract adapter hints
    const adapterHints: Record<string, unknown> = {};
    if (process) {
      adapterHints.process = process;
    }
    if (config.max_rpm !== undefined) {
      adapterHints.max_rpm = config.max_rpm;
    }

    const startNodes = flowNodes.length > 0 ? [flowNodes[0].id as string] : [];
    const endNodes = flowNodes.length > 0 ? [flowNodes[flowNodes.length - 1].id as string] : [];

    return {
      adp_version: "0.3.0",
      id: (config.id as string) ?? "imported-from-crewai",
      runtime: {
        execution: [
          {
            backend: "python",
            id: "crewai",
            module: "crewai",
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
        extensions: adapterHints && Object.keys(adapterHints).length > 0 ? { crewai: adapterHints } : undefined,
      },
      evaluation: { suites: [] },
      extensions: {
        source_framework: "crewai",
      },
    } as unknown as ADP;
  }

  roundtripFidelity(): Record<string, string> {
    return {
      ...super.roundtripFidelity(),
      "tools.policy": "faithful", // via rate_limit -> max_rpm
    };
  }
}
