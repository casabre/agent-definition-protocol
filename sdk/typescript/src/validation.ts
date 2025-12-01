import fs from "fs";
import path from "path";
import Ajv from "ajv";
import yaml from "js-yaml";
import { fileURLToPath } from "url";

const ajv = new Ajv({ allErrors: true, strict: false, validateSchema: false });
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
  const validate = ajv.compile(adpSchema);
  const ok = validate(adp);
  if (ok) return [];
  return (validate.errors || []).map((e) => `${e.instancePath} ${e.message}`.trim());
}
