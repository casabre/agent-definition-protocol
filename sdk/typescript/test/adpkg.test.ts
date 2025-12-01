import fs from "fs";
import path from "path";
import assert from "assert";
import { createPackage, openPackage } from "../src/adpkg.ts";

function buildSource(tmp: string) {
  const adpDir = path.join(tmp, "adp");
  fs.mkdirSync(adpDir, { recursive: true });
  fs.writeFileSync(
    path.join(adpDir, "agent.yaml"),
    `adp_version: "0.1.0"\nid: "agent.test"\nruntime:\n  execution:\n    - backend: "python"\n      id: "py"\n      entrypoint: "agent.main:app"\nflow: {}\nevaluation:\n  suites:\n    - id: "basic"\n      metrics:\n        - id: "m1"\n          type: "deterministic"\n          function: "noop"\n          scoring: "boolean"\n          threshold: "true"\n`
  );
}

(function run() {
  const tmp = fs.mkdtempSync(path.join(process.cwd(), "ts-oci-"));
  buildSource(tmp);
  const outDir = path.join(tmp, "oci");
  createPackage(tmp, outDir);
  assert.ok(fs.existsSync(path.join(outDir, "oci-layout")));
  const adp = openPackage(outDir) as any;
  assert.equal(adp.id, "agent.test");
  console.log("ts adpkg test passed");
})();
