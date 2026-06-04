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
// v0.3.0 schemas
const memorySchema = loadSchema("memory.schema.json");
const workspaceSchema = loadSchema("workspace.schema.json");
const sandboxSchema = loadSchema("sandbox.schema.json");
const artifactsSchema = loadSchema("artifacts.schema.json");
const observabilitySchema = loadSchema("observability.schema.json");

ajv.addSchema(runtimeSchema, "runtime.schema.json");
ajv.addSchema(flowSchema, "flow.schema.json");
ajv.addSchema(evaluationSchema, "evaluation.schema.json");
ajv.addSchema(memorySchema, "memory.schema.json");
ajv.addSchema(workspaceSchema, "workspace.schema.json");
ajv.addSchema(sandboxSchema, "sandbox.schema.json");
ajv.addSchema(artifactsSchema, "artifacts.schema.json");
ajv.addSchema(observabilitySchema, "observability.schema.json");

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
  /* c8 ignore next */
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

const _DOT_PATH_RE = /^[a-zA-Z_][a-zA-Z0-9_]*(\.[a-zA-Z_][a-zA-Z0-9_]*)*$/;
const _VALID_TRACE_EVENTS = new Set([
  "model_request", "tool_call", "flow_node", "loop_iteration",
  "interrupt", "cost_check", "artifact_write"
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

  // Build nodeIds early — needed by both agentspec checks and graph checks.
  const nodes: any[] = graph?.nodes ?? [];
  const nodeIds = new Set<string>();
  for (const node of nodes) {
    if (nodeIds.has(node.id)) {
      errors.push(`duplicate node id '${node.id}' in graph.nodes`);
    }
    nodeIds.add(node.id);
  }

  // --- AgentSpec Interop Checks (AS-1, AS-2) ---
  // Placed before the graph early-return so AS-2 (runtime-only) always runs.
  {
    const agentspec = (adp as any)?.interop?.agentspec ?? {};

    // Check AS-1: node_map keys must match node IDs in flow.graph.nodes
    const nodeMap: Record<string, string> = agentspec.node_map ?? {};
    for (const mappedNodeId of Object.keys(nodeMap)) {
      if (!nodeIds.has(mappedNodeId)) {
        errors.push(
          `interop.agentspec.node_map: key '${mappedNodeId}' does not match any node id in flow.graph.nodes`
        );
      }
    }

    // Check AS-2: llm_map[].backend_id must match runtime.execution[].id (no graph dependency)
    const runtimeBackendIds = new Set<string>(
      (adp.runtime?.execution ?? []).map((e: any) => e.id).filter(Boolean)
    );
    for (const binding of agentspec.llm_map ?? []) {
      if (binding.backend_id && !runtimeBackendIds.has(binding.backend_id)) {
        errors.push(
          `interop.agentspec.llm_map: backend_id '${binding.backend_id}' does not match any id in runtime.execution`
        );
      }
    }

    // Check AS-3: ref MUST NOT contain path traversal sequences
    const ref: string = agentspec.ref ?? "";
    if (ref && ref.includes("..")) {
      errors.push(`interop.agentspec.ref '${ref}' MUST NOT contain path traversal sequences (..)`);
    }
  }

  if (!graph) return errors;

  const edges: any[] = graph?.edges ?? [];

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
  for (const toolListKey of ["mcp_servers", "http_apis", "sql_functions", "sandbox"] as const) {
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
  for (const toolListKey of ["mcp_servers", "http_apis", "sql_functions", "sandbox"] as const) {
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

  // =========================================================================
  // v0.3.0 Semantic Validation Checks (15-35b)
  // =========================================================================

  // --- Loop Checks (15-16) ---

  // Check 15: loop.body_nodes[] must reference known node IDs in flow.graph.nodes[]
  const loopNodes = nodes.filter((n) => n.kind === "loop");
  for (const loopNode of loopNodes) {
    const bodyNodes = loopNode.body_nodes ?? [];
    for (const bodyNodeId of bodyNodes) {
      if (!nodeIds.has(bodyNodeId)) {
        errors.push(
          `loop node '${loopNode.id}': body_nodes references '${bodyNodeId}' which is not found in graph.nodes`
        );
      }
    }
  }

  // Check 15b: loop.body_nodes[] must contain at least 2 nodes connected by at least one edge
  for (const loopNode of loopNodes) {
    const bodyNodes = loopNode.body_nodes ?? [];
    if (bodyNodes.length >= 2) {
      // Build adjacency from edges
      const edgeMap: Record<string, Set<string>> = {};
      for (const edge of edges) {
        if (!edgeMap[edge.from]) edgeMap[edge.from] = new Set();
        edgeMap[edge.from].add(edge.to);
      }
      // Check if any body node connects to another body node
      let hasConnection = false;
      for (const nodeId of bodyNodes) {
        if (edgeMap[nodeId]) {
          const connected = edgeMap[nodeId];
          for (const target of connected) {
            if (bodyNodes.includes(target)) {
              hasConnection = true;
              break;
            }
          }
        }
        if (hasConnection) break;
      }
      if (!hasConnection) {
        errors.push(
          `loop node '${loopNode.id}': body_nodes [${bodyNodes.join(", ")}] must contain at least 2 nodes connected by at least one edge`
        );
      }
    }
  }

  // Check 16: Loop node MUST NOT reference itself (directly or transitively) in body_nodes
  for (const loopNode of loopNodes) {
    const loopId = loopNode.id ?? "";
    const bodyNodes = loopNode.body_nodes ?? [];
    if (bodyNodes.includes(loopId)) {
      errors.push(
        `loop node '${loopId}': body_nodes MUST NOT reference the loop node itself`
      );
    }
    // Check transitive: if any body_node is a loop that has this loop in its body_nodes
    for (const bodyNodeId of bodyNodes) {
      const bodyNode = nodes.find((n) => n.id === bodyNodeId);
      if (bodyNode && bodyNode.kind === "loop") {
        const nestedBody = bodyNode.body_nodes ?? [];
        if (nestedBody.includes(loopId)) {
          errors.push(
            `loop node '${loopId}': circular loop reference detected with '${bodyNodeId}'`
          );
        }
      }
    }
  }

  // --- Tools Policy Checks (17, 29) ---

  // Check 17: policy.cache.key_fields[] entries MUST use dot-path notation
  for (const toolListKey of ["mcp_servers", "http_apis", "sql_functions", "sandbox"] as const) {
    for (const tool of tools[toolListKey] ?? []) {
      const policy = tool.policy ?? {};
      if (policy) {
        const cache = policy.cache ?? {};
        if (cache) {
          const keyFields = cache.key_fields ?? [];
          for (const field of keyFields) {
            if (!_DOT_PATH_RE.test(field)) {
              errors.push(
                `tool '${tool.id ?? "?"}': cache.key_fields entry '${field}' must use dot-path notation`
              );
            }
          }
        }
      }
    }
  }

  // Check 29: Any tool with load_strategy: "on_demand" MUST have a non-empty description
  for (const toolListKey of ["mcp_servers", "http_apis", "sql_functions"] as const) {
    for (const tool of tools[toolListKey] ?? []) {
      if (tool.load_strategy === "on_demand") {
        const desc = tool.description ?? "";
        if (!desc.trim()) {
          errors.push(
            `tool '${tool.id ?? "?"}': load_strategy 'on_demand' requires a non-empty description`
          );
        }
      }
    }
  }

  // --- Memory Checks (18-21c, 24) ---

  const memory = adp?.memory ?? {};

  // Check 18: memory.stores[] IDs must be unique
  if (typeof memory === "object" && !Array.isArray(memory)) {
    const stores = memory.stores ?? [];
    const storeIds: string[] = [];
    for (const store of stores) {
      if (store.id) storeIds.push(store.id);
    }
    const seen = new Set<string>();
    for (const sid of storeIds) {
      if (seen.has(sid)) {
        errors.push(`memory: duplicate store id '${sid}'`);
      }
      seen.add(sid);
    }

    // Check 19: memory.operations[].store_ref must reference a known stores[].id
    const operations = memory.operations ?? [];
    const storeIdsSet = new Set(storeIds);
    for (const op of operations) {
      const storeRef = op.store_ref;
      if (storeRef && !storeIdsSet.has(storeRef)) {
        errors.push(
          `memory.operations: store_ref '${storeRef}' not found in memory.stores`
        );
      }
    }

    // Check 20: memory.context_assembly.order[].store_ref must reference a known stores[].id
    const contextAssembly = memory.context_assembly ?? {};
    const order = contextAssembly.order ?? [];
    for (const item of order) {
      const storeRef = item.store_ref;
      if (storeRef && !storeIdsSet.has(storeRef)) {
        errors.push(
          `memory.context_assembly.order: store_ref '${storeRef}' not found in memory.stores`
        );
      }
    }

    // Check 21: memory.working.summary_model_ref (when present) must reference a known runtime.models[].id
    const working = memory.working ?? {};
    const summaryModelRef = working.summary_model_ref;
    if (summaryModelRef && !modelIds.has(summaryModelRef)) {
      errors.push(
        `memory.working.summary_model_ref '${summaryModelRef}' not found in runtime.models`
      );
    }

    // Check 21b: memory.working.summary_model_ref MUST be present when strategy = "summary"
    if (working.strategy === "summary" && !working.summary_model_ref) {
      errors.push(
        "memory.working: summary_model_ref MUST be present when strategy is 'summary'"
      );
    }

    // Check 21c: memory.working.compaction_threshold_tokens MUST be <= max_tokens
    const compaction = working.compaction_threshold_tokens;
    const maxTokens = working.max_tokens;
    if (compaction !== undefined && maxTokens !== undefined) {
      if (compaction > maxTokens) {
        errors.push(
          `memory.working: compaction_threshold_tokens (${compaction}) MUST be <= max_tokens (${maxTokens})`
        );
      }
    }

    // Check 24: memory.context_assembly.static_injection[].path validation
    const staticInjections = contextAssembly.static_injection ?? [];
    const hasWorkspace = Boolean(adp.workspace);
    for (const si of staticInjections) {
      if (si.source === "file") {
        const filePath = si.path ?? "";
        if (filePath.includes("..") || filePath.startsWith("/")) {
          errors.push(
            `memory.context_assembly.static_injection: path '${filePath}' must be a relative path without .. traversal`
          );
        }
        if (!hasWorkspace) {
          errors.push(
            `memory.context_assembly.static_injection: path '${filePath}' requires a workspace section to be declared`
          );
        }
      }
    }
  }

  // --- Guardrails Checks (22-23, 30) ---

  const guardrailsV3 = adp?.guardrails ?? {};

  // Check 22: guardrails.interrupts[].tool_refs[] must reference known tool IDs
  const interrupts = guardrailsV3.interrupts ?? [];
  const allToolIdsAllTypes = new Set<string>();
  for (const toolListKey of ["mcp_servers", "http_apis", "sql_functions", "sandbox"] as const) {
    for (const tool of tools[toolListKey] ?? []) {
      if (tool.id) allToolIdsAllTypes.add(tool.id);
    }
  }
  for (const interrupt of interrupts) {
    const toolRefs = interrupt.tool_refs ?? [];
    for (const toolRef of toolRefs) {
      if (!allToolIdsAllTypes.has(toolRef)) {
        errors.push(
          `guardrails.interrupts: tool_ref '${toolRef}' not found in tools`
        );
      }
    }
  }

  // Check 22b: guardrails.interrupts[].execution_mode MUST NOT be set when mode: "pause_and_notify"
  for (const interrupt of interrupts) {
    if (interrupt.mode === "pause_and_notify" && interrupt.execution_mode !== undefined) {
      errors.push(
        `guardrails.interrupts '${interrupt.id ?? "?"}': execution_mode MUST NOT be set when mode is 'pause_and_notify'`
      );
    }
  }

  // Check 23: guardrails.cost.interrupt_ref (when present) must reference a known guardrails.interrupts[].id
  const cost = guardrailsV3.cost ?? {};
  const interruptIds = new Set<string>();
  for (const interrupt of interrupts) {
    if (interrupt.id) interruptIds.add(interrupt.id);
  }
  if (cost.interrupt_ref && !interruptIds.has(cost.interrupt_ref)) {
    errors.push(
      `guardrails.cost.interrupt_ref '${cost.interrupt_ref}' not found in guardrails.interrupts`
    );
  }

  // Check 30: guardrails.cost.downgrade_model_ref MUST be present when on_threshold_exceeded: "downgrade"
  if (cost.on_threshold_exceeded === "downgrade") {
    if (!cost.downgrade_model_ref) {
      errors.push(
        "guardrails.cost: downgrade_model_ref MUST be present when on_threshold_exceeded is 'downgrade'"
      );
    } else if (!modelIds.has(cost.downgrade_model_ref)) {
      errors.push(
        `guardrails.cost.downgrade_model_ref '${cost.downgrade_model_ref}' not found in runtime.models`
      );
    }
  }

  // --- Workspace Checks (25-26, 31) ---

  const workspace = adp?.workspace;

  if (workspace) {
    // Check 25: workspace.permissions.write[] paths MUST NOT escape workspace.root (no .. traversal)
    const permissions = workspace.permissions ?? {};
    for (const writePath of permissions.write ?? []) {
      if (writePath.includes("..")) {
        errors.push(
          `workspace.permissions.write: path '${writePath}' MUST NOT escape workspace.root`
        );
      }
    }

    // Check 25b: Exactly one of workspace.root or workspace.root_env_var MUST be present
    const root = workspace.root;
    const rootEnvVar = workspace.root_env_var;
    if (root !== undefined && rootEnvVar !== undefined) {
      errors.push(
        "workspace: exactly one of 'root' or 'root_env_var' MUST be present, not both"
      );
    }
    if (root === undefined && rootEnvVar === undefined) {
      errors.push(
        "workspace: exactly one of 'root' or 'root_env_var' MUST be present"
      );
    }

    // Check 26: workspace.git.auto_commit: true requires workspace.git.enabled: true
    const git = workspace.git ?? {};
    if (git.auto_commit && !git.enabled) {
      errors.push(
        "workspace.git: auto_commit requires enabled to be true"
      );
    }

    // Check 31: workspace.mounts[].id values must be unique; workspace.mounts[].target paths MUST NOT escape workspace.root
    const mounts = workspace.mounts ?? [];
    const mountIds: string[] = [];
    for (const mount of mounts) {
      if (mount.id) mountIds.push(mount.id);
      const target = mount.target ?? "";
      if (target.includes("..")) {
        errors.push(
          `workspace.mounts: target path '${target}' MUST NOT escape workspace.root`
        );
      }
    }
    if (mountIds.length !== new Set(mountIds).size) {
      const seen = new Set<string>();
      for (const mid of mountIds) {
        if (seen.has(mid)) {
          errors.push(`workspace.mounts: duplicate mount id '${mid}'`);
        }
        seen.add(mid);
      }
    }
  } // end if (workspace)

  // --- Sandbox Checks (27-28, 32) ---

  const sandboxTools = tools.sandbox ?? [];

  // Check 27: tools.sandbox[].policy.timeout_ms MUST be present
  for (const sandbox of sandboxTools) {
    const policy = sandbox.policy ?? {};
    if (typeof policy === "object" && !("timeout_ms" in policy)) {
      errors.push(
        `tools.sandbox '${sandbox.id ?? "?"}': policy.timeout_ms MUST be present (no unbounded sandbox execution)`
      );
    }
  }

  // Check 28: tools.sandbox[].mounts[].source: "workspace" requires a workspace section to be declared
  const hasWorkspaceDeclared = Boolean(adp.workspace);
  for (const sandbox of sandboxTools) {
    const sandboxMounts = sandbox.mounts ?? [];
    for (const mount of sandboxMounts) {
      if (mount.source === "workspace" && !hasWorkspaceDeclared) {
        errors.push(
          `tools.sandbox '${sandbox.id ?? "?"}': mounts[].source 'workspace' requires a workspace section`
        );
      }
    }
  }

  // Check 32: tools.sandbox[].snapshot.enabled: true with provider: "custom" emits a WARNING
  for (const sandbox of sandboxTools) {
    const snapshot = sandbox.snapshot ?? {};
    const provider = sandbox.provider;
    if (snapshot.enabled && provider === "custom") {
      errors.push(
        `WARNING: tools.sandbox '${sandbox.id ?? "?"}': snapshot.enabled with provider 'custom' may not be supported`
      );
    }
  }

  // --- Artifacts Checks (33-34) ---

  const artifacts = adp?.artifacts ?? {};

  // Check 33: artifacts.stores[].id must be unique
  if (typeof artifacts === "object") {
    const artifactStores = artifacts.stores ?? [];
    const artifactStoreIds: string[] = [];
    for (const store of artifactStores) {
      if (store.id) artifactStoreIds.push(store.id);
    }
    if (artifactStoreIds.length !== new Set(artifactStoreIds).size) {
      const seen = new Set<string>();
      for (const sid of artifactStoreIds) {
        if (seen.has(sid)) {
          errors.push(`artifacts.stores: duplicate store id '${sid}'`);
        }
        seen.add(sid);
      }
    }

    // Check 34: nodes[].params.artifact.store_ref must reference a known artifacts.stores[].id
    const artifactStoreIdsSet = new Set(artifactStoreIds);
    for (const node of nodes) {
      const params = node.params ?? {};
      const artifact = params.artifact ?? {};
      const storeRef = artifact.store_ref;
      if (storeRef && !artifactStoreIdsSet.has(storeRef)) {
        errors.push(
          `node '${node.id}' params.artifact.store_ref '${storeRef}' not found in artifacts.stores`
        );
      }
    }
  }

  // --- Observability Checks (35-35b) ---

  const observability = adp?.observability ?? {};

  // Check 35: observability.tracing.trace_events[] entries must be from the valid enum
  const tracing = observability.tracing ?? {};
  for (const event of tracing.trace_events ?? []) {
    if (!_VALID_TRACE_EVENTS.has(event)) {
      errors.push(
        `observability.tracing.trace_events: '${event}' is not a valid trace event`
      );
    }
  }

  // Check 35b: observability.cost_reporting.model_refs[] (when present) must reference known runtime.models[].id
  const costReporting = observability.cost_reporting ?? {};
  for (const modelRef of costReporting.model_refs ?? []) {
    if (!modelIds.has(modelRef)) {
      errors.push(
        `observability.cost_reporting.model_refs: '${modelRef}' not found in runtime.models`
      );
    }
  }

  return errors;
}
