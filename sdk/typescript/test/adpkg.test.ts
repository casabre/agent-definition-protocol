import fs from "fs";
import path from "path";
import assert from "assert";
import { createPackage, openPackage } from "../src/adpkg.js";

function buildSource(tmp: string) {
  const adpDir = path.join(tmp, "adp");
  fs.mkdirSync(adpDir, { recursive: true });
  fs.writeFileSync(
    path.join(adpDir, "agent.yaml"),
    `adp_version: "0.1.0"\nid: "agent.test"\nruntime:\n  execution:\n    - backend: "python"\n      id: "py"\n      entrypoint: "agent.main:app"\nflow: {}\nevaluation:\n  suites:\n    - id: "basic"\n      metrics:\n        - id: "m1"\n          type: "deterministic"\n          function: "noop"\n          scoring: "boolean"\n          threshold: "true"\n`
  );
}

function buildSourceWithMetadata(tmp: string) {
  buildSource(tmp);
  const metadataDir = path.join(tmp, "metadata");
  fs.mkdirSync(metadataDir, { recursive: true });
  fs.writeFileSync(
    path.join(metadataDir, "version.json"),
    JSON.stringify({
      agent_id: "agent.test",
      agent_version: "1.0.0",
      spec_version: "0.1.0",
      build_timestamp: "2024-01-15T10:30:00Z",
    })
  );
}

test("createPackage creates OCI layout structure", () => {
  const tmp = fs.mkdtempSync(path.join(process.cwd(), "ts-oci-"));
  try {
    buildSource(tmp);
    const outDir = path.join(tmp, "oci");
    createPackage(tmp, outDir);
    
    assert.ok(fs.existsSync(path.join(outDir, "oci-layout")), "oci-layout should exist");
    assert.ok(fs.existsSync(path.join(outDir, "index.json")), "index.json should exist");
    assert.ok(fs.existsSync(path.join(outDir, "blobs", "sha256")), "blobs/sha256 directory should exist");
    
    // Verify oci-layout content
    const layout = JSON.parse(fs.readFileSync(path.join(outDir, "oci-layout"), "utf8"));
    assert.deepStrictEqual(layout, { imageLayoutVersion: "1.0.0" }, "oci-layout should have correct version");
    
    // Verify index.json structure
    const index = JSON.parse(fs.readFileSync(path.join(outDir, "index.json"), "utf8"));
    assert.ok(Array.isArray(index.manifests), "index.json should contain manifests array");
    assert.strictEqual(index.manifests.length, 1, "index.json should reference exactly one manifest");
    assert.strictEqual(
      index.manifests[0].mediaType,
      "application/vnd.oci.image.manifest.v1+json",
      "manifest should have correct media type"
    );
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
});

test("openPackage reads ADP correctly", () => {
  const tmp = fs.mkdtempSync(path.join(process.cwd(), "ts-oci-"));
  try {
    buildSource(tmp);
    const outDir = path.join(tmp, "oci");
    createPackage(tmp, outDir);
    
    const adp = openPackage(outDir) as any;
    assert.strictEqual(adp.id, "agent.test", "ADP id should match");
    assert.strictEqual(adp.adp_version, "0.1.0", "ADP version should match");
    assert.ok(adp.runtime, "ADP should have runtime");
    assert.ok(Array.isArray(adp.runtime.execution), "runtime.execution should be array");
    assert.strictEqual(adp.runtime.execution.length, 1, "should have one execution entry");
    assert.strictEqual(adp.runtime.execution[0].backend, "python", "backend should match");
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
});

test("package contains required files in layer", () => {
  const tmp = fs.mkdtempSync(path.join(process.cwd(), "ts-oci-"));
  try {
    buildSourceWithMetadata(tmp);
    const outDir = path.join(tmp, "oci");
    createPackage(tmp, outDir);
    
    // Extract manifest
    const index = JSON.parse(fs.readFileSync(path.join(outDir, "index.json"), "utf8"));
    const manifestDigest = index.manifests[0].digest.replace("sha256:", "");
    const manifestPath = path.join(outDir, "blobs", "sha256", manifestDigest);
    assert.ok(fs.existsSync(manifestPath), "manifest blob should exist");
    
    const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
    assert.strictEqual(manifest.mediaType, "application/vnd.oci.image.manifest.v1+json");
    assert.ok(Array.isArray(manifest.layers), "manifest should have layers");
    assert.ok(manifest.layers.length > 0, "manifest should have at least one layer");
    
    // Verify layer exists
    const layerDigest = manifest.layers[0].digest.replace("sha256:", "");
    const layerPath = path.join(outDir, "blobs", "sha256", layerDigest);
    assert.ok(fs.existsSync(layerPath), "layer blob should exist");
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
});

test("package config blob has correct structure", () => {
  const tmp = fs.mkdtempSync(path.join(process.cwd(), "ts-oci-"));
  try {
    buildSource(tmp);
    const outDir = path.join(tmp, "oci");
    createPackage(tmp, outDir);
    
    const index = JSON.parse(fs.readFileSync(path.join(outDir, "index.json"), "utf8"));
    const manifestDigest = index.manifests[0].digest.replace("sha256:", "");
    const manifest = JSON.parse(
      fs.readFileSync(path.join(outDir, "blobs", "sha256", manifestDigest), "utf8")
    );
    
    const configDigest = manifest.config.digest.replace("sha256:", "");
    const configPath = path.join(outDir, "blobs", "sha256", configDigest);
    assert.ok(fs.existsSync(configPath), "config blob should exist");
    
    const config = JSON.parse(fs.readFileSync(configPath, "utf8"));
    assert.ok(config.agent_id, "config should contain agent_id");
    assert.ok(config.adp_version, "config should contain adp_version");
    assert.strictEqual(config.agent_id, "agent.test");
    assert.strictEqual(config.adp_version, "0.1.0");
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
});

test("package handles missing adp directory", () => {
  const tmp = fs.mkdtempSync(path.join(process.cwd(), "ts-oci-"));
  try {
    const outDir = path.join(tmp, "oci");
    assert.throws(
      () => createPackage(tmp, outDir),
      /agent\.yaml/,
      "should throw error for missing agent.yaml"
    );
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
});
