/**
 * LlamaIndex adapter for ADP v0.3.0.
 *
 * Converts between ADP manifests and LlamaIndex configurations.
 */

import { AdapterBase } from "./base";
import { AdapterRegistry } from "./registry";
import { ADP } from "../adp";

export class LlamaIndexAdapter extends AdapterBase {
  readonly frameworkId = "llamaindex";

  constructor() {
    super();
    AdapterRegistry.register(LlamaIndexAdapter);
  }

  export(manifest: ADP): Record<string, unknown> {
    const data = manifest as Record<string, unknown>;

    const tools = (data.tools as Record<string, unknown>) ?? {};
    const memory = (data.memory as Record<string, unknown>) ?? {};

    // Map tools to LlamaIndex tools
    const indexTools: Array<Record<string, unknown>> = [];

    const toolListKeys: string[] = ["mcp_servers", "http_apis", "sql_functions"];
    for (const toolListKey of toolListKeys) {
      const toolList = (tools[toolListKey] as Array<Record<string, unknown>>) ?? [];
      for (const tool of toolList) {
        indexTools.push({
          name: tool.id as string,
          description: (tool.description as string) ?? "",
          function: tool.id as string,
        });
      }
    }

    // Map memory stores to LlamaIndex memory
    const memoryConfig: Record<string, unknown> = {};
    if (memory && typeof memory === "object") {
      const stores = (memory.stores as Array<Record<string, unknown>>) ?? [];
      for (const store of stores) {
        const storeType = (store.type as string) ?? "semantic";
        if (storeType === "semantic") {
          memoryConfig.vector_store = {
            provider: store.provider,
            index: store.index,
          };
        } else if (storeType === "episodic") {
          memoryConfig.chat_memory = {
            provider: store.provider,
          };
        }
      }
    }

    // Build QueryEngine configuration
    const queryEngine: Record<string, unknown> = {
      tools: indexTools,
      memory: memoryConfig,
    };

    // Apply adapter hints
    const adapterHints = (
      (data.runtime as Record<string, unknown>)?.adapter_hints as Record<string, unknown>
    )?.llamaindex as Record<string, unknown> | undefined;

    if (adapterHints?.embedder_config !== undefined) {
      queryEngine.embedder = adapterHints.embedder_config;
    }

    return queryEngine;
  }

  importFrom(config: Record<string, unknown>): ADP {
    const tools = (config.tools as Array<Record<string, unknown>>) ?? [];
    const memoryConfig = (config.memory as Record<string, unknown>) ?? {};
    const embedder = config.embedder as Record<string, unknown> | undefined;

    // Convert tools to ADP tools
    const adpTools: Array<Record<string, unknown>> = [];
    for (const tool of tools) {
      adpTools.push({
        id: tool.name as string,
        description: (tool.description as string) ?? "",
        base_url: (tool.base_url as string) ?? "",
      });
    }

    // Convert memory to ADP memory
    const adpMemory: Record<string, unknown> = {};
    if (memoryConfig.vector_store !== undefined) {
      const vs = memoryConfig.vector_store as Record<string, unknown>;
      adpMemory.stores = [{
        id: "vector_store",
        type: "semantic",
        provider: vs.provider,
        index: vs.index,
      }];
    }
    if (memoryConfig.chat_memory !== undefined) {
      const cm = memoryConfig.chat_memory as Record<string, unknown>;
      if (!adpMemory.stores) {
        adpMemory.stores = [];
      }
      (adpMemory.stores as Array<Record<string, unknown>>).push({
        id: "chat_memory",
        type: "episodic",
        provider: cm.provider,
      });
    }

    // Extract adapter hints
    const adapterHints: Record<string, unknown> = {};
    if (embedder) {
      adapterHints.embedder_config = embedder;
    }

    return {
      adp_version: "0.3.0",
      id: (config.id as string) ?? "imported-from-llamaindex",
      runtime: {
        execution: [
          {
            backend: "python",
            id: "llamaindex",
            module: "llama_index",
          },
        ],
      },
      flow: {
        id: "imported-flow",
        graph: {
          nodes: [
            {
              id: "query",
              kind: "llm",
            },
          ],
          edges: [],
          start_nodes: ["query"],
          end_nodes: ["query"],
        },
        extensions: adapterHints && Object.keys(adapterHints).length > 0 ? { llamaindex: adapterHints } : undefined,
      },
      tools: adpTools.length > 0 ? { http_apis: adpTools } : undefined,
      memory: adpMemory && Object.keys(adpMemory).length > 0 ? adpMemory : undefined,
      evaluation: { suites: [] },
      extensions: {
        source_framework: "llamaindex",
      },
    } as unknown as ADP;
  }

  roundtripFidelity(): Record<string, string> {
    return {
      ...super.roundtripFidelity(),
      "memory.stores": "faithful",
      "tools": "faithful",
    };
  }
}
