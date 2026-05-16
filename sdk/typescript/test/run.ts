import assert from "assert";
import fs from "fs";
import path from "path";
import { createPackage, openPackage } from "../src/adpkg.js";
import { validateAdp, validateAdpSemantics } from "../src/validation.js";

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
  console.log("ts tests passed");
})();
