import fs from "fs";
import path from "path";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";
import yaml from "js-yaml";
import { fileURLToPath } from "url";

const ajv = new Ajv2020({ allErrors: true, allowUnionTypes: true });
addFormats(ajv);
const __dirname = path.dirname(fileURLToPath(import.meta.url));

function loadSchema(name: string) {
  // Move from dist-test/src or dist/src back to repository root.
  const base = path.resolve(__dirname, "../../../../schemas", name);
  return JSON.parse(fs.readFileSync(base, "utf8"));
}

const adpSchema = loadSchema("adp.schema.json");
const runtimeSchema = loadSchema("runtime.schema.json");
const flowSchema = loadSchema("flow.schema.json");
const evaluationSchema = loadSchema("evaluation.schema.json");
ajv.addSchema(runtimeSchema, "runtime.schema.json");
ajv.addSchema(flowSchema, "flow.schema.json");
ajv.addSchema(evaluationSchema, "evaluation.schema.json");

export function validateAdp(adp: any): string[] {
  // Conformance class enforcement
  const conformanceClass = adp?.conformance_class;
  const isFlowEmpty = !adp?.flow || (typeof adp.flow === "object" && Object.keys(adp.flow).length === 0);
  const isEvalEmpty = !adp?.evaluation || (typeof adp.evaluation === "object" && Object.keys(adp.evaluation).length === 0);
  if (conformanceClass === "full" && isFlowEmpty) {
    return ["conformance_class 'full' declared but flow is empty"];
  }
  if (conformanceClass === "full" && isEvalEmpty) {
    return ["conformance_class 'full' declared but evaluation is empty"];
  }

  const validate = ajv.compile(adpSchema);
  const ok = validate(adp);
  if (ok) return [];
  return (validate.errors || []).map((e) => `${e.instancePath} ${e.message}`.trim());
}

export function validateAdpSemantics(adp: any): string[] {
  const errors: string[] = [];
  const graph = adp?.flow?.graph;
  if (!graph) return errors;

  const nodes: any[] = graph?.nodes ?? [];
  const edges: any[] = graph?.edges ?? [];

  const nodeIds = new Set<string>();
  for (const node of nodes) {
    if (nodeIds.has(node.id)) {
      errors.push(`duplicate node id '${node.id}' in graph.nodes`);
    }
    nodeIds.add(node.id);
  }

  for (const edge of edges) {
    if (!nodeIds.has(edge.from)) {
      errors.push(`edge from '${edge.from}' to '${edge.to}': node '${edge.from}' not found in graph.nodes`);
    }
    if (!nodeIds.has(edge.to)) {
      errors.push(`edge from '${edge.from}' to '${edge.to}': node '${edge.to}' not found in graph.nodes`);
    }
  }

  for (const nid of (graph?.start_nodes ?? [])) {
    if (!nodeIds.has(nid)) errors.push(`start_node '${nid}' not found in graph.nodes`);
  }
  for (const nid of (graph?.end_nodes ?? [])) {
    if (!nodeIds.has(nid)) errors.push(`end_node '${nid}' not found in graph.nodes`);
  }

  const suites: any[] = adp?.evaluation?.suites ?? [];
  const suiteIds = new Set(suites.map((s: any) => s.id));
  const models: any[] = adp?.runtime?.models ?? [];
  const modelIds = new Set(models.map((m: any) => m.id));
  const executionIds = new Set((adp?.runtime?.execution ?? []).map((e: any) => e.id));

  for (const node of nodes) {
    if (node.suite_ref && !suiteIds.has(node.suite_ref)) {
      errors.push(`node '${node.id}' suite_ref '${node.suite_ref}' not found in evaluation.suites`);
    }
    if (node.model_ref && models.length > 0 && !modelIds.has(node.model_ref)) {
      errors.push(`node '${node.id}' model_ref '${node.model_ref}' not found in runtime.models`);
    }
    if (node.runtime_ref && !executionIds.has(node.runtime_ref)) {
      errors.push(`node '${node.id}' runtime_ref '${node.runtime_ref}' not found in runtime.execution`);
    }
  }

  return errors;
}
