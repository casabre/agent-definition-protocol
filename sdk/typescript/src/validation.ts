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

const _GEN_AI_ATTR_RE = /^gen_ai\.[a-z0-9_.]+$|^x_[a-z0-9]+\.[a-z0-9_.]+$/;
const _KNOWN_COMPLIANCE_STANDARDS = new Set([
  "gdpr",
  "hipaa",
  "soc2",
  "eu-ai-act",
  "iso-27001",
  "fedramp",
]);

export function validateAdpSemantics(adp: any): string[] {
  const errors: string[] = [];

  // Pre-composition guard: warn if unresolved composition fields are present
  if (adp?.extends || adp?.import) {
    errors.push(
      "WARNING: manifest has unresolved composition fields (extends/import); " +
        "semantic validation may be incomplete — call resolveAdp() first"
    );
  }

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

  // Check 7: guardrail policy_ref must be non-empty
  const guardrails = adp?.guardrails ?? {};
  for (const railListKey of ["input", "output"] as const) {
    for (const rail of guardrails[railListKey] ?? []) {
      if (!(rail.policy_ref ?? "").trim()) {
        errors.push(`guardrail '${rail.id ?? "?"}': policy_ref is empty`);
      }
    }
  }

  // Check 8: telemetry.required_attributes must match gen_ai.* or x_<vendor>.*
  const telemetry = adp?.telemetry ?? {};
  for (const attr of telemetry.required_attributes ?? []) {
    if (!_GEN_AI_ATTR_RE.test(attr)) {
      errors.push(
        `telemetry.required_attributes: '${attr}' is not a valid gen_ai.* or x_<vendor>.* attribute`
      );
    }
  }

  // Check 9: tool auth.env_var required when scheme != "none"
  const tools = adp?.tools ?? {};
  for (const toolListKey of ["mcp_servers", "http_apis", "sql_functions"] as const) {
    for (const tool of tools[toolListKey] ?? []) {
      const auth = tool.auth ?? {};
      if (auth && (auth.scheme ?? "none") !== "none") {
        if (!(auth.env_var ?? "").trim()) {
          errors.push(
            `tool '${tool.id ?? "?"}': auth.env_var required when scheme is not 'none'`
          );
        }
      }
    }
  }

  // Check 10: compliance standard must be known or start with x_
  const governance = adp?.governance ?? {};
  for (const entry of governance.compliance ?? []) {
    const standard = entry.standard ?? "";
    if (!_KNOWN_COMPLIANCE_STANDARDS.has(standard) && !standard.startsWith("x_")) {
      errors.push(
        `compliance standard '${standard}' is unknown; use x_<vendor>.<name> for custom standards`
      );
    }
  }

  // Check 11: node tool_ref must match a tool ID in tools.*
  const allToolIds = new Set<string>();
  for (const toolListKey of ["mcp_servers", "http_apis", "sql_functions"] as const) {
    for (const tool of tools[toolListKey] ?? []) {
      if (tool.id) allToolIds.add(tool.id);
    }
  }
  for (const node of nodes) {
    if (node.tool_ref && !allToolIds.has(node.tool_ref)) {
      errors.push(
        `node '${node.id}' tool_ref '${node.tool_ref}' not found in tools`
      );
    }
  }

  // Check 12: hooks[].node_filter entries must reference known flow node IDs
  const hooks: any[] = adp?.hooks ?? [];
  for (const hook of hooks) {
    for (const filterId of hook?.node_filter ?? []) {
      if (!nodeIds.has(filterId)) {
        errors.push(
          `hook event '${hook.event ?? "?"}' node_filter '${filterId}' does not reference a known flow node`
        );
      }
    }
  }

  // Check 13: subflow nodes with adp_ref that is not a URI/path must resolve to subagents[].id
  const subagents: any[] = adp?.subagents ?? [];
  const subagentIds = new Set<string>(subagents.map((s: any) => s.id).filter(Boolean));
  for (const node of nodes) {
    if (node.kind === "subflow" && node.adp_ref) {
      const ref: string = node.adp_ref;
      const isUriOrPath =
        /^[a-zA-Z][a-zA-Z0-9+\-.]*:/.test(ref) ||
        ref.includes("/") ||
        ref.endsWith(".yaml") ||
        ref.endsWith(".json");
      if (!isUriOrPath && !subagentIds.has(ref)) {
        errors.push(
          `subflow node '${node.id}' adp_ref '${ref}' does not resolve to a known subagents[] entry`
        );
      }
    }
  }

  // Check 14: evaluator evaluator_ref must resolve to a known x_testing evaluator ID
  const xTesting: any = (adp as any)?.x_testing ?? {};
  const testingEvaluatorIds = new Set<string>();
  for (const ev of xTesting?.evaluators ?? []) {
    if (ev.id) testingEvaluatorIds.add(ev.id);
  }
  for (const judge of xTesting?.judges ?? []) {
    if (judge.id) testingEvaluatorIds.add(judge.id);
  }
  if (testingEvaluatorIds.size > 0) {
    for (const suite of suites) {
      for (const metric of (suite as any)?.metrics ?? []) {
        const evaluatorRef = metric?.evaluator_ref;
        if (evaluatorRef && !testingEvaluatorIds.has(evaluatorRef)) {
          errors.push(
            `evaluator '${metric.id ?? "?"}' evaluator_ref '${evaluatorRef}' does not resolve to a known x_testing evaluator`
          );
        }
      }
    }
  }

  // Deprecation warning: judges[] without evaluators[]
  if ((xTesting?.judges ?? []).length > 0 && (xTesting?.evaluators ?? []).length === 0) {
    errors.push(
      "WARNING: x_testing.judges[] is deprecated; migrate to x_testing.evaluators[]"
    );
  }

  return errors;
}
