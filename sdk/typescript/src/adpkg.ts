import fs from "fs";
import path from "path";
import * as tar from "tar";
import crypto from "crypto";
import yaml from "js-yaml";
import { validateAdp } from "./validation.ts";

const OCI_LAYOUT = { imageLayoutVersion: "1.0.0" };
const MANIFEST_MEDIA_TYPE = "application/vnd.oci.image.manifest.v1+json";
const LAYER_MEDIA_TYPE = "application/vnd.adp.package.v1+tar";
const CONFIG_MEDIA_TYPE = "application/vnd.adp.config.v1+json";

function sha256(buf: Buffer): string {
  return "sha256:" + crypto.createHash("sha256").update(buf).digest("hex");
}

function writeBlob(blobsDir: string, digest: string, data: Buffer) {
  const [, hex] = digest.split(":");
  const dir = path.join(blobsDir, "sha256");
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, hex), data);
  return data.length;
}

export function createPackage(srcDir: string, outDir: string) {
  const adpPath = path.join(srcDir, "adp", "agent.yaml");
  const adpContent = fs.readFileSync(adpPath, "utf8");
  const adpObj = yaml.load(adpContent) as any;
  const errors = validateAdp(adpObj);
  if (errors.length) throw new Error(`ADP validation failed: ${errors.join("; ")}`);

  fs.mkdirSync(outDir, { recursive: true });
  const blobsRoot = path.join(outDir, "blobs");
  fs.mkdirSync(path.join(blobsRoot, "sha256"), { recursive: true });

  // Config blob
  const configBuf = Buffer.from(JSON.stringify({ agent_id: adpObj.id, adp_version: adpObj.adp_version }));
  const configDigest = sha256(configBuf);
  const configSize = writeBlob(blobsRoot, configDigest, configBuf);

  // Layer tar
  const layerTar = path.join(outDir, "layer.tar");
  tar.c({ file: layerTar, cwd: srcDir, sync: true }, fs.readdirSync(srcDir));
  const layerBuf = fs.readFileSync(layerTar);
  const layerDigest = sha256(layerBuf);
  const layerSize = writeBlob(blobsRoot, layerDigest, layerBuf);
  fs.unlinkSync(layerTar);

  const manifest = {
    schemaVersion: 2,
    mediaType: MANIFEST_MEDIA_TYPE,
    config: { mediaType: CONFIG_MEDIA_TYPE, digest: configDigest, size: configSize },
    layers: [{ mediaType: LAYER_MEDIA_TYPE, digest: layerDigest, size: layerSize }],
  };
  const manifestBuf = Buffer.from(JSON.stringify(manifest, null, 2));
  const manifestDigest = sha256(manifestBuf);
  writeBlob(blobsRoot, manifestDigest, manifestBuf);

  const index = {
    schemaVersion: 2,
    manifests: [
      {
        mediaType: MANIFEST_MEDIA_TYPE,
        digest: manifestDigest,
        size: manifestBuf.length,
        annotations: { "org.opencontainers.image.title": adpObj.id },
      },
    ],
  };
  fs.writeFileSync(path.join(outDir, "index.json"), JSON.stringify(index, null, 2));
  fs.writeFileSync(path.join(outDir, "oci-layout"), JSON.stringify(OCI_LAYOUT));
}

export function openPackage(pkgDir: string) {
  const index = JSON.parse(fs.readFileSync(path.join(pkgDir, "index.json"), "utf8"));
  const manifestDesc = index.manifests[0];
  const manifestPath = path.join(pkgDir, "blobs", manifestDesc.digest.replace("sha256:", "sha256/"));
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  const layerDesc = manifest.layers[0];
  const layerPath = path.join(pkgDir, "blobs", layerDesc.digest.replace("sha256:", "sha256/"));
  const extractDir = fs.mkdtempSync(path.join(pkgDir, "extract-"));
  tar.x({ file: layerPath, cwd: extractDir, sync: true });
  const adpContent = fs.readFileSync(path.join(extractDir, "adp", "agent.yaml"), "utf8");
  return yaml.load(adpContent) as any;
}
