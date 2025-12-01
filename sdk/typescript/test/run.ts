import assert from "assert";
import fs from "fs";
import path from "path";
import { createPackage, openPackage } from "../src/adpkg.ts";
import { validateAdp } from "../src/validation.ts";

function buildSource(tmp: string) {
  const adpDir = path.join(tmp, "adp");
  fs.mkdirSync(adpDir, { recursive: true });
  fs.writeFileSync(
    path.join(adpDir, "agent.yaml"),
    `adp_version: "0.1.0"\nid: "agent.test"\nruntime:\n  execution:\n    - backend: "python"\n      id: "py"\n      entrypoint: "agent.main:app"\nflow: { id: "flow.test", graph: { nodes: [{ id: "n1", kind: "input" }], edges: [], start_nodes: ["n1"], end_nodes: ["n1"] } }\nevaluation:\n  suites:\n    - id: basic\n      metrics:\n        - id: m1\n          type: deterministic\n          function: noop\n          scoring: boolean\n          threshold: true\n`
  );
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

(function run() {
  testValidate();
  testPackage();
  console.log("ts tests passed");
})();
