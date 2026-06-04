/**
 * Google ADK adapter for ADP v0.3.0.
 *
 * Converts between ADP manifests and Google Agent Development Kit configurations.
 */

import { AdapterBase } from "./base";
import { AdapterRegistry } from "./registry";
import { ADP } from "../adp";

export class GoogleADKAdapter extends AdapterBase {
  readonly frameworkId = "google_adk";

  constructor() {
    super();
    AdapterRegistry.register(GoogleADKAdapter);
  }

  export(manifest: ADP): Record<string, unknown> {
    const data = manifest as Record<string, unknown>;

    const flow = (data.flow as Record<string, unknown>) ?? {};
    const graph = (flow.graph as Record<string, unknown>) ?? {};
    const nodes = (graph.nodes as Array<Record<string, unknown>>) ?? [];
    const edges = (graph.edges as Array<Record<string, unknown>>) ?? [];
    const tools = (data.tools as Record<string, unknown>) ?? {};
    const artifacts = (data.artifacts as Record<string, unknown>) ?? {};
    const memory = (data.memory as Record<string, unknown>) ?? {};

    // Map nodes to ADK agents
    const agents: Record<string, Record<string, unknown>> = {};

    for (const node of nodes) {
      const nid = (node.id as string) ?? "";
      const kind = (node.kind as string) ?? "";

      const agent: Record<string, unknown> = { name: nid };

      if (kind === "llm") {
        agent.type = "LLMAgent";
        agent.model = node.model_ref as string ?? "gemini-1.5-pro";
      } else if (kind === "tool") {
        agent.type = "ToolAgent";
        agent.tool = node.tool_ref as string;
      } else if (kind === "router") {
        agent.type = "RouterAgent";
        agent.strategy = node.strategy as string;
      }

      agents[nid] = agent;
    }

    // Map tools
    const adkTools: Array<Record<string, unknown>> = [];
    const toolListKeys: string[] = ["mcp_servers", "http_apis", "sql_functions"];
    for (const toolListKey of toolListKeys) {
      const toolList = (tools[toolListKey] as Array<Record<string, unknown>>) ?? [];
      for (const tool of toolList) {
        adkTools.push({
          name: tool.id as string,
          description: (tool.description as string) ?? "",
          function: tool.id as string,
        });
      }
    }

    // Map artifacts
    const artifactStores: Array<Record<string, unknown>> = [];
    if (artifacts && typeof artifacts === "object") {
      const stores = (artifacts.stores as Array<Record<string, unknown>>) ?? [];
      for (const store of stores) {
        artifactStores.push({
          id: store.id as string,
          provider: store.provider,
          bucket: store.bucket,
          scope: store.scope,
        });
      }
    }

    // Map memory to session service
    const sessionService: Record<string, unknown> = {};
    if (memory && typeof memory === "object") {
      const stores = (memory.stores as Array<Record<string, unknown>>) ?? [];
      for (const store of stores) {
        if (store.scope === "session") {
          sessionService.provider = store.provider;
          sessionService.endpoint = store.endpoint;
        }
      }
    }

    // Build ADK configuration
    const adkConfig: Record<string, unknown> = {
      agents: Object.values(agents),
      tools: adkTools,
      artifacts: artifactStores,
      session_service: sessionService,
    };

    // Apply adapter hints
    const adapterHints = (
      (data.runtime as Record<string, unknown>)?.adapter_hints as Record<string, unknown>
    )?.google_adk as Record<string, unknown> | undefined;

    if (adapterHints?.memory_store !== undefined) {
      adkConfig.memory_store = adapterHints.memory_store;
    }

    return adkConfig;
  }

  importFrom(config: Record<string, unknown>): ADP {
    const agents = (config.agents as Array<Record<string, unknown>>) ?? [];
    const tools = (config.tools as Array<Record<string, unknown>>) ?? [];
    const artifacts = (config.artifacts as Array<Record<string, unknown>>) ?? [];
    const sessionService = (config.session_service as Record<string, unknown>) ?? {};
    const memoryStore = config.memory_store as string | undefined;

    const flowNodes: Array<Record<string, unknown>> = [];
    const flowEdges: Array<Record<string, unknown>> = [];

    // Convert agents to nodes
    const kindMap: Record<string, string> = {
      LLMAgent: "llm",
      ToolAgent: "tool",
      RouterAgent: "router",
    };

    for (const agent of agents) {
      const agentType = (agent.type as string) ?? "";
      const name = (agent.name as string) ?? "";

      const kind = kindMap[agentType] ?? "llm";
      const node: Record<string, unknown> = { id: name, kind };

      if (agentType === "LLMAgent") {
        node.model_ref = agent.model as string;
      } else if (agentType === "ToolAgent") {
        node.tool_ref = agent.tool as string;
      } else if (agentType === "RouterAgent") {
        node.strategy = agent.strategy as string;
      }

      flowNodes.push(node);
    }

    // Extract adapter hints
    const adapterHints: Record<string, unknown> = {};
    if (memoryStore) {
      adapterHints.memory_store = memoryStore;
    }

    // Convert artifacts to ADP artifacts
    const adpArtifacts: Record<string, unknown> = {};
    if (artifacts.length > 0) {
      adpArtifacts.stores = artifacts.map((a) => ({
        id: a.id as string,
        provider: a.provider as string,
        bucket: a.bucket as string,
        scope: (a.scope as string) ?? "agent",
      }));
    }

    // Convert session service to memory
    const adpMemory: Record<string, unknown> = {};
    if (Object.keys(sessionService).length > 0) {
      adpMemory.stores = [{
        id: "session_memory",
        type: "episodic",
        provider: sessionService.provider,
        endpoint: sessionService.endpoint,
        scope: "session",
      }];
    }

    const startNodes = flowNodes.length > 0 ? [flowNodes[0].id as string] : [];

    return {
      adp_version: "0.3.0",
      id: (config.id as string) ?? "imported-from-google-adk",
      runtime: {
        execution: [
          {
            backend: "python",
            id: "google_adk",
            module: "google.adk",
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
        extensions: adapterHints && Object.keys(adapterHints).length > 0 ? { google_adk: adapterHints } : undefined,
      },
      tools: tools.length > 0 ? { http_apis: tools.map((t) => ({
        id: t.name as string,
        description: (t.description as string) ?? "",
        base_url: "",
      })) } : undefined,
      artifacts: adpArtifacts && Object.keys(adpArtifacts).length > 0 ? adpArtifacts : undefined,
      memory: adpMemory && Object.keys(adpMemory).length > 0 ? adpMemory : undefined,
      evaluation: { suites: [] },
      extensions: {
        source_framework: "google_adk",
      },
    } as unknown as ADP;
  }

  roundtripFidelity(): Record<string, string> {
    return {
      ...super.roundtripFidelity(),
      "artifacts": "faithful",
      "memory.stores": "faithful",
      "tools": "faithful",
    };
  }
}
