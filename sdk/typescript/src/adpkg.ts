import fs from "fs";
import path from "path";
import { Readable } from "stream";
import AdmZip from "adm-zip";
import yaml from "js-yaml";
import { validateAdp } from "./validation";

export function createPackage(srcDir: string, outPath: string) {
  const zip = new AdmZip();
  const adpPath = path.join(srcDir, "adp", "agent.yaml");
  const adpContent = fs.readFileSync(adpPath, "utf8");
  validateAdp(yaml.load(adpContent) as any);
  zip.addFile("adp/agent.yaml", Buffer.from(adpContent, "utf8"));
  const acsPath = path.join(srcDir, "acs", "container.yaml");
  if (fs.existsSync(acsPath)) {
    zip.addLocalFile(acsPath, "acs");
  }
  ["eval", "tools", "src", "metadata"].forEach((dir) => {
    const full = path.join(srcDir, dir);
    if (fs.existsSync(full)) {
      zip.addLocalFolder(full, dir);
    }
  });
  zip.writeZip(outPath);
}

export function openPackage(pkgPath: string) {
  const zip = new AdmZip(pkgPath);
  const adpEntry = zip.readAsText("adp/agent.yaml");
  return yaml.load(adpEntry) as any;
}
