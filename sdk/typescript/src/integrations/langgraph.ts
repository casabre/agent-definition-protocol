/**
 * ADP → LangGraph and LangGraph → ADP conversion utilities.
 *
 * Lazy import: module is importable whether or not @langchain/langgraph is installed.
 * Throws a clear error at call time when the package is absent.
 *
 * See spec/framework-interop.md §LangGraph Mapping for the full mapping guide.
 */
import { createRequire } from "module";
import type { ADP, FlowNode } from "../adp.js";

const _require = createRequire(import.meta.url);
let _lg: any = null;
try {
  _lg = _require("@langchain/langgraph");
} catch {
  _lg = null;
}

export interface ADPState {
  inputs: Record<string, unknown>;
  context: Record<string, unknown>;
  memory: Record<string, unknown>;
  tool_responses: Record<string, unknown[]>;
}

export type BackendFactory = (node: FlowNode, entry: Record<string, unknown>) => (state: ADPState) => ADPState;

export const COMPAT_MATRIX: Record<string, string[]> = {
  llm:       ["openai", "anthropic", "vllm", "bedrock", "langchain", "litellm"],
  tool:      ["python", "docker", "http", "mcp", "wasm"],
  retriever: ["pinecone", "weaviate", "chroma", "redis", "pgvector"],
  evaluator: [],
  router:    [],
  input:     [],
  output:    [],
};

function _assertAvailable(): void {
  if (!_lg) {
    throw new Error(
      "@langchain/langgraph required: npm install @langchain/langgraph"
    );
  }
}

/* c8 ignore start */
function parseCondition(conditionStr: string): (state: ADPState) => boolean {
  const parts = conditionStr.split(" ");
  if (parts.length < 3) throw new Error(`Invalid condition: ${conditionStr}`);
  const [keyPath, op, rawValue] = [parts[0], parts[1], parts.slice(2).join(" ")];
  const path = keyPath.split(".");
  let value: unknown;
  try { value = JSON.parse(rawValue); } catch { value = rawValue; }
  const ops: Record<string, (a: unknown, b: unknown) => boolean> = {
    "==": (a, b) => a === b,
    "!=": (a, b) => a !== b,
    ">":  (a, b) => (a as number) > (b as number),
    ">=": (a, b) => (a as number) >= (b as number),
    "<":  (a, b) => (a as number) < (b as number),
    "<=": (a, b) => (a as number) <= (b as number),
  };
  const opFn = ops[op];
  if (!opFn) throw new Error(`Unsupported operator '${op}' in condition: ${conditionStr}`);
  return (state: ADPState) => {
    let current: unknown = state;
    for (const segment of path) {
      current = (current as Record<string, unknown>)[segment];
    }
    return opFn(current, value);
  };
}

function _defaultCallable(node: FlowNode): (state: ADPState) => ADPState {
  const nodeId = node.id;
  const kind = node.kind;
  return (state: ADPState): ADPState => {
    const newState = { ...state };
    if (kind === "llm" || kind === "retriever" || kind === "evaluator") {
      newState.context = { ...state.context, [nodeId]: {} };
    } else if (kind === "tool") {
      const responses = [...(state.tool_responses[nodeId] ?? [])];
      newState.tool_responses = { ...state.tool_responses, [nodeId]: responses };
    }
    return newState;
  };
}
/* c8 ignore stop */

export function makeConditionFn(conditionStr: string): (state: ADPState) => boolean {
  return parseCondition(conditionStr);
}

/* c8 ignore start */
export function buildLangGraphFromAdp(
  manifest: ADP,
  backendFactory?: BackendFactory,
): { graph: unknown; nodeMap: Record<string, FlowNode> } {
  _assertAvailable();
  const { StateGraph, END } = _lg!;
  const flow = (manifest as any).flow.graph;
  const runtime = (manifest as any).runtime ?? {};
  const execution: unknown[] = runtime.execution ?? [];
  const nodeMap: Record<string, FlowNode> = {};

  const graph = new StateGraph({ channels: {} as any });

  for (const node of flow.nodes as FlowNode[]) {
    nodeMap[node.id] = node;
    const fn = backendFactory ? backendFactory(node, {}) : _defaultCallable(node);
    graph.addNode(node.id, fn as any);
  }

  const condEdges: Record<string, Array<{ from: string; to: string; condition: string }>> = {};
  for (const edge of flow.edges as Array<{ from: string; to: string; condition?: string }>) {
    if (edge.condition) {
      (condEdges[edge.from] ??= []).push(edge as any);
    } else {
      graph.addEdge(edge.from as any, edge.to as any);
    }
  }

  for (const [source, edges] of Object.entries(condEdges)) {
    const targetMap = Object.fromEntries(
      edges.map((e) => [e.to, makeConditionFn(e.condition)])
    );
    graph.addConditionalEdges(
      source as any,
      (state: ADPState) => {
        for (const [targetId, condFn] of Object.entries(targetMap)) {
          if (condFn(state)) return targetId;
        }
        return END;
      },
      { ...Object.fromEntries(Object.keys(targetMap).map((k) => [k, k])), [END]: END },
    );
  }

  for (const startId of flow.start_nodes as string[]) {
    graph.setEntryPoint(startId);
  }

  return { graph: graph.compile(), nodeMap };
}

export function adpFromLangGraph(
  graph: unknown,
  nodeMap: Record<string, FlowNode>,
  originalManifest: ADP,
): ADP {
  _assertAvailable();
  const draw = (graph as any).getGraph();
  const nodes = (draw.nodes as string[])
    .filter((nid) => nid !== "__start__" && nid !== "__end__")
    .map((nid) => nodeMap[nid] ?? { id: nid, kind: "unknown" as const });
  const edges = (draw.edges as Array<{ source: string; target: string }>)
    .filter((e) => e.source !== "__start__" && e.target !== "__end__")
    .map((e) => ({ from: e.source, to: e.target }));
  const startNodes = (draw.edges as Array<{ source: string; target: string }>)
    .filter((e) => e.source === "__start__").map((e) => e.target);
  const endNodes = (draw.edges as Array<{ source: string; target: string }>)
    .filter((e) => e.target === "__end__").map((e) => e.source);
  return {
    ...originalManifest,
    flow: {
      ...(originalManifest as any).flow,
      graph: { nodes, edges, start_nodes: startNodes, end_nodes: endNodes },
    },
  } as ADP;
}
/* c8 ignore stop */
