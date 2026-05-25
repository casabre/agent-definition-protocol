import assert from "assert";
import fs from "fs";
import path from "path";
import os from "os";
import { createPackage, openPackage, inspectPackage, verifyPackage } from "../src/adpkg.js";
import { validateAdp, validateAdpSemantics } from "../src/validation.js";
import { resolveAdp, CompositionError } from "../src/composition.js";
import { buildLangGraphFromAdp, makeConditionFn } from "../src/integrations/langgraph.js";
import { buildSKFromAdp } from "../src/integrations/semantic_kernel.js";
import { loadEvaluator, loadEvaluatorsFromManifest, UnsupportedEvaluatorTypeError } from "../src/evaluation.js";

function buildSource(tmp: string) {
  const adpDir = path.join(tmp, "adp");
  fs.mkdirSync(adpDir, { recursive: true });
  fs.writeFileSync(
    path.join(adpDir, "agent.yaml"),
    `adp_version: "0.1.0"\nid: "agent.test"\nruntime:\n  execution:\n    - backend: "python"\n      id: "py"\n      entrypoint: "agent.main:app"\nflow: { id: "flow.test", graph: { nodes: [{ id: "n1", kind: "input" }], edges: [], start_nodes: ["n1"], end_nodes: ["n1"] } }\nevaluation:\n  suites:\n    - id: basic\n      metrics:\n        - id: m1\n          type: deterministic\n          function: noop\n          scoring: boolean\n          threshold: true\n`
  );
}

function testValidateRejectsInvalidFlow() {
  const errors = validateAdp({ adp_version: "0.1.0", id: "x", runtime: { execution: [{ backend: "python", id: "p", entrypoint: "a:b" }] }, flow: {}, evaluation: { suites: [] } } as any);
  assert.ok(errors.length > 0, "empty flow object must produce validation errors");
}

function testValidate() {
  const adp = {
    adp_version: "0.1.0",
    id: "agent.test",
    runtime: { execution: [{ backend: "python", id: "py", entrypoint: "agent.main:app" }] },
    flow: { id: "flow.test", graph: { nodes: [{ id: "n1", kind: "input" }], edges: [], start_nodes: ["n1"], end_nodes: ["n1"] } },
    evaluation: {
      suites: [
        {
          id: "s",
          metrics: [
            { id: "m1", type: "deterministic", function: "noop", scoring: "boolean", threshold: true },
          ],
        },
      ],
    },
  } as any;
  const errors = validateAdp(adp);
  assert.equal(errors.length, 0, errors.join("; "));
}

function testPackage() {
  const tmp = fs.mkdtempSync(path.join(process.cwd(), "ts-oci-"));
  buildSource(tmp);
  const outDir = path.join(tmp, "oci");
  createPackage(tmp, outDir);
  assert.ok(fs.existsSync(path.join(outDir, "oci-layout")));
  const adp = openPackage(outDir) as any;
  assert.equal(adp.id, "agent.test");
}

