import fs from "fs";
import yaml from "js-yaml";
import { validateAdp } from "./validation";

export interface ADP {
  adp_version: string;
  id: string;
  runtime: any;
  flow: any;
  evaluation: any;
  [key: string]: any;
}

export function parseADP(path: string): ADP {
  const data = yaml.load(fs.readFileSync(path, "utf8")) as ADP;
  return data;
}

export function validateADPFile(path: string): string[] {
  const adp = parseADP(path);
  return validateAdp(adp);
}
