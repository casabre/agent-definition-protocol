import assert from "assert";
import fs from "fs";
import path from "path";
import { createPackage, openPackage } from "../src/adpkg.js";
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

(function run() {
  testValidateRejectsInvalidFlow();
  testValidate();
  testPackage();
  testSemanticValidationPassesForValidAdp();
  testSemanticValidationRejectsDanglingEdge();
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
  // Evaluator loader smoke tests
  testLoadEvaluatorLLMJudge();
  testLoadEvaluatorUnknownTypeThrows();
  testLoadEvaluatorsFromManifestMerges();
  console.log("ts tests passed");
})();
