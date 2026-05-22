/**
 * ADP → Semantic Kernel conversion utilities. Import-only: ADP → SK.
 * Export (SK → ADP) is deferred to v0.3.0.
 *
 * Works without @microsoft/semantic-kernel installed (mock mode).
 * Targets semantic-kernel >= 1.0 (Node.js SDK).
 *
 * See spec/framework-interop.md §Semantic Kernel Mapping for the mapping guide.
 */
import { createRequire } from "module";
import type { ADP, FlowNode } from "../adp.js";

const _require = createRequire(import.meta.url);
let _sk: any = null;
try {
  _sk = _require("@microsoft/semantic-kernel");
} catch {
  _sk = null;
}

export interface ProcessStep {
  id: string;
  kind: string;
  model_ref?: string;
  provider?: string;
  model?: string;
  tool_ref?: string;
  sk_construct?: string;
}

export type SKBackendFactory = (manifest: ADP, options: Record<string, unknown>) => void;

export function buildSKFromAdp(
  manifest: ADP,
  backendFactory?: SKBackendFactory,
): { kernel: unknown; processSteps: ProcessStep[] } {
  const flow = (manifest as any).flow.graph;
  const runtime = (manifest as any).runtime ?? {};
  const models: Array<{ id: string; provider: string; model: string }> = runtime.models ?? [];

  let kernel: unknown;
  if (_sk) {
    const { Kernel } = _sk as any;
    kernel = new Kernel();
    if (backendFactory) backendFactory(manifest, { kernel });
  } else {
    kernel = { type: "mock_kernel" };
  }

  const processSteps: ProcessStep[] = [];
  for (const node of flow.nodes as FlowNode[]) {
    const kind = node.kind;
    let step: ProcessStep;
    if (kind === "llm") {
      const modelRef = node.model_ref as string | undefined;
      const modelEntry = models.find((m) => m.id === modelRef);
      step = { id: node.id, kind: "llm", model_ref: modelRef };
      if (modelEntry) {
        step.provider = modelEntry.provider;
        step.model = modelEntry.model;
      }
      if (_sk) step.sk_construct = "KernelFunction";
    } else if (kind === "tool") {
      step = { id: node.id, kind: "tool", tool_ref: node.tool_ref as string | undefined };
      if (_sk) step.sk_construct = "KernelPlugin";
    } else {
      step = { id: node.id, kind: kind as string };
    }
    processSteps.push(step);
  }

  return { kernel, processSteps };
}