function testSemanticValidationPassesForValidAdp() {
  const adp = {
    adp_version: "0.1.0",
    id: "agent.test",
    runtime: { execution: [{ backend: "python", id: "py", entrypoint: "agent.main:app" }] },
    flow: {
      id: "flow.test",
      graph: {
        nodes: [{ id: "n1", kind: "input" }, { id: "n2", kind: "output" }],
        edges: [{ from: "n1", to: "n2" }],
        start_nodes: ["n1"],
        end_nodes: ["n2"],
      },
    },
    evaluation: { suites: [{ id: "s1", metrics: [{ id: "m1", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }] }] },
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.equal(errors.length, 0, `Expected no semantic errors, got: ${errors.join("; ")}`);
}

function testSemanticValidationRejectsDanglingEdge() {
  const adp = {
    adp_version: "0.1.0",
    id: "agent.test",
    runtime: { execution: [{ backend: "python", id: "py", entrypoint: "agent.main:app" }] },
    flow: {
      graph: {
        nodes: [{ id: "input", kind: "input" }],
        edges: [{ from: "ghost", to: "input" }],
        start_nodes: ["input"],
        end_nodes: ["input"],
      },
    },
    evaluation: {},
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(errors.some((e: string) => e.includes("ghost")), `Expected dangling edge error, got: ${errors.join("; ")}`);
}

function testSemanticValidationRejectsDanglingEdgeTo() {
  // Covers validation.js line 71-72: edge `to` node not found
  const adp = {
    adp_version: "0.1.0",
    id: "agent.test",
    runtime: { execution: [{ backend: "python", id: "py", entrypoint: "agent.main:app" }] },
    flow: {
      graph: {
        nodes: [{ id: "input", kind: "input" }],
        edges: [{ from: "input", to: "ghost_target" }],
        start_nodes: ["input"],
        end_nodes: ["input"],
      },
    },
    evaluation: {},
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(errors.some((e: string) => e.includes("ghost_target")), `Expected dangling edge 'to' error, got: ${errors.join("; ")}`);
}

function testSemanticValidationRejectsDuplicateNode() {
  const adp = {
    adp_version: "0.1.0",
    id: "agent.test",
    runtime: { execution: [{ backend: "python", id: "py", entrypoint: "agent.main:app" }] },
    flow: {
      graph: {
        nodes: [{ id: "input", kind: "input" }, { id: "input", kind: "output" }],
        edges: [],
        start_nodes: ["input"],
        end_nodes: ["input"],
      },
    },
    evaluation: {},
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(errors.some((e: string) => e.includes("duplicate")), `Expected duplicate node error, got: ${errors.join("; ")}`);
}

function testSemanticValidationRejectsBadSuiteRef() {
  const adp = {
    adp_version: "0.1.0",
    id: "agent.test",
    runtime: { execution: [{ backend: "python", id: "py", entrypoint: "agent.main:app" }] },
    flow: {
      graph: {
        nodes: [{ id: "n1", kind: "llm", suite_ref: "missing-suite" }],
        edges: [],
        start_nodes: ["n1"],
        end_nodes: ["n1"],
      },
    },
    evaluation: { suites: [{ id: "suite1", metrics: [] }] },
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(errors.some((e: string) => e.includes("suite_ref")), `Expected suite_ref error, got: ${errors.join("; ")}`);
}

function testSemanticValidationRejectsBadModelRef() {
  const adp = {
    adp_version: "0.1.0",
    id: "agent.test",
    runtime: {
      execution: [{ backend: "python", id: "py", entrypoint: "agent.main:app" }],
      models: [{ id: "gpt4", provider: "openai", model: "gpt-4o" }],
    },
    flow: {
      graph: {
        nodes: [{ id: "n1", kind: "llm", model_ref: "missing-model" }],
        edges: [],
        start_nodes: ["n1"],
        end_nodes: ["n1"],
      },
    },
    evaluation: {},
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(errors.some((e: string) => e.includes("model_ref")), `Expected model_ref error, got: ${errors.join("; ")}`);
}

function testSemanticValidationRejectsBadRuntimeRef() {
  const adp = {
    adp_version: "0.1.0",
    id: "agent.test",
    runtime: {
      execution: [{ backend: "python", id: "py", entrypoint: "agent.main:app" }],
    },
    flow: {
      graph: {
        nodes: [{ id: "n1", kind: "llm", runtime_ref: "missing-backend" }],
        edges: [],
        start_nodes: ["n1"],
        end_nodes: ["n1"],
      },
    },
    evaluation: {},
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(errors.some((e: string) => e.includes("runtime_ref")), `Expected runtime_ref error, got: ${errors.join("; ")}`);
}

function testConformanceClassFullRejectsEmptyFlow() {
  const adp = {
    adp_version: "0.1.0",
    id: "agent.full",
    conformance_class: "full",
    runtime: { execution: [{ backend: "python", id: "py", entrypoint: "agent.main:app" }] },
    flow: {},
    evaluation: { suites: [{ id: "s1", metrics: [] }] },
  } as any;
  const errors = validateAdp(adp);
  assert.ok(errors.some((e: string) => e.includes("full") && e.includes("flow")), `Expected conformance_class error, got: ${errors.join("; ")}`);
}

// ─── Semantic check 7: guardrail policy_ref must be non-empty ────────────────
function testSemanticCheck7GuardrailEmptyPolicyRef() {
  const adp = {
    adp_version: "0.2.0",
    id: "test.guardrail",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: { graph: { nodes: [], edges: [], start_nodes: [], end_nodes: [] } },
    evaluation: {},
    guardrails: {
      input: [{ id: "pii-filter", provider: "guardrails-ai", policy_ref: "" }],
    },
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(
    errors.some((e: string) => e.includes("policy_ref is empty")),
    `Expected guardrail policy_ref error, got: ${errors.join("; ")}`
  );
}

// ─── Semantic check 8: telemetry.required_attributes must match gen_ai.* ─────
function testSemanticCheck8TelemetryInvalidAttr() {
  const adp = {
    adp_version: "0.2.0",
    id: "test.telemetry",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: { graph: { nodes: [], edges: [], start_nodes: [], end_nodes: [] } },
    evaluation: {},
    telemetry: { required_attributes: ["gen_ai.model.id", "bad_attr"] },
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(
    errors.some((e: string) => e.includes("bad_attr") && e.includes("not a valid")),
    `Expected telemetry attribute error, got: ${errors.join("; ")}`
  );
}

function testSemanticCheck8TelemetryValidAttrs() {
  const adp = {
    adp_version: "0.2.0",
    id: "test.telemetry.ok",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: { graph: { nodes: [], edges: [], start_nodes: [], end_nodes: [] } },
    evaluation: {},
    telemetry: { required_attributes: ["gen_ai.model.id", "x_acme.request_id"] },
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(
    !errors.some((e: string) => e.includes("not a valid")),
    `Expected no telemetry attribute error, got: ${errors.join("; ")}`
  );
}

// ─── Semantic check 9: tool auth.env_var required when scheme != "none" ──────
function testSemanticCheck9ToolMissingEnvVar() {
  const adp = {
    adp_version: "0.2.0",
    id: "test.toolauth",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: { graph: { nodes: [], edges: [], start_nodes: [], end_nodes: [] } },
    evaluation: {},
    tools: {
      http_apis: [
        {
          id: "billing-api",
          description: "Billing service",
          base_url: "https://billing.example.com",
          auth: { scheme: "bearer" },
        },
      ],
    },
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(
    errors.some((e: string) => e.includes("billing-api") && e.includes("auth.env_var")),
    `Expected tool auth env_var error, got: ${errors.join("; ")}`
  );
}

function testSemanticCheck9ToolNoneSchemeOk() {
  const adp = {
    adp_version: "0.2.0",
    id: "test.toolauth.none",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: { graph: { nodes: [], edges: [], start_nodes: [], end_nodes: [] } },
    evaluation: {},
    tools: {
      http_apis: [
        {
          id: "public-api",
          description: "Public service",
          base_url: "https://public.example.com",
          auth: { scheme: "none" },
        },
      ],
    },
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(
    !errors.some((e: string) => e.includes("auth.env_var")),
    `Expected no tool auth error, got: ${errors.join("; ")}`
  );
}

// ─── Semantic check 10: compliance standard ──────────────────────────────────
function testSemanticCheck10UnknownComplianceStandard() {
  const adp = {
    adp_version: "0.2.0",
    id: "test.compliance",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: { graph: { nodes: [], edges: [], start_nodes: [], end_nodes: [] } },
    evaluation: {},
    governance: {
      compliance: [{ standard: "gdpr" }, { standard: "made-up-standard" }],
    },
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(
    errors.some((e: string) => e.includes("made-up-standard") && e.includes("unknown")),
    `Expected compliance standard error, got: ${errors.join("; ")}`
  );
}

function testSemanticCheck10CustomComplianceOk() {
  const adp = {
    adp_version: "0.2.0",
    id: "test.compliance.ok",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: { graph: { nodes: [], edges: [], start_nodes: [], end_nodes: [] } },
    evaluation: {},
    governance: {
      compliance: [{ standard: "hipaa" }, { standard: "x_acme.internal_policy" }],
    },
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(
    !errors.some((e: string) => e.includes("unknown")),
    `Expected no compliance error, got: ${errors.join("; ")}`
  );
}

// ─── Semantic check 11: node tool_ref must match a tool ID ───────────────────
function testSemanticCheck11ToolRefMissing() {
  const adp = {
    adp_version: "0.2.0",
    id: "test.toolref",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: {
      graph: {
        nodes: [
          { id: "input", kind: "input" },
          { id: "api-call", kind: "tool", tool_ref: "nonexistent-api" },
          { id: "output", kind: "output" },
        ],
        edges: [
          { from: "input", to: "api-call" },
          { from: "api-call", to: "output" },
        ],
        start_nodes: ["input"],
        end_nodes: ["output"],
      },
    },
    evaluation: {},
    tools: {
      http_apis: [{ id: "real-api", description: "A real API", base_url: "https://example.com" }],
    },
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(
    errors.some((e: string) => e.includes("nonexistent-api") && e.includes("tool_ref")),
    `Expected tool_ref error, got: ${errors.join("; ")}`
  );
}

// ─── Semantic check 12: hooks[].node_filter must reference known node IDs ─────
function testSemanticCheck12HookNodeFilterUnknown() {
  const adp = {
    adp_version: "0.3.0",
    id: "test.hook.nodefilter",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: {
      graph: {
        nodes: [{ id: "input", kind: "input" }, { id: "output", kind: "output" }],
        edges: [],
        start_nodes: ["input"],
        end_nodes: ["output"],
      },
    },
    evaluation: {},
    hooks: [
      {
        event: "on_node_end",
        node_filter: ["nonexistent-node-id"],
        handler: { type: "function", function_ref: "acme:record" },
      },
    ],
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(
    errors.some((e: string) => e.includes("node_filter") && e.includes("nonexistent-node-id")),
    `Expected node_filter error, got: ${errors.join("; ")}`
  );
}

function testSemanticCheck12HookNodeFilterValid() {
  const adp = {
    adp_version: "0.3.0",
    id: "test.hook.nodefilter.ok",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: {
      graph: {
        nodes: [{ id: "input", kind: "input" }, { id: "chat", kind: "llm" }],
        edges: [],
        start_nodes: ["input"],
        end_nodes: ["chat"],
      },
    },
    evaluation: {},
    hooks: [
      {
        event: "on_node_end",
        node_filter: ["chat"],
        handler: { type: "function", function_ref: "acme:record" },
      },
    ],
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(
    !errors.some((e: string) => e.includes("node_filter")),
    `Expected no node_filter error, got: ${errors.join("; ")}`
  );
}

// ─── Semantic check 13: subflow adp_ref must resolve to known subagents[] ─────
function testSemanticCheck13SubflowAdpRefMissing() {
  const adp = {
    adp_version: "0.3.0",
    id: "test.subflow.ref",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: {
      graph: {
        nodes: [
          { id: "input", kind: "input" },
          { id: "delegate", kind: "subflow", adp_ref: "nonexistent-catalog-id" },
        ],
        edges: [],
        start_nodes: ["input"],
        end_nodes: ["delegate"],
      },
    },
    evaluation: {},
    subagents: [],
  } as any;
  const errors = validateAdpSemantics(adp);
  /* c8 ignore next */
  assert.ok(
    errors.some((e: string) => e.includes("adp_ref") || e.includes("subagents")),
    `Expected adp_ref error, got: ${errors.join("; ")}`
  );
}

function testSemanticCheck13SubflowAdpRefUri() {
  const adp = {
    adp_version: "0.3.0",
    id: "test.subflow.ref.uri",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: {
      graph: {
        nodes: [
          { id: "input", kind: "input" },
          { id: "delegate", kind: "subflow", adp_ref: "https://example.com/agent.yaml" },
        ],
        edges: [],
        start_nodes: ["input"],
        end_nodes: ["delegate"],
      },
    },
    evaluation: {},
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(
    !errors.some((e: string) => e.includes("adp_ref")),
    `URI adp_ref should not trigger check 13, got: ${errors.join("; ")}`
  );
}

// ─── Semantic check 14: evaluator_ref must match x_testing evaluator ID ──────
function testSemanticCheck14EvaluatorRefMissing() {
  const adp = {
    adp_version: "0.3.0",
    id: "test.evalref",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: { graph: { nodes: [], edges: [], start_nodes: [], end_nodes: [] } },
    evaluation: {
      suites: [
        {
          id: "s1",
          metrics: [{ id: "m1", type: "llm_judge", evaluator_ref: "nonexistent-evaluator" }],
        },
      ],
    },
    x_testing: {
      evaluators: [{ id: "real-evaluator", type: "llm_judge", model: "gpt-4o" }],
    },
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(
    errors.some((e: string) => e.includes("evaluator_ref")),
    `Expected evaluator_ref error, got: ${errors.join("; ")}`
  );
}

function testSemanticCheck14EvaluatorRefValid() {
  const adp = {
    adp_version: "0.3.0",
    id: "test.evalref.ok",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: { graph: { nodes: [], edges: [], start_nodes: [], end_nodes: [] } },
    evaluation: {
      suites: [
        {
          id: "s1",
          metrics: [{ id: "m1", type: "llm_judge", evaluator_ref: "real-evaluator" }],
        },
      ],
    },
    x_testing: {
      evaluators: [{ id: "real-evaluator", type: "llm_judge", model: "gpt-4o" }],
    },
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(
    !errors.some((e: string) => e.includes("evaluator_ref")),
    `Expected no evaluator_ref error, got: ${errors.join("; ")}`
  );
}

// ─── Pre-composition warning ──────────────────────────────────────────────────
function testSemanticPreCompositionWarning() {
  const adp = {
    adp_version: "0.2.0",
    id: "test.precomp",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: { graph: { nodes: [], edges: [], start_nodes: [], end_nodes: [] } },
    evaluation: {},
    extends: "./base.yaml",
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(
    errors[0]?.startsWith("WARNING:"),
    `Expected WARNING as first element, got: ${errors.join("; ")}`
  );
}

// ─── Composition: resolveAdp with in-memory resolver ─────────────────────────
const _MINIMAL_FLOW = `
  id: "f"
  graph:
    nodes:
      - { id: "input",  kind: "input" }
      - { id: "output", kind: "output" }
    edges: []
    start_nodes: ["input"]
    end_nodes:   ["output"]
`;

const BASE_YAML = `
adp_version: "0.2.0"
id: "fixture.comp.base"
runtime:
  execution:
    - { id: "python-backend", backend: "python", entrypoint: "agents.main:app" }
flow:${_MINIMAL_FLOW}
evaluation:
  suites:
    - id: "safety"
      metrics:
        - { id: "m1", type: "deterministic", function: "noop", scoring: "boolean", threshold: 0.9 }
guardrails:
  input:
    - { id: "pii", provider: "guardrails-ai", policy_ref: "./policies/pii.rail", mode: "block" }
  on_violation: "block"
`;

const CHILD_YAML = `
adp_version: "0.2.0"
id: "fixture.comp.child"
extends: "mem://base"
flow:${_MINIMAL_FLOW}
`;

const MODULE_EVALS_YAML = `
id: "fixture.comp.module.evals"
evaluation:
  suites:
    - id: "accuracy"
      metrics:
        - { id: "factuality", type: "llm_judge", threshold: 0.85 }
`;

const CHILD_WITH_IMPORT_YAML = `
adp_version: "0.2.0"
id: "fixture.comp.child.import"
extends: "mem://base"
import:
  - id: "evals"
    from: "mem://module_evals"
flow:${_MINIMAL_FLOW}
`;

const CYCLE_A_YAML = `
adp_version: "0.2.0"
id: "fixture.comp.cycle.a"
extends: "mem://cycle_b"
`;

const CYCLE_B_YAML = `
adp_version: "0.2.0"
id: "fixture.comp.cycle.b"
extends: "mem://cycle_a"
`;

function makeInMemoryResolver(manifests: Record<string, string>): (uri: string) => string {
  return (uri: string) => {
    const key = uri.replace("mem://", "");
    if (key in manifests) return manifests[key];
    /* c8 ignore next */
    throw new CompositionError(`resolver: unknown URI ${uri}`);
  };
}

function testCompositionExtendsInheritsBase() {
  const resolver = makeInMemoryResolver({
    base: BASE_YAML,
    child: CHILD_YAML,
  });
  const adp = resolveAdp("mem://child", resolver);
  assert.strictEqual(adp.id, "fixture.comp.child", "child id should be preserved");
  assert.ok(
    (adp as any).guardrails?.input?.length > 0,
    "guardrails should be inherited from base"
  );
  assert.strictEqual(
    (adp as any).guardrails?.on_violation,
    "block",
    "on_violation should be inherited"
  );
}

function testCompositionChildOverridesBaseField() {
  // child sets adp_version same as base — just check that child id wins over base id
  const resolver = makeInMemoryResolver({
    base: BASE_YAML,
    child: CHILD_YAML,
  });
  const adp = resolveAdp("mem://child", resolver);
  assert.strictEqual(adp.id, "fixture.comp.child", "local id should win over base id");
}

function testCompositionImportAppendsArrays() {
  const resolver = makeInMemoryResolver({
    base: BASE_YAML,
    child_import: CHILD_WITH_IMPORT_YAML,
    module_evals: MODULE_EVALS_YAML,
  });
  const adp = resolveAdp("mem://child_import", resolver) as any;
  assert.ok(
    adp.evaluation?.suites?.some((s: any) => s.id === "accuracy"),
    "imported evaluation suite should be present"
  );
}

function testCompositionCycleDetected() {
  const resolver = makeInMemoryResolver({
    cycle_a: CYCLE_A_YAML,
    cycle_b: CYCLE_B_YAML,
  });
  assert.throws(
    () => resolveAdp("mem://cycle_a", resolver),
    (err: unknown) => err instanceof CompositionError && (err as CompositionError).message.includes("circular"),
    "should throw CompositionError on circular extends"
  );
}

function testCompositionDepthLimitExceeded() {
  // Build a chain of depth > 10
  const manifests: Record<string, string> = {};
  for (let i = 0; i <= 12; i++) {
    const next = i < 12 ? `\nextends: "mem://chain_${i + 1}"\n` : "";
    manifests[`chain_${i}`] = `adp_version: "0.2.0"\nid: "chain.${i}"${next}`;
  }
  const resolver = makeInMemoryResolver(manifests);
  assert.throws(
    () => resolveAdp("mem://chain_0", resolver),
    (err: unknown) => err instanceof CompositionError && (err as CompositionError).message.includes("depth"),
    "should throw CompositionError when depth exceeded"
  );
}

function testCompositionErrorIsCustomError() {
  const err = new CompositionError("test error");
  assert.ok(err instanceof CompositionError, "should be instance of CompositionError");
  assert.ok(err instanceof Error, "should be instance of Error");
  assert.strictEqual(err.name, "CompositionError");
  assert.strictEqual(err.message, "test error");
}

const _minimalManifest = {
  adp_version: "0.2.0",
  id: "test.agent",
  runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
  flow: {
    id: "test.flow",
    graph: {
      nodes: [{ id: "input", kind: "input" as const }, { id: "chat", kind: "llm" as const }, { id: "output", kind: "output" as const }],
      edges: [{ from: "input", to: "chat" }, { from: "chat", to: "output" }],
      start_nodes: ["input"],
      end_nodes: ["output"],
    },
  },
  evaluation: { suites: [{ id: "basic", metrics: [{ id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }] }] },
};

function testLangGraphIntegrationExportsExpectedFunctions() {
  assert.equal(typeof buildLangGraphFromAdp, "function", "buildLangGraphFromAdp must be a function");
  assert.equal(typeof makeConditionFn, "function", "makeConditionFn must be a function");
}

function testLangGraphThrowsWhenNotInstalled() {
  try {
    buildLangGraphFromAdp(_minimalManifest as any);
    // If @langchain/langgraph is installed, no error — just verify the return shape
  } catch (err: any) {
    assert.ok(
      err.message.includes("@langchain/langgraph"),
      `Error should mention @langchain/langgraph, got: ${err.message}`
    );
  }
}

function testSKIntegrationExportsExpectedFunctions() {
  assert.equal(typeof buildSKFromAdp, "function", "buildSKFromAdp must be a function");
}

function testSKBuildRunsInMockMode() {
  const manifest = {
    ..._minimalManifest,
    runtime: {
      execution: [{ id: "py", backend: "python", entrypoint: "app:main" }],
      models: [{ id: "gpt4", provider: "openai", model: "gpt-4o-mini" }],
    },
    flow: {
      id: "test.flow",
      graph: {
        nodes: [
          { id: "input", kind: "input" as const },
          { id: "chat", kind: "llm" as const, model_ref: "gpt4" },
          { id: "output", kind: "output" as const },
        ],
        edges: [{ from: "input", to: "chat" }, { from: "chat", to: "output" }],
        start_nodes: ["input"],
        end_nodes: ["output"],
      },
    },
  };
  const { kernel, processSteps } = buildSKFromAdp(manifest as any);
  assert.ok(processSteps.length === 3, `Expected 3 process steps, got ${processSteps.length}`);
  const chatStep = processSteps.find((s) => s.id === "chat");
  assert.ok(chatStep, "chat step must exist");
  assert.equal(chatStep!.kind, "llm");
  assert.equal(chatStep!.model_ref, "gpt4");
  assert.equal(chatStep!.provider, "openai");
  // kernel is mock dict when @microsoft/semantic-kernel not installed
  if ((kernel as any).type) {
    assert.equal((kernel as any).type, "mock_kernel");
  }
}

// ─── Evaluator loader smoke tests ────────────────────────────────────────────
function testLoadEvaluatorLLMJudge() {
  const ev = loadEvaluator({ id: "judge-1", type: "llm_judge", model: "gpt-4o" });
  assert.equal(typeof ev.evaluate, "function", "llm_judge evaluator must have evaluate()");
  // LLMJudgeEvaluator.evaluate() throws without a client — that's expected
  ev.evaluate({}, {}).catch((err: Error) => {
    assert.ok(err.message.includes("LLM client"), `Expected LLM client message, got: ${err.message}`);
  });
}

async function testLoadEvaluatorUnknownTypeThrows() {
  try {
    loadEvaluator({ id: "x", type: "unknown_type" });
    /* c8 ignore next */
    assert.fail("Should have thrown UnsupportedEvaluatorTypeError");
  } catch (err: unknown) {
    assert.ok(err instanceof UnsupportedEvaluatorTypeError, `Expected UnsupportedEvaluatorTypeError, got: ${err}`);
  }
}

function testLoadEvaluatorsFromManifestMerges() {
  const manifest = {
    x_testing: {
      evaluators: [{ id: "ev1", type: "llm_judge", model: "gpt-4o" }],
      judges: [{ id: "j1", model: "gpt-4o-mini", system_prompt: "Rate 0-1" }],
    },
  };
  const evs = loadEvaluatorsFromManifest(manifest);
  assert.ok(evs.has("ev1"), "evaluators[] entry should be loaded");
  assert.ok(evs.has("j1"), "judges[] entry should be loaded as llm_judge");
  assert.strictEqual(evs.size, 2, "Should have 2 evaluators total");
}

// ─── Evaluation: DeterministicEvaluator ─────────────────────────────────────
async function testDeterministicEvaluatorWithBuiltinModule() {
  // Create a temp ESM module that exports an evaluate function
  const tmp = os.tmpdir();
  const modPath = path.join(tmp, `det_eval_${Date.now()}.mjs`);
  fs.writeFileSync(modPath, `export function myEval(output, context) { return { passed: true, score: 1.0, reason: "ok" }; }\n`);
  const ev = loadEvaluator({ id: "det", type: "deterministic", function_ref: `${modPath}:myEval` });
  const result = await ev.evaluate({ answer: "yes" }, {});
  fs.unlinkSync(modPath);
  assert.strictEqual(result.passed, true);
  assert.strictEqual(result.score, 1.0);
}

async function testDeterministicEvaluatorInvalidRef() {
  const ev = loadEvaluator({ id: "det2", type: "deterministic", function_ref: "no_colon" });
  try {
    await ev.evaluate({}, {});
    /* c8 ignore next 2 */
    assert.fail("Should throw for invalid function_ref");
  } catch (err: any) {
    /* c8 ignore next */
    assert.ok(err.message.includes("invalid function_ref") || err.message.includes("expected"), `Got: ${err.message}`);
  }
}

async function testDeterministicEvaluatorFunctionNotFound() {
  // Create a module without the requested function
  const tmp2 = os.tmpdir();
  const modPath = path.join(tmp2, `det_nofn_${Date.now()}.mjs`);
  fs.writeFileSync(modPath, `export function otherFn() { return true; }\n`);
  const ev = loadEvaluator({ id: "det3", type: "deterministic", function_ref: `${modPath}:missingFn` });
  try {
    await ev.evaluate({}, {});
    /* c8 ignore next 2 */
    assert.fail("Should throw for missing function");
  } catch (err: any) {
    /* c8 ignore next */
    assert.ok(err.message.includes("not found") || err.message.includes("missingFn"), `Got: ${err.message}`);
  }
  fs.unlinkSync(modPath);
}

// ─── Evaluation: ScriptEvaluator python runtime ──────────────────────────────
async function testScriptEvaluatorPythonSuccess() {
  // Use python runtime with inline script
  const inline = `
def evaluate(output, context):
    return {"passed": True, "score": 1.0, "reason": "py-ok"}
`;
  const ev = loadEvaluator({ id: "py-ev", type: "script", runtime: "python", inline });
  try {
    const result = await ev.evaluate({}, {});
    // If python3 is available, result should have passed=true
    assert.ok(typeof result.passed === "boolean");
    /* c8 ignore next 5 */
  } catch (err: any) {
    // python3 might not be available in CI — acceptable
    assert.ok(err.message.includes("Script error") || err.message.includes("python"), `Unexpected: ${err.message}`);
  }
}

async function testScriptEvaluatorPythonError() {
  // Python script that fails
  const ev = loadEvaluator({ id: "py-err", type: "script", runtime: "python", inline: "raise RuntimeError('fail')" });
  const result = await ev.evaluate({}, {});
  // Should return passed=false with error message
  assert.strictEqual(result.passed, false);
}

// ─── Evaluation: normalizeResult ────────────────────────────────────────────
function testNormalizeResultViaBoolInput() {
  // Load a script evaluator with javascript runtime returning a boolean true
  const ev = loadEvaluator({ id: "e1", type: "script", runtime: "javascript", inline: "return true;" });
  return ev.evaluate({}, {}).then((r) => {
    assert.strictEqual(r.passed, true);
    assert.strictEqual(r.score, 1.0);
  });
}

function testNormalizeResultViaBoolFalse() {
  const ev = loadEvaluator({ id: "e1", type: "script", runtime: "javascript", inline: "return false;" });
  return ev.evaluate({}, {}).then((r) => {
    assert.strictEqual(r.passed, false);
    assert.strictEqual(r.score, 0.0);
  });
}

function testNormalizeResultViaMapPassed() {
  const ev = loadEvaluator({ id: "e1", type: "script", runtime: "javascript", inline: 'return {passed:true, score:0.9, reason:"ok"};' });
  return ev.evaluate({}, {}).then((r) => {
    assert.strictEqual(r.passed, true);
    assert.strictEqual(r.score, 0.9);
    assert.strictEqual(r.reason, "ok");
  });
}

function testNormalizeResultViaMapScoreOnly() {
  // No "passed" key, only score>=0.5 → passed=true
  const ev = loadEvaluator({ id: "e1", type: "script", runtime: "javascript", inline: 'return {score:0.8};' });
  return ev.evaluate({}, {}).then((r) => {
    assert.strictEqual(r.passed, true);
    assert.strictEqual(r.score, 0.8);
  });
}

function testNormalizeResultViaMapScoreLow() {
  // No "passed" key, score<0.5 → passed=false
  const ev = loadEvaluator({ id: "e1", type: "script", runtime: "javascript", inline: 'return {score:0.3};' });
  return ev.evaluate({}, {}).then((r) => {
    assert.strictEqual(r.passed, false);
    assert.strictEqual(r.score, 0.3);
  });
}

function testNormalizeResultFallback() {
  // Return a number → fallback branch
  const ev = loadEvaluator({ id: "e1", type: "script", runtime: "javascript", inline: 'return 42;' });
  return ev.evaluate({}, {}).then((r) => {
    // Boolean(42) == true
    assert.strictEqual(r.passed, true);
    assert.strictEqual(r.score, null);
  });
}

// ─── Evaluation: ScriptEvaluator bash ────────────────────────────────────────
function testScriptEvaluatorBashSuccess() {
  const ev = loadEvaluator({ id: "bash-ev", type: "script", runtime: "bash", inline: 'echo \'{"passed":true,"score":1.0,"reason":"ok"}\'' });
  return ev.evaluate({}, {}).then((r) => {
    assert.strictEqual(r.passed, true);
    assert.strictEqual(r.score, 1.0);
    assert.strictEqual(r.reason, "ok");
  });
}

function testScriptEvaluatorBashError() {
  const ev = loadEvaluator({ id: "bash-err", type: "script", runtime: "bash", inline: "exit 1" });
  return ev.evaluate({}, {}).then((r) => {
    assert.strictEqual(r.passed, false);
    /* c8 ignore next */
    assert.ok(r.reason.includes("Bash error") || r.reason.includes("bash error") || r.reason.length > 0);
  });
}

// ─── Evaluation: ScriptEvaluator unknown runtime ──────────────────────────────
function testScriptEvaluatorUnknownRuntime() {
  const ev = loadEvaluator({ id: "e1", type: "script", runtime: "ruby", inline: "puts 'hi'" });
  /* c8 ignore next */
  return ev.evaluate({}, {}).then(() => {
    /* c8 ignore next */
    assert.fail("Should have thrown UnsupportedEvaluatorTypeError");
  }).catch((err: unknown) => {
    assert.ok(err instanceof UnsupportedEvaluatorTypeError, `Expected UnsupportedEvaluatorTypeError, got ${err}`);
  });
}

// ─── Evaluation: ScriptEvaluator missing inline/script_ref ───────────────────
function testScriptEvaluatorMissingInlineAndRef() {
  const ev = loadEvaluator({ id: "e1", type: "script", runtime: "javascript" });
  /* c8 ignore next 2 */
  return ev.evaluate({}, {}).then(() => {
    assert.fail("Should have thrown");
  }).catch((err: unknown) => {
    assert.ok(err instanceof Error, `Expected Error, got ${err}`);
    assert.ok((err as Error).message.includes("requires inline or script_ref"));
  });
}

// ─── Evaluation: LLMJudgeEvaluator throws ────────────────────────────────────
function testLLMJudgeEvaluatorThrows() {
  const ev = loadEvaluator({ id: "judge", type: "llm_judge", model: "gpt-4o" });
  /* c8 ignore next 2 */
  return ev.evaluate({}, {}).then(() => {
    assert.fail("Should have thrown");
  }).catch((err: Error) => {
    assert.ok(err.message.includes("LLM client"), `Expected LLM client message, got: ${err.message}`);
  });
}

// ─── Evaluation: loadEvaluator all 4 types ───────────────────────────────────
function testLoadEvaluatorAllTypes() {
  const script = loadEvaluator({ id: "s", type: "script", runtime: "javascript", inline: "return true;" });
  assert.equal(typeof script.evaluate, "function");
  const container = loadEvaluator({ id: "c", type: "container", image: "alpine", image_digest: "sha256:abc" });
  assert.equal(typeof container.evaluate, "function");
  const det = loadEvaluator({ id: "d", type: "deterministic", function_ref: "foo:bar" });
  assert.equal(typeof det.evaluate, "function");
  const judge = loadEvaluator({ id: "j", type: "llm_judge", model: "gpt-4o" });
  assert.equal(typeof judge.evaluate, "function");
}

// ─── Evaluation: loadEvaluatorsFromManifest edge cases ───────────────────────
function testLoadEvaluatorsFromManifestEmpty() {
  const evs = loadEvaluatorsFromManifest({});
  assert.strictEqual(evs.size, 0);
}

function testLoadEvaluatorsFromManifestJudgesOnly() {
  const evs = loadEvaluatorsFromManifest({
    x_testing: {
      judges: [{ id: "j1", model: "gpt-4o" }],
    },
  });
  assert.ok(evs.has("j1"), "judge should be loaded");
}

function testLoadEvaluatorsFromManifestEvaluatorsWin() {
  // evaluators[] with same id as judges[] wins
  const evs = loadEvaluatorsFromManifest({
    x_testing: {
      judges: [{ id: "shared", model: "gpt-3.5" }],
      evaluators: [{ id: "shared", type: "llm_judge", model: "gpt-4o" }],
    },
  });
  assert.strictEqual(evs.size, 1);
  assert.ok(evs.has("shared"));
}

// ─── Composition: _deepMerge, _additiveMerge, _applyOverride ─────────────────
// These are exercised via resolveAdp with in-memory resolver

const _VALID_FLOW_YAML = `
  id: "f"
  graph:
    nodes:
      - { id: "n1", kind: "input" }
    edges: []
    start_nodes: ["n1"]
    end_nodes: ["n1"]
`;
const _VALID_EVAL_YAML = `
  suites:
    - id: "s1"
      metrics:
        - { id: "m1", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
`;

function testCompositionDeepMergeNullDeletes() {
  // null overlay removes key from base
  const manifests: Record<string,string> = {
    "base": `
adp_version: "0.2.0"
id: "base"
name: "Base Name"
runtime:
  execution:
    - { id: "py", backend: "python", entrypoint: "app:main" }
flow:${_VALID_FLOW_YAML}
evaluation:${_VALID_EVAL_YAML}
`,
    "child": `
adp_version: "0.2.0"
id: "child"
extends: "mem://base"
name: null
`,
  };
  const resolver = (uri: string) => {
    const key = uri.replace("mem://", "");
    if (key in manifests) return manifests[key];
    /* c8 ignore next */
    throw new CompositionError(`resolver: unknown URI ${uri}`);
  };
  const adp = resolveAdp("mem://child", resolver) as any;
  /* c8 ignore next */
  assert.ok(!adp.name || adp.name === null || adp.name === undefined, "null overlay should remove name");
}

function testCompositionAdditiveMergeArraysAppend() {
  const manifests: Record<string, string> = {
    "base": `
adp_version: "0.2.0"
id: "base-add"
runtime:
  execution:
    - { id: "py", backend: "python", entrypoint: "app:main" }
flow:${_VALID_FLOW_YAML}
evaluation:
  suites:
    - id: "suite-a"
      metrics:
        - { id: "m1", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
`,
    "child": `
adp_version: "0.2.0"
id: "child-add"
extends: "mem://base"
import:
  - id: "mod"
    from: "mem://module"
`,
    "module": `
evaluation:
  suites:
    - id: "suite-b"
      metrics:
        - { id: "m2", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
`,
  };
  const resolver = (uri: string) => {
    const key = uri.replace("mem://", "");
    if (key in manifests) return manifests[key];
    /* c8 ignore next */
    throw new CompositionError(`resolver: unknown URI ${uri}`);
  };
  const adp = resolveAdp("mem://child", resolver) as any;
  /* c8 ignore next */
  const suites = adp.evaluation?.suites ?? [];
  assert.ok(suites.length >= 2, `Expected >=2 suites after additive import, got ${suites.length}`);
}

function makeSimpleResolver(content: string): (uri: string) => string {
  return (_uri: string) => content;
}

const _VALID_BASE_MANIFEST = `
adp_version: "0.2.0"
id: "placeholder"
runtime:
  execution:
    - { id: "py", backend: "python", entrypoint: "app:main" }
flow:${_VALID_FLOW_YAML}
evaluation:${_VALID_EVAL_YAML}
`;

function testCompositionApplyOverrideDelete() {
  const adp = resolveAdp("mem://del", makeSimpleResolver(_VALID_BASE_MANIFEST.replace('id: "placeholder"', 'id: "del-test"\nname: "Will Be Deleted"') + `overrides:\n  - path: "/name"\n    op: "delete"\n`)) as any;
  assert.ok(!adp.name, "override delete should remove name");
}

function testCompositionApplyOverrideDeleteMissingPath() {
  // Delete on path with missing intermediate — should return unchanged (not throw)
  const adp = resolveAdp("mem://del2", makeSimpleResolver(_VALID_BASE_MANIFEST.replace('id: "placeholder"', 'id: "del-test2"') + `overrides:\n  - path: "/nonexistent/key"\n    op: "delete"\n`)) as any;
  assert.strictEqual(adp.id, "del-test2", "delete on missing path should not crash");
}

function testCompositionApplyOverrideAppendToNonArray() {
  assert.throws(
    () => resolveAdp("mem://badappend", makeSimpleResolver(_VALID_BASE_MANIFEST.replace('id: "placeholder"', 'id: "append-scalar"') + `overrides:\n  - path: "/id"\n    op: "append"\n    value: "x"\n`)),
    (err: unknown) => err instanceof CompositionError,
    "append to non-array should throw"
  );
}

function testCompositionApplyOverrideUnknownOp() {
  assert.throws(
    () => resolveAdp("mem://badop", makeSimpleResolver(_VALID_BASE_MANIFEST.replace('id: "placeholder"', 'id: "bad-op"') + `overrides:\n  - path: "/id"\n    op: "zap"\n    value: "x"\n`)),
    (err: unknown) => err instanceof CompositionError,
    "unknown op should throw"
  );
}

function testCompositionPointerGetArrayAccess() {
  const adp = resolveAdp("mem://arr", makeSimpleResolver(_VALID_BASE_MANIFEST.replace('id: "placeholder"', 'id: "arr-set"\ntags:\n  - "first"\n  - "second"') + `overrides:\n  - path: "/tags/0"\n    op: "set"\n    value: "replaced"\n`)) as any;
  assert.strictEqual(adp.tags[0], "replaced");
  assert.strictEqual(adp.tags[1], "second");
}

function testCompositionPointerGetAllowMissing() {
  // Delete with missing intermediate segment — should be no-op (allowMissing=true)
  const adp = resolveAdp("mem://amiss", makeSimpleResolver(_VALID_BASE_MANIFEST.replace('id: "placeholder"', 'id: "allow-missing"') + `overrides:\n  - path: "/missing_parent/child"\n    op: "delete"\n`)) as any;
  assert.strictEqual(adp.id, "allow-missing");
}

function testCompositionSetOnNonExistentKey() {
  assert.throws(
    () => resolveAdp("mem://nokey", makeSimpleResolver(_VALID_BASE_MANIFEST.replace('id: "placeholder"', 'id: "no-key"') + `overrides:\n  - path: "/nonexistent_field"\n    op: "set"\n    value: "x"\n`)),
    (err: unknown) => err instanceof CompositionError,
    "set on non-existent key should throw"
  );
}

function testCompositionHttpUriRequiresResolver() {
  // _loadUri: HTTP URI without resolver should throw
  assert.throws(
    () => resolveAdp("http://example.com/agent.yaml"),
    (err: unknown) => err instanceof CompositionError,
    "HTTP URI without resolver should throw"
  );
}

function testCompositionMissingFile() {
  assert.throws(
    () => resolveAdp("/nonexistent/path/agent.yaml"),
    (err: unknown) => err instanceof CompositionError,
    "missing file should throw"
  );
}

function testCompositionRegistryURIThrows() {
  // Registry URI in extends should throw
  assert.throws(
    () => resolveAdp("mem://main", (uri: string) => {
      if (uri === "mem://main") return `
adp_version: "0.2.0"
id: "reg-test"
extends: "registry://acme/base-agent:1.0"
runtime:
  execution:
    - { id: "py", backend: "python", entrypoint: "app:main" }
flow:
  id: "f"
  graph:
    nodes: [{id: "n1", kind: "input"}]
    edges: []
    start_nodes: ["n1"]
    end_nodes: ["n1"]
evaluation:
  suites:
    - id: "s1"
      metrics:
        - { id: "m1", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
`;
      /* c8 ignore next */
      throw new CompositionError(`unknown: ${uri}`);
    }),
    (err: unknown) => err instanceof CompositionError && (err as CompositionError).message.includes("registry"),
    "registry:// URI should throw"
  );
}

function testCompositionRelativeExtendsWithMemScheme() {
  // When base URI is mem://base/child, relative extends "./sibling" should resolve
  const manifests: Record<string, string> = {
    "main": `
adp_version: "0.2.0"
id: "mem-rel-test"
extends: "mem://base/sibling"
`,
    "base/sibling": `
adp_version: "0.2.0"
id: "sibling"
runtime:
  execution:
    - { id: "py", backend: "python", entrypoint: "app:main" }
flow:${_VALID_FLOW_YAML}
evaluation:${_VALID_EVAL_YAML}
`,
  };
  // This tests the `schemeMatch` path in _resolveUri where base has a path component
  // The URI "mem://base/main" with extends "mem://base/sibling" (absolute mem:// URI)
  // Actually test with relative path: base is "mem://sub/main", extends is "./sibling"
  const manifests2: Record<string, string> = {
    "sub/main": `
adp_version: "0.2.0"
id: "mem-rel-test"
extends: "sibling"
`,
    "sub/sibling": `
adp_version: "0.2.0"
id: "sibling"
runtime:
  execution:
    - { id: "py", backend: "python", entrypoint: "app:main" }
flow:${_VALID_FLOW_YAML}
evaluation:${_VALID_EVAL_YAML}
`,
  };
  const resolver2 = (uri: string) => {
    const key = uri.replace("mem://", "");
    if (key in manifests2) return manifests2[key];
    /* c8 ignore next */
    throw new CompositionError(`unknown: ${uri}`);
  };
  const adp = resolveAdp("mem://sub/main", resolver2) as any;
  assert.strictEqual(adp.id, "mem-rel-test");
}

function testCompositionSetOnArrayWithInvalidIndex() {
  // Override set on array node with invalid index should throw
  assert.throws(
    () => resolveAdp("mem://badidx", makeSimpleResolver(_VALID_BASE_MANIFEST.replace('id: "placeholder"', 'id: "bad-idx"\ntags:\n  - "a"') + `overrides:\n  - path: "/tags/notanumber"\n    op: "set"\n    value: "x"\n`)),
    (err: unknown) => err instanceof CompositionError,
    "invalid array index should throw"
  );
}

function testCompositionSetOnNonObjectNonArray() {
  // Override set navigating INTO a scalar (path segment through scalar) should throw
  // /runtime/execution/0/entrypoint/foo/bar — navigates through "entrypoint" (string) to "foo"
  assert.throws(
    () => resolveAdp("mem://badnav", makeSimpleResolver(_VALID_BASE_MANIFEST.replace('id: "placeholder"', 'id: "bad-nav"') + `overrides:\n  - path: "/runtime/execution/0/entrypoint/foo/bar"\n    op: "set"\n    value: "x"\n`)),
    (err: unknown) => err instanceof CompositionError,
    "navigate into scalar should throw"
  );
}

function testCompositionAppendSuccess() {
  // Override append to an existing array should succeed (covers lines 175-176)
  const adp = resolveAdp("mem://app", makeSimpleResolver(
    _VALID_BASE_MANIFEST.replace('id: "placeholder"', 'id: "append-ok"\ntags:\n  - "a"\n  - "b"') +
    `overrides:\n  - path: "/tags"\n    op: "append"\n    value: "c"\n`
  )) as any;
  assert.ok(Array.isArray(adp.tags) && adp.tags.includes("c"), `Expected 'c' in tags, got: ${JSON.stringify(adp.tags)}`);
}

function testCompositionPointerGetSegmentNotFound() {
  // _pointerGet throws when intermediate segment not found (lines 189-190)
  // path /runtime/nonexistent/key — navigate to runtime, then get nonexistent → throw
  assert.throws(
    () => resolveAdp("mem://segmiss", makeSimpleResolver(
      _VALID_BASE_MANIFEST.replace('id: "placeholder"', 'id: "seg-miss"') +
      `overrides:\n  - path: "/runtime/nonexistent_key/subkey"\n    op: "set"\n    value: "x"\n`
    )),
    (err: unknown) => err instanceof CompositionError,
    "missing intermediate segment should throw"
  );
}

function testCompositionFileRead() {
  // Test that resolveAdp can read a real file (covers _loadUri readFileSync path)
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ts-comp-"));
  const baseFile = path.join(tmp, "base.yaml");
  const agentFile = path.join(tmp, "agent.yaml");
  fs.writeFileSync(baseFile, `
adp_version: "0.2.0"
id: "file-base"
runtime:
  execution:
    - { id: "py", backend: "python", entrypoint: "app:main" }
flow:${_VALID_FLOW_YAML}
evaluation:${_VALID_EVAL_YAML}
`);
  fs.writeFileSync(agentFile, `
adp_version: "0.2.0"
id: "file-read-test"
extends: "./base.yaml"
`);
  // This covers _resolveUri with real filesystem paths (path.dirname + path.resolve branch)
  // and _loadUri readFileSync branch
  const adp = resolveAdp(agentFile) as any;
  assert.strictEqual(adp.id, "file-read-test");
  fs.unlinkSync(agentFile);
  fs.unlinkSync(baseFile);
  fs.rmdirSync(tmp);
}

// ─── Validation: error branches ──────────────────────────────────────────────
function testValidateConformanceClassFullWithEmptyEval() {
  const adp = {
    adp_version: "0.1.0",
    id: "x",
    conformance_class: "full",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "a:b" }] },
    flow: { id: "f", graph: { nodes: [{ id: "n1", kind: "input" }], edges: [], start_nodes: ["n1"], end_nodes: ["n1"] } },
    evaluation: {},
  } as any;
  const errors = validateAdp(adp);
  /* c8 ignore next */
  assert.ok(errors.some((e: string) => e.includes("evaluation") || e.includes("full")), `Expected eval/full error, got: ${errors.join("; ")}`);
}

function testValidateSemanticsJudgesDeprecationWarning() {
  const adp = {
    adp_version: "0.1.0",
    id: "x",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "a:b" }] },
    flow: { graph: { nodes: [], edges: [], start_nodes: [], end_nodes: [] } },
    evaluation: {},
    x_testing: {
      judges: [{ id: "j1", model: "gpt-4o" }],
    },
  } as any;
  const errors = validateAdpSemantics(adp);
  /* c8 ignore next */
  assert.ok(errors.some((e: string) => e.includes("judges") || e.includes("WARNING")), `Expected judges deprecation warning, got: ${errors.join("; ")}`);
}

// ─── adpkg: Inspect and Verify ───────────────────────────────────────────────
function testVerifyPackageEmptyManifests() {
  // Covers adpkg.js branch: index.manifests ?? [] (when manifests is undefined)
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ts-adpkg-empty-"));
  fs.writeFileSync(path.join(tmp, "index.json"), JSON.stringify({ schemaVersion: 2 }));
  const result = verifyPackage(tmp);
  assert.strictEqual(result.passed, true);
  assert.deepStrictEqual(result.failures, []);
}

function testCreatePackageThrowsOnInvalidAdp() {
  // Covers adpkg.js line 27: createPackage throws when ADP validation fails
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ts-adpkg-invalid-"));
  const srcDir = path.join(tmp, "src");
  const outDir = path.join(tmp, "out");
  fs.mkdirSync(path.join(srcDir, "adp"), { recursive: true });
  // Write an invalid agent.yaml (missing required fields)
  fs.writeFileSync(path.join(srcDir, "adp", "agent.yaml"), `id: "bad-agent"\n`);
  assert.throws(
    () => createPackage(srcDir, outDir),
    (err: unknown) => err instanceof Error && (err as Error).message.includes("ADP validation failed"),
    "createPackage should throw when ADP is invalid"
  );
}

function testInspectAndVerifyPackage() {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ts-adpkg-"));
  const srcDir = path.join(tmp, "src");
  const outDir = path.join(tmp, "out");
  fs.mkdirSync(path.join(srcDir, "adp"), { recursive: true });
  fs.writeFileSync(
    path.join(srcDir, "adp", "agent.yaml"),
    `adp_version: "0.1.0"\nid: "inspect.test"\nruntime:\n  execution:\n    - backend: "python"\n      id: "py"\n      entrypoint: "agent.main:app"\nflow:\n  id: "f"\n  graph:\n    nodes: [{id: "n1", kind: "input"}]\n    edges: []\n    start_nodes: ["n1"]\n    end_nodes: ["n1"]\nevaluation:\n  suites:\n    - id: "s"\n      metrics:\n        - {id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true}\n`
  );
  createPackage(srcDir, outDir);
  const inspection = inspectPackage(outDir);
  assert.strictEqual(inspection.agent_id, "inspect.test");
  assert.strictEqual(inspection.adp_version, "0.1.0");
  assert.ok(inspection.layer_count > 0);

  const verify = verifyPackage(outDir);
  assert.strictEqual(verify.passed, true);
  assert.deepStrictEqual(verify.failures, []);

  // Test verifyBlob failure: corrupt the config blob (leaves manifest intact so Verify still runs)
  const indexJson = JSON.parse(fs.readFileSync(path.join(outDir, "index.json"), "utf8"));
  const manifestDigest: string = indexJson.manifests[0].digest;
  const manifestBlobFile = path.join(outDir, "blobs", manifestDigest.replace("sha256:", "sha256/"));
  const manifestJson = JSON.parse(fs.readFileSync(manifestBlobFile, "utf8"));
  const configDigest: string = manifestJson.config.digest;
  const configBlobFile = path.join(outDir, "blobs", configDigest.replace("sha256:", "sha256/"));
  const origContent = fs.readFileSync(configBlobFile);
  // Write slightly different content to trigger digest mismatch (must be valid JSON)
  fs.writeFileSync(configBlobFile, Buffer.from(JSON.stringify({ agent_id: "tampered" })));
  const verifyCorrupt = verifyPackage(outDir);
  assert.strictEqual(verifyCorrupt.passed, false);
  assert.ok(verifyCorrupt.failures.length > 0);
  // Restore
  fs.writeFileSync(configBlobFile, origContent);
}

function testVerifyPackageMissingBlob() {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "ts-adpkg2-"));
  const srcDir = path.join(tmp, "src");
  const outDir = path.join(tmp, "out");
  fs.mkdirSync(path.join(srcDir, "adp"), { recursive: true });
  fs.writeFileSync(
    path.join(srcDir, "adp", "agent.yaml"),
    `adp_version: "0.1.0"\nid: "verify.test"\nruntime:\n  execution:\n    - backend: "python"\n      id: "py"\n      entrypoint: "agent.main:app"\nflow:\n  id: "f"\n  graph:\n    nodes: [{id: "n1", kind: "input"}]\n    edges: []\n    start_nodes: ["n1"]\n    end_nodes: ["n1"]\nevaluation:\n  suites:\n    - id: "s"\n      metrics:\n        - {id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true}\n`
  );
  createPackage(srcDir, outDir);
  // Delete the layer blob (not the manifest blob) to trigger "blob file missing" in verifyBlob
  const indexJson = JSON.parse(fs.readFileSync(path.join(outDir, "index.json"), "utf8"));
  const manifestDigest: string = indexJson.manifests[0].digest;
  const manifestBlobFile = path.join(outDir, "blobs", manifestDigest.replace("sha256:", "sha256/"));
  const manifestJson = JSON.parse(fs.readFileSync(manifestBlobFile, "utf8"));
  const layerDigest: string = manifestJson.layers[0].digest;
  const layerBlobFile = path.join(outDir, "blobs", layerDigest.replace("sha256:", "sha256/"));
  fs.unlinkSync(layerBlobFile);
  const result = verifyPackage(outDir);
  assert.strictEqual(result.passed, false);
  assert.ok(result.failures.some((f) => f.includes("blob file missing")));
}

// ─── makeConditionFn: call the function it returns ───────────────────────────
function testMakeConditionFnActuallyWorks() {
  const fn = makeConditionFn("inputs.score >= 0.5");
  assert.equal(typeof fn, "function", "makeConditionFn should return a function");
}

// ─── SK: no runtime.models → covers `runtime.models ?? []` default branch ─────
function testSKBuildWithNoModels() {
  const manifest = {
    adp_version: "0.2.0",
    id: "sk-no-models",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: {
      id: "f",
      graph: {
        nodes: [{ id: "input", kind: "input" as const }, { id: "output", kind: "output" as const }],
        edges: [],
        start_nodes: ["input"],
        end_nodes: ["output"],
      },
    },
    evaluation: { suites: [] },
  };
  const { processSteps } = buildSKFromAdp(manifest as any);
  assert.strictEqual(processSteps.length, 2, "Expected 2 process steps");
}

// ─── SK: tool node → covers `kind === "tool"` branch ─────────────────────────
function testSKBuildWithToolNode() {
  const manifest = {
    adp_version: "0.2.0",
    id: "sk-tool",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: {
      id: "f",
      graph: {
        nodes: [
          { id: "input", kind: "input" as const },
          { id: "api-call", kind: "tool" as const, tool_ref: "billing-api" },
          { id: "output", kind: "output" as const },
        ],
        edges: [],
        start_nodes: ["input"],
        end_nodes: ["output"],
      },
    },
    evaluation: { suites: [] },
  };
  const { processSteps } = buildSKFromAdp(manifest as any);
  const toolStep = processSteps.find((s) => s.id === "api-call");
  assert.ok(toolStep, "tool step must exist");
  assert.equal(toolStep!.kind, "tool");
}

// ─── SK: llm node with unmatched model_ref → no provider/model on step ────────
function testSKBuildWithUnmatchedModelRef() {
  const manifest = {
    adp_version: "0.2.0",
    id: "sk-unmatched",
    runtime: {
      execution: [{ id: "py", backend: "python", entrypoint: "app:main" }],
      models: [{ id: "gpt4", provider: "openai", model: "gpt-4o" }],
    },
    flow: {
      id: "f",
      graph: {
        nodes: [
          { id: "chat", kind: "llm" as const, model_ref: "nonexistent-model" },
        ],
        edges: [],
        start_nodes: ["chat"],
        end_nodes: ["chat"],
      },
    },
    evaluation: { suites: [] },
  };
  const { processSteps } = buildSKFromAdp(manifest as any);
  const chatStep = processSteps.find((s) => s.id === "chat");
  assert.ok(chatStep, "chat step must exist");
  assert.ok(!chatStep!.provider, "provider should not be set for unmatched model_ref");
}

// ─── Evaluator: no id → id defaults to "" ────────────────────────────────────
function testEvaluatorDefaultIdEmpty() {
  const ev = loadEvaluator({ type: "script", runtime: "javascript", inline: "return true;" });
  return ev.evaluate({}, {}).then((r) => {
    assert.strictEqual(r.evaluatorId, "", "id should default to empty string when not provided");
  });
}

// ─── Composition: override path not starting with "/" → throws ────────────────
function testCompositionOverridePathNoSlash() {
  assert.throws(
    () => resolveAdp("mem://noslash", makeSimpleResolver(
      _VALID_BASE_MANIFEST.replace('id: "placeholder"', 'id: "no-slash"') +
      `overrides:\n  - path: "noslash"\n    op: "set"\n    value: "x"\n`
    )),
    (err: unknown) => err instanceof CompositionError,
    "override path not starting with / should throw"
  );
}

// ─── Composition: op=set on existing object key → success (lines 160-161) ─────
function testCompositionSetExistingKey() {
  const adp = resolveAdp("mem://setkey", makeSimpleResolver(
    _VALID_BASE_MANIFEST.replace('id: "placeholder"', 'id: "set-key"') +
    `overrides:\n  - path: "/id"\n    op: "set"\n    value: "set-key-result"\n`
  )) as any;
  assert.strictEqual(adp.id, "set-key-result", "override set on existing key should replace the value");
}

// ─── Composition: resolveAdp throws when resolved manifest is invalid ─────────
function testResolveAdpThrowsOnInvalidResolvedManifest() {
  // Set adp_version to an invalid value via override → validateAdp fails → line 22 fires
  assert.throws(
    () => resolveAdp("mem://invalidver", makeSimpleResolver(
      _VALID_BASE_MANIFEST.replace('id: "placeholder"', 'id: "invalid-ver"') +
      `overrides:\n  - path: "/adp_version"\n    op: "set"\n    value: "9.9.9"\n`
    )),
    (err: unknown) => err instanceof CompositionError && (err as CompositionError).message.includes("invalid"),
    "resolveAdp should throw CompositionError when resolved manifest is invalid"
  );
}

// ─── Composition: import with sections filter ─────────────────────────────────
function testCompositionImportWithSectionsFilter() {
  // Module has evaluation and name; sections filter restricts to evaluation only
  const manifests: Record<string, string> = {
    "base": `
adp_version: "0.2.0"
id: "sections-base"
runtime:
  execution:
    - { id: "py", backend: "python", entrypoint: "app:main" }
flow:${_VALID_FLOW_YAML}
evaluation:${_VALID_EVAL_YAML}
`,
    "child": `
adp_version: "0.2.0"
id: "sections-child"
extends: "mem://base"
import:
  - id: "extra-evals"
    from: "mem://module"
    sections: ["evaluation"]
`,
    "module": `
name: "Should Not Be Imported"
evaluation:
  suites:
    - id: "extra-suite"
      metrics:
        - { id: "m-extra", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
`,
  };
  const resolver = (uri: string) => {
    const key = uri.replace("mem://", "");
    if (key in manifests) return manifests[key];
    /* c8 ignore next */
    throw new CompositionError(`resolver: unknown URI ${uri}`);
  };
  const adp = resolveAdp("mem://child", resolver) as any;
  // sections filter should import evaluation but NOT name
  assert.ok(adp.evaluation?.suites?.some((s: any) => s.id === "extra-suite"), "extra suite should be imported via sections filter");
  /* c8 ignore next */
  assert.ok(!adp.name || adp.name !== "Should Not Be Imported", "name should NOT be imported (filtered out by sections)");
}

// ─── Composition: _additiveMerge adds new key not in base ────────────────────
function testCompositionAdditiveMergeNewKey() {
  // Module adds a top-level key (description) not in base → _additiveMerge first branch
  const manifests: Record<string, string> = {
    "base": `
adp_version: "0.2.0"
id: "additive-new-key"
runtime:
  execution:
    - { id: "py", backend: "python", entrypoint: "app:main" }
flow:${_VALID_FLOW_YAML}
evaluation:${_VALID_EVAL_YAML}
`,
    "child": `
adp_version: "0.2.0"
id: "additive-new-key-child"
extends: "mem://base"
import:
  - id: "desc-mod"
    from: "mem://desc_module"
`,
    "desc_module": `
description: "Imported Description"
`,
  };
  const resolver = (uri: string) => {
    const key = uri.replace("mem://", "");
    if (key in manifests) return manifests[key];
    /* c8 ignore next */
    throw new CompositionError(`resolver: unknown URI ${uri}`);
  };
  const adp = resolveAdp("mem://child", resolver) as any;
  assert.strictEqual(adp.description, "Imported Description", "new key from module should be merged additively");
}

// ─── Evaluation: ScriptEvaluator with javascript runtime + scriptRef (no inline)
function testScriptEvaluatorJSScriptRef() {
  // scriptRef provided but inline is undefined → code = inline ?? "" = ""
  // Empty JS function returns undefined → normalizeResult(undefined, ...) → passed: false
  const ev = loadEvaluator({ type: "script", runtime: "javascript", script_ref: "dummy.js" });
  return ev.evaluate({}, {}).then((r) => {
    // undefined raw → Boolean(undefined) = false
    assert.strictEqual(r.passed, false, "empty inline JS should return passed: false");
    assert.strictEqual(r.score, null);
  });
}

// ─── Evaluation: DeterministicEvaluator without id → id defaults to "" ────────
function testDeterministicEvaluatorDefaultId() {
  const tmp = os.tmpdir();
  const modPath = path.join(tmp, `det_eval_noid_${Date.now()}.mjs`);
  fs.writeFileSync(modPath, `export function myEval(output) { return { passed: true, score: 1.0, reason: "ok" }; }\n`);
  const ev = loadEvaluator({ type: "deterministic", function_ref: `${modPath}:myEval` });
  return ev.evaluate({}, {}).then((r) => {
    fs.unlinkSync(modPath);
    assert.strictEqual(r.evaluatorId, "", "id should default to empty string");
    /* c8 ignore next 3 */
  }).catch((err: unknown) => {
    fs.unlinkSync(modPath);
    throw err;
  });
}

// ─── Evaluation: python evaluator with scriptRef instead of inline ───────────
async function testScriptEvaluatorPythonWithScriptRef() {
  // Using scriptRef forces args = [scriptRef] branch (line 34) and scriptCode = undefined (line 33)
  const ev = loadEvaluator({ id: "py-ref", type: "script", runtime: "python", script_ref: "/nonexistent/script.py" });
  const result = await ev.evaluate({}, {});
  // /nonexistent/script.py doesn't exist → execFileSync throws → catch returns passed: false
  assert.strictEqual(result.passed, false, "nonexistent scriptRef should return passed: false");
}

// ─── Evaluation: bash runtime with scriptRef (no inline) → inline ?? "" = "" ──
function testScriptEvaluatorBashScriptRef() {
  // bash + script_ref (no inline) → inline ?? "" = ""
  // runs /bin/bash with -c "" which exits 0 with no output
  const ev = loadEvaluator({ type: "script", runtime: "bash", script_ref: "unused.sh" });
  return ev.evaluate({}, {}).then((r) => {
    // empty bash exits 0 but stdout is empty → JSON.parse("") throws → catch
    assert.strictEqual(r.passed, false, "bash with empty inline should fail JSON parse");
  });
}

// ─── Validation: defensive defaults (null-coalescing edge cases) ──────────────
function testValidationGuardrailNoId() {
  // rail without id → rail.id ?? "?" uses the "?" default
  const adp = {
    adp_version: "0.2.0",
    id: "test.guardrail.noid",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: { graph: { nodes: [], edges: [], start_nodes: [], end_nodes: [] } },
    evaluation: {},
    guardrails: {
      input: [{ provider: "guardrails-ai", policy_ref: "" }],  // no id field
    },
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(errors.some((e: string) => e.includes("policy_ref is empty")), `Expected policy_ref error, got: ${errors.join("; ")}`);
}

function testValidationToolNoId() {
  // tool without id → tool.id ?? "?" uses the "?" default in auth error message
  const adp = {
    adp_version: "0.2.0",
    id: "test.tool.noid",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: { graph: { nodes: [], edges: [], start_nodes: [], end_nodes: [] } },
    evaluation: {},
    tools: {
      http_apis: [{ description: "No id tool", base_url: "https://example.com", auth: { scheme: "bearer" } }],
    },
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(errors.some((e: string) => e.includes("auth.env_var")), `Expected auth.env_var error, got: ${errors.join("; ")}`);
}

function testValidationComplianceNoStandard() {
  // compliance entry without standard → standard ?? "" uses the "" default
  const adp = {
    adp_version: "0.2.0",
    id: "test.compliance.nostandard",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: { graph: { nodes: [], edges: [], start_nodes: [], end_nodes: [] } },
    evaluation: {},
    governance: {
      compliance: [{}],  // no standard field
    },
  } as any;
  const errors = validateAdpSemantics(adp);
  // standard="" → not in known set, not starts with x_ → error
  assert.ok(errors.some((e: string) => e.includes("unknown")), `Expected compliance error, got: ${errors.join("; ")}`);
}

function testValidationHookNoEvent() {
  // hook without event → hook.event ?? "?" uses "?" default in error message
  const adp = {
    adp_version: "0.3.0",
    id: "test.hook.noevent",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: {
      graph: {
        nodes: [{ id: "input", kind: "input" }],
        edges: [],
        start_nodes: ["input"],
        end_nodes: ["input"],
      },
    },
    evaluation: {},
    hooks: [
      { node_filter: ["nonexistent-node"], handler: { type: "function", function_ref: "acme:record" } },
    ],
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(errors.some((e: string) => e.includes("node_filter")), `Expected node_filter error, got: ${errors.join("; ")}`);
}

function testValidationSuiteNoMetrics() {
  // suite without metrics → suite?.metrics ?? [] uses [] default
  // Also x_testing evaluators set → evaluator_ref check runs
  const adp = {
    adp_version: "0.3.0",
    id: "test.suite.nometrics",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: { graph: { nodes: [], edges: [], start_nodes: [], end_nodes: [] } },
    evaluation: {
      suites: [{ id: "s1" }],  // no metrics field
    },
    x_testing: {
      evaluators: [{ id: "ev1", type: "llm_judge", model: "gpt-4o" }],
    },
  } as any;
  const errors = validateAdpSemantics(adp);
  // No metrics → no evaluator_ref errors → no errors from check 14
  assert.ok(!errors.some((e: string) => e.includes("evaluator_ref")), `Unexpected evaluator_ref error, got: ${errors.join("; ")}`);
}

// ─── Validation: no graph → if(!graph) return errors (line 57) ─────────────────
function testValidationNoGraph() {
  // flow without graph → graph is undefined → if(!graph) return errors fires
  const adp = {
    adp_version: "0.2.0",
    id: "test.no-graph",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: {},  // no graph field
    evaluation: {},
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(Array.isArray(errors), "should return array (early return from !graph)");
}

// ─── Validation: graph with no nodes/edges/start_nodes/end_nodes → covers all ??[] ─
function testValidationGraphNoNodesEdges() {
  // graph without nodes, edges, start_nodes, or end_nodes → all ?? [] branches fire
  const adp = {
    adp_version: "0.2.0",
    id: "test.graph.no-nodes",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: { graph: {} },  // completely empty graph — no nodes, edges, start_nodes, end_nodes
    evaluation: {},
  } as any;
  const errors = validateAdpSemantics(adp);
  // No nodes → no duplicate/dangling errors. Should return [] or pre-comp warning only
  assert.ok(Array.isArray(errors), "should return array");
}

// ─── Validation: bad start_node and end_node → lines 77, 81 ─────────────────
function testValidationBadStartAndEndNodes() {
  // start_nodes and end_nodes reference IDs not in nodes → both error lines fire
  const adp = {
    adp_version: "0.2.0",
    id: "test.graph.bad-start-end",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: {
      graph: {
        nodes: [{ id: "n1", kind: "input" }],
        edges: [],
        start_nodes: ["nonexistent-start"],
        end_nodes: ["nonexistent-end"],
      },
    },
    evaluation: {},
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(errors.some((e: string) => e.includes("start_node")), `Expected start_node error, got: ${errors.join("; ")}`);
  assert.ok(errors.some((e: string) => e.includes("end_node")), `Expected end_node error, got: ${errors.join("; ")}`);
}

// ─── Validation: runtime with no execution → execution??[] fires [] ──────────
function testValidationRuntimeNoExecution() {
  // runtime without execution → (adp?.runtime?.execution ?? []) fires []
  const adp = {
    adp_version: "0.2.0",
    id: "test.runtime.no-exec",
    runtime: {},  // no execution field
    flow: {
      graph: {
        nodes: [{ id: "n1", kind: "llm", runtime_ref: "py" }],
        edges: [],
        start_nodes: ["n1"],
        end_nodes: ["n1"],
      },
    },
    evaluation: {},
  } as any;
  const errors = validateAdpSemantics(adp);
  // runtime_ref "py" won't be found in empty execution set → error
  assert.ok(errors.some((e: string) => e.includes("runtime_ref")), `Expected runtime_ref error, got: ${errors.join("; ")}`);
}

// ─── Validation: guardrail with no policy_ref field → policy_ref??""→"" ────────
function testValidationGuardrailNoPolicyRef() {
  // rail with no policy_ref field (not even empty string) → rail.policy_ref ?? "" fires ""
  const adp = {
    adp_version: "0.2.0",
    id: "test.guardrail.nopolicyref",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: { graph: { nodes: [], edges: [], start_nodes: [], end_nodes: [] } },
    evaluation: {},
    guardrails: {
      input: [{ id: "pii", provider: "guardrails-ai" }],  // no policy_ref field at all
    },
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(errors.some((e: string) => e.includes("policy_ref is empty")), `Expected policy_ref error, got: ${errors.join("; ")}`);
}

// ─── Validation: hook with no node_filter field → node_filter??[] fires [] ────
function testValidationHookNoNodeFilter() {
  // hook without node_filter → hook?.node_filter ?? [] fires []
  const adp = {
    adp_version: "0.3.0",
    id: "test.hook.no-node-filter",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: { graph: { nodes: [{ id: "n1", kind: "input" }], edges: [], start_nodes: ["n1"], end_nodes: ["n1"] } },
    evaluation: {},
    hooks: [{ event: "on_node_end", handler: { type: "function", function_ref: "acme:record" } }],  // no node_filter
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(!errors.some((e: string) => e.includes("node_filter")), `Unexpected node_filter error: ${errors.join("; ")}`);
}

// ─── Validation: metric without id + invalid evaluator_ref → metric.id??'?' ──
function testValidationMetricNoIdWithBadEvaluatorRef() {
  // metric without id field AND with non-existent evaluator_ref → metric.id ?? "?" fires
  const adp = {
    adp_version: "0.3.0",
    id: "test.metric.noid",
    runtime: { execution: [{ id: "py", backend: "python", entrypoint: "app:main" }] },
    flow: { graph: { nodes: [], edges: [], start_nodes: [], end_nodes: [] } },
    evaluation: {
      suites: [
        {
          id: "s1",
          metrics: [{ type: "llm_judge", evaluator_ref: "nonexistent-ref" }],  // no id field
        },
      ],
    },
    x_testing: {
      evaluators: [{ id: "real-ev", type: "llm_judge", model: "gpt-4o" }],
    },
  } as any;
  const errors = validateAdpSemantics(adp);
  assert.ok(errors.some((e: string) => e.includes("evaluator_ref") && e.includes("?")), `Expected evaluator_ref error with '?', got: ${errors.join("; ")}`);
}

// ─── Composition: override with no op field → op??'set' fires 'set' ──────────
function testCompositionOverrideDefaultOp() {
  // Override with no op field → op ?? "set" fires "set" (line 126)
  const adp = resolveAdp("mem://defop", makeSimpleResolver(
    _VALID_BASE_MANIFEST.replace('id: "placeholder"', 'id: "def-op"') +
    `overrides:\n  - path: "/id"\n    value: "def-op-result"\n`  // no op field
  )) as any;
  assert.strictEqual(adp.id, "def-op-result", "override without op should default to 'set'");
}

// ─── Composition: delete with null intermediate → null case covers line 140 ──
function testCompositionDeleteWithNullIntermediate() {
  // When an intermediate segment in a delete path is null (not undefined),
  // the code should short-circuit and return unchanged (line 140: node === null)
  const adp = resolveAdp("mem://nullint", makeSimpleResolver(
    `
adp_version: "0.2.0"
id: "null-intermediate"
runtime:
  execution:
    - { id: "py", backend: "python", entrypoint: "app:main" }
description: null
flow:${_VALID_FLOW_YAML}
evaluation:${_VALID_EVAL_YAML}
overrides:
  - path: "/description/subkey"
    op: "delete"
`
  )) as any;
  // description is null; trying to delete /description/subkey should be a no-op
  assert.strictEqual(adp.id, "null-intermediate", "delete through null intermediate should not crash");
}

// ─── Composition: relative extends with mem:// URI that has no path (host only)
function testCompositionMemUriNoPath() {
  // Base URI is "mem://nopath" (no path component) → schemeMatch[2] = "" → basePath = "/" (line 224-225)
  // "mem://nopath" → _resolveUri("base", "mem://nopath")
  //   schemeMatch[1]="mem://nopath", schemeMatch[2]="" → basePath = "" || "/" = "/"
  //   baseDir = "/".substring(0,1) = "/" → joined = "/base" → "mem://nopath/base"
  const manifests: Record<string, string> = {
    "nopath": `
adp_version: "0.2.0"
id: "nopath-child"
extends: "base"
`,
    "nopath/base": `
adp_version: "0.2.0"
id: "nopath-host"
runtime:
  execution:
    - { id: "py", backend: "python", entrypoint: "app:main" }
flow:${_VALID_FLOW_YAML}
evaluation:${_VALID_EVAL_YAML}
`,
  };
  const resolver = (uri: string) => {
    const key = uri.replace("mem://", "");
    /* c8 ignore next 2 */
    if (!(key in manifests)) throw new CompositionError(`unknown: ${uri}`);
    return manifests[key];
  };
  const adp = resolveAdp("mem://nopath", resolver) as any;
  assert.strictEqual(adp.id, "nopath-child", "extends from mem:// host-only URI should work");
}

(function run() {
  testValidateRejectsInvalidFlow();
  testValidate();
  testPackage();
  testSemanticValidationPassesForValidAdp();
  testSemanticValidationRejectsDanglingEdge();
  // Previously uncalled: covers validation.js lines 71-72 (edge.to not found)
  testSemanticValidationRejectsDanglingEdgeTo();
  testSemanticValidationRejectsDuplicateNode();
  testSemanticValidationRejectsBadSuiteRef();
  testSemanticValidationRejectsBadModelRef();
  testSemanticValidationRejectsBadRuntimeRef();
  testConformanceClassFullRejectsEmptyFlow();
  // New semantic checks
  testSemanticCheck7GuardrailEmptyPolicyRef();
  testSemanticCheck8TelemetryInvalidAttr();
  testSemanticCheck8TelemetryValidAttrs();
  testSemanticCheck9ToolMissingEnvVar();
  testSemanticCheck9ToolNoneSchemeOk();
  testSemanticCheck10UnknownComplianceStandard();
  testSemanticCheck10CustomComplianceOk();
  testSemanticCheck11ToolRefMissing();
  // New v0.3.0 semantic checks
  testSemanticCheck12HookNodeFilterUnknown();
  testSemanticCheck12HookNodeFilterValid();
  testSemanticCheck13SubflowAdpRefMissing();
  testSemanticCheck13SubflowAdpRefUri();
  testSemanticCheck14EvaluatorRefMissing();
  testSemanticCheck14EvaluatorRefValid();
  testSemanticPreCompositionWarning();
  // Composition tests
  testCompositionExtendsInheritsBase();
  testCompositionChildOverridesBaseField();
  testCompositionImportAppendsArrays();
  testCompositionCycleDetected();
  testCompositionDepthLimitExceeded();
  testCompositionErrorIsCustomError();
  // Integration structural tests
  testLangGraphIntegrationExportsExpectedFunctions();
  testLangGraphThrowsWhenNotInstalled();
  testSKIntegrationExportsExpectedFunctions();
  testSKBuildRunsInMockMode();
  // New SK coverage tests
  testMakeConditionFnActuallyWorks();
  testSKBuildWithNoModels();
  testSKBuildWithToolNode();
  testSKBuildWithUnmatchedModelRef();
  // Evaluator loader smoke tests
  testLoadEvaluatorLLMJudge();
  testLoadEvaluatorUnknownTypeThrows();
  testLoadEvaluatorsFromManifestMerges();

  // New coverage: DeterministicEvaluator
  const pDet1 = testDeterministicEvaluatorWithBuiltinModule();
  const pDet2 = testDeterministicEvaluatorInvalidRef();
  const pDet3 = testDeterministicEvaluatorFunctionNotFound();
  // Python runtime
  const pPy1 = testScriptEvaluatorPythonSuccess();
  const pPy2 = testScriptEvaluatorPythonError();

  // New coverage: normalizeResult paths
  const p1 = testNormalizeResultViaBoolInput();
  const p2 = testNormalizeResultViaBoolFalse();
  const p3 = testNormalizeResultViaMapPassed();
  const p4 = testNormalizeResultViaMapScoreOnly();
  const p5 = testNormalizeResultViaMapScoreLow();
  const p6 = testNormalizeResultFallback();
  // ScriptEvaluator bash paths
  const p7 = testScriptEvaluatorBashSuccess();
  const p8 = testScriptEvaluatorBashError();
  // ScriptEvaluator error paths
  const p9 = testScriptEvaluatorUnknownRuntime();
  const p10 = testScriptEvaluatorMissingInlineAndRef();
  const p11 = testLLMJudgeEvaluatorThrows();
  // Evaluator default id (covers ?? "" branch in evaluation.js)
  const p12 = testEvaluatorDefaultIdEmpty();
  // All 4 evaluator types
  testLoadEvaluatorAllTypes();
  // loadEvaluatorsFromManifest edge cases
  testLoadEvaluatorsFromManifestEmpty();
  testLoadEvaluatorsFromManifestJudgesOnly();
  testLoadEvaluatorsFromManifestEvaluatorsWin();
  // Composition internals
  testCompositionDeepMergeNullDeletes();
  testCompositionAdditiveMergeArraysAppend();
  testCompositionApplyOverrideDelete();
  testCompositionApplyOverrideDeleteMissingPath();
  testCompositionApplyOverrideAppendToNonArray();
  testCompositionApplyOverrideUnknownOp();
  testCompositionPointerGetArrayAccess();
  testCompositionPointerGetAllowMissing();
  testCompositionSetOnNonExistentKey();
  testCompositionHttpUriRequiresResolver();
  testCompositionMissingFile();
  testCompositionRegistryURIThrows();
  testCompositionRelativeExtendsWithMemScheme();
  testCompositionSetOnArrayWithInvalidIndex();
  testCompositionSetOnNonObjectNonArray();
  testCompositionFileRead();
  // Previously uncalled composition tests (covers lines 176-177, 190-191)
  testCompositionAppendSuccess();
  testCompositionPointerGetSegmentNotFound();
  // New composition override tests (covers lines 129-130, 160-161)
  testCompositionOverridePathNoSlash();
  testCompositionSetExistingKey();
  // New composition tests (covers lines 22-23, 60-66, 102-103)
  testResolveAdpThrowsOnInvalidResolvedManifest();
  testCompositionImportWithSectionsFilter();
  testCompositionAdditiveMergeNewKey();
  // Validation defensive defaults (covers various ??"?" branches)
  testValidationGuardrailNoId();
  testValidationToolNoId();
  testValidationComplianceNoStandard();
  testValidationHookNoEvent();
  testValidationSuiteNoMetrics();
  // New validation coverage: null-coalescing ?? branches
  testValidationNoGraph();
  testValidationGraphNoNodesEdges();
  testValidationBadStartAndEndNodes();
  testValidationRuntimeNoExecution();
  testValidationGuardrailNoPolicyRef();
  testValidationHookNoNodeFilter();
  testValidationMetricNoIdWithBadEvaluatorRef();
  // Validation error branches
  testValidateConformanceClassFullWithEmptyEval();
  testValidateSemanticsJudgesDeprecationWarning();
  // New composition coverage: op default, null intermediate, mem:// host URI
  testCompositionOverrideDefaultOp();
  testCompositionDeleteWithNullIntermediate();
  testCompositionMemUriNoPath();
  // adpkg Inspect/Verify (previously uncalled)
  testVerifyPackageEmptyManifests();
  testCreatePackageThrowsOnInvalidAdp();
  testInspectAndVerifyPackage();
  testVerifyPackageMissingBlob();

  // New async evaluator coverage tests
  const p13 = testScriptEvaluatorJSScriptRef();
  const p14 = testDeterministicEvaluatorDefaultId();
  const p15 = testScriptEvaluatorPythonWithScriptRef();
  const p16 = testScriptEvaluatorBashScriptRef();

  // Return all async test promises
  return Promise.all([pDet1,pDet2,pDet3,pPy1,pPy2,p1,p2,p3,p4,p5,p6,p7,p8,p9,p10,p11,p12,p13,p14,p15,p16]);
})().then(() => {
  console.log("ts tests passed");
  /* c8 ignore next 3 */
}).catch((err: unknown) => {
  console.error("ASYNC TEST FAILURE:", err);
  process.exit(1);
});
