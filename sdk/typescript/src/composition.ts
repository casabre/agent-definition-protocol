import fs from "fs";
import path from "path";
import yaml from "js-yaml";
import { ADP } from "./adp.js";
import { validateAdp, validateAdpSemantics } from "./validation.js";

export class CompositionError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "CompositionError";
  }
}

export type Resolver = (uri: string) => string;

const MAX_DEPTH = 10;

export function resolveAdp(filePath: string, resolver?: Resolver): ADP {
  // When a resolver is provided, treat filePath as an opaque URI;
  // otherwise resolve it to an absolute filesystem path.
  const absPath = resolver !== undefined ? filePath : path.resolve(filePath);
  const raw = _loadUri(absPath, resolver);
  const merged = _resolveManifest(raw, absPath, new Set(), 0, resolver);
  const schemaErrors = validateAdp(merged);
  const semanticErrors = validateAdpSemantics(merged);
  const allErrors = [...schemaErrors, ...semanticErrors].filter(
    (e) => !e.startsWith("WARNING:")
  );
  if (allErrors.length > 0) {
    throw new CompositionError(
      "Resolved manifest is invalid:\n" + allErrors.join("\n")
    );
  }
  return merged as ADP;
}

function _resolveManifest(
  data: Record<string, unknown>,
  baseUri: string,
  seen: Set<string>,
  depth: number,
  resolver?: Resolver
): Record<string, unknown> {
  if (depth > MAX_DEPTH) {
    throw new CompositionError(`extends chain depth exceeded ${MAX_DEPTH}`);
  }
  if (seen.has(baseUri)) {
    throw new CompositionError(`circular extends detected: ${baseUri}`);
  }
  const newSeen = new Set(seen);
  newSeen.add(baseUri);

  let merged: Record<string, unknown> = {};

  const extendsUri = data["extends"] as string | undefined;
  if (extendsUri) {
    const absUri = _resolveUri(extendsUri, baseUri);
    const baseRaw = _loadUri(absUri, resolver);
    const baseResolved = _resolveManifest(baseRaw, absUri, newSeen, depth + 1, resolver);
    merged = _applyPatch(merged, baseResolved);
  }

  // Apply local fields using id-keyed merge semantics:
  // objects deep-merge; id-carrying lists merge by id; other lists replace.
  const local: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(data)) {
    if (k !== "extends" && k !== "import" && k !== "overrides") {
      local[k] = v;
    }
  }
  merged = _applyPatch(merged, local);

  // Apply imports additively AFTER local
  const imports = data["import"];
  if (Array.isArray(imports)) {
    for (const entry of imports as Array<Record<string, unknown>>) {
      const moduleUri = _resolveUri(entry["from"] as string, baseUri);
      const moduleRaw = _loadUri(moduleUri, resolver);
      const sections = (entry["sections"] as string[] | undefined) ?? [];
      let moduleData: Record<string, unknown> = moduleRaw;
      if (sections.length > 0) {
        moduleData = {};
        for (const s of sections) {
          if (Object.prototype.hasOwnProperty.call(moduleRaw, s)) {
            moduleData[s] = moduleRaw[s];
          }
        }
      }
      merged = _additiveMerge(merged, moduleData);
    }
  }

  // Apply overrides (step 4 — final word)
  const overrides = data["overrides"];
  if (Array.isArray(overrides)) {
    for (const override of overrides as Array<Record<string, unknown>>) {
      merged = _applyOverride(merged, override);
    }
  }

  return merged;
}

function _applyPatch(
  base: Record<string, unknown>,
  patch: Record<string, unknown>
): Record<string, unknown> {
  const result: Record<string, unknown> = Object.assign({}, base);
  for (const [key, val] of Object.entries(patch)) {
    if (val === null) {
      delete result[key];
    } else if (
      typeof val === "object" &&
      !Array.isArray(val) &&
      typeof result[key] === "object" &&
      !Array.isArray(result[key]) &&
      result[key] !== null
    ) {
      result[key] = _applyPatch(
        result[key] as Record<string, unknown>,
        val as Record<string, unknown>
      );
    } else if (Array.isArray(val) && Array.isArray(result[key])) {
      if (_allHaveID(val)) {
        result[key] = _idKeyedMerge(result[key] as unknown[], val);
      } else {
        result[key] = _deepCopy(val);
      }
    } else {
      result[key] = _deepCopy(val);
    }
  }
  return result;
}

function _idKeyedMerge(baseList: unknown[], patchList: unknown[]): unknown[] {
  const result: unknown[] = baseList.map((item) => _deepCopy(item));
  const index = new Map<string, number>();
  result.forEach((item, i) => {
    if (typeof item === "object" && item !== null && !Array.isArray(item)) {
      const id = (item as Record<string, unknown>)["id"];
      if (typeof id === "string") index.set(id, i);
    }
  });
  for (const patchItem of patchList) {
    if (typeof patchItem === "object" && patchItem !== null && !Array.isArray(patchItem)) {
      const id = (patchItem as Record<string, unknown>)["id"];
      if (typeof id === "string" && index.has(id)) {
        result[index.get(id)!] = _applyPatch(
          result[index.get(id)!] as Record<string, unknown>,
          patchItem as Record<string, unknown>
        );
      } else {
        result.push(_deepCopy(patchItem));
      }
    /* c8 ignore next 4 */
    } else {
      result.push(_deepCopy(patchItem));
    }
  }
  return result;
}

function _allHaveID(list: unknown[]): boolean {
  /* c8 ignore next 2 */
  if (list.length === 0) return false;
  return list.every(
    (item) =>
      typeof item === "object" &&
      item !== null &&
      !Array.isArray(item) &&
      "id" in (item as Record<string, unknown>)
  );
}

function _additiveMerge(
  base: Record<string, unknown>,
  module: Record<string, unknown>
): Record<string, unknown> {
  const result: Record<string, unknown> = Object.assign({}, base);
  for (const [key, val] of Object.entries(module)) {
    if (!(key in result)) {
      result[key] = _deepCopy(val);
    } else if (Array.isArray(val) && Array.isArray(result[key])) {
      result[key] = [...(result[key] as unknown[]), ..._deepCopy(val) as unknown[]];
    } else if (
      typeof val === "object" &&
      !Array.isArray(val) &&
      val !== null &&
      typeof result[key] === "object" &&
      !Array.isArray(result[key]) &&
      result[key] !== null
    ) {
      result[key] = _additiveMerge(
        result[key] as Record<string, unknown>,
        val as Record<string, unknown>
      );
    } else {
      result[key] = _deepCopy(val);
    }
  }
  return result;
}

function _deepCopy<T>(val: T): T {
  return JSON.parse(JSON.stringify(val));
}

function _applyOverride(
  data: Record<string, unknown>,
  override: Record<string, unknown>
): Record<string, unknown> {
  const pointerPath = override["path"] as string;
  const op = (override["op"] as string | undefined) ?? "set";
  const value = override["value"];

  if (!pointerPath.startsWith("/")) {
    throw new CompositionError(
      `override path must start with '/': ${JSON.stringify(pointerPath)}`
    );
  }

  const segments = pointerPath
    .slice(1)
    .split("/")
    .map(_unescapePointer);

  const result: Record<string, unknown> = _deepCopy(data);

  if (op === "delete") {
    let node: unknown = result;
    for (const seg of segments.slice(0, -1)) {
      node = _pointerGet(node, seg, pointerPath, true);
      /* c8 ignore next */
      if (node === undefined || node === null) return result;
    }
    const last = segments[segments.length - 1];
    if (typeof node === "object" && node !== null && !Array.isArray(node)) {
      delete (node as Record<string, unknown>)[last];
    }
    return result;
  }

  let node: unknown = result;
  for (const seg of segments.slice(0, -1)) {
    node = _pointerGet(node, seg, pointerPath, false);
  }
  const last = segments[segments.length - 1];

  if (op === "set") {
    if (typeof node === "object" && node !== null && !Array.isArray(node)) {
      const nodeObj = node as Record<string, unknown>;
      if (!(last in nodeObj)) {
        throw new CompositionError(
          `override set: path '${pointerPath}' does not exist`
        );
      }
      nodeObj[last] = _deepCopy(value);
    } else if (Array.isArray(node)) {
      const idx = _toIndex(last, pointerPath);
      (node as unknown[])[idx] = _deepCopy(value);
      /* c8 ignore next 4 */
    } else {
      throw new CompositionError(
        `override set: cannot navigate path '${pointerPath}'`
      );
    }
  } else if (op === "append") {
    const target = _pointerGet(node, last, pointerPath, false);
    if (!Array.isArray(target)) {
      throw new CompositionError(
        `override append: path '${pointerPath}' does not resolve to an array`
      );
    }
    (target as unknown[]).push(_deepCopy(value));
  } else {
    throw new CompositionError(`unknown override op: ${JSON.stringify(op)}`);
  }

  return result;
}

function _pointerGet(
  node: unknown,
  segment: string,
  pointerPath: string,
  allowMissing: boolean
): unknown {
  if (typeof node === "object" && node !== null && !Array.isArray(node)) {
    const nodeObj = node as Record<string, unknown>;
    if (allowMissing) {
      return nodeObj[segment];
    }
    if (!(segment in nodeObj)) {
      throw new CompositionError(
        `override: path segment '${segment}' not found (path: '${pointerPath}')`
      );
    }
    return nodeObj[segment];
  }
  if (Array.isArray(node)) {
    const idx = _toIndex(segment, pointerPath);
    return (node as unknown[])[idx];
  }
  throw new CompositionError(
    `override: cannot navigate into ${typeof node} at path '${pointerPath}'`
  );
}

function _toIndex(segment: string, pointerPath: string): number {
  const idx = parseInt(segment, 10);
  if (isNaN(idx)) {
    throw new CompositionError(
      `override: array index '${segment}' is not an integer (path: '${pointerPath}')`
    );
  }
  return idx;
}

function _unescapePointer(segment: string): string {
  return segment.replace(/~1/g, "/").replace(/~0/g, "~");
}

function _resolveUri(uri: string, baseUri: string): string {
  // Any URI with a scheme (e.g. https://, mem://, file://) is absolute
  if (/^[a-z][a-z0-9+\-.]*:\/\//i.test(uri)) {
    if (uri.startsWith("registry://")) {
      throw new CompositionError(
        `registry:// URIs are not supported in v0.2.0; planned for v0.3.0: ${JSON.stringify(uri)}`
      );
    }
    return uri;
  }
  // Relative path: resolve against the base URI's "directory"
  // For custom schemes (non-file), try to build a sibling URI if possible
  const schemeMatch = /^([a-z][a-z0-9+\-.]*:\/\/[^/]*)(.*)$/i.exec(baseUri);
  if (schemeMatch) {
    // base is a custom-scheme URI; resolve relative path against its "directory"
    const schemeAndHost = schemeMatch[1];
    const basePath = schemeMatch[2] || "/";
    /* c8 ignore next */
    const baseDir = basePath.substring(0, basePath.lastIndexOf("/") + 1) || "/";
    // Simple relative path join (no full URL resolution needed here)
    const joined = baseDir + uri.replace(/^\.\//, "");
    return schemeAndHost + joined;
  }
  const baseDir = path.dirname(baseUri);
  return path.resolve(baseDir, uri);
}

function _loadUri(uri: string, resolver?: Resolver): Record<string, unknown> {
  if (resolver !== undefined) {
    const raw = resolver(uri);
    return yaml.load(raw) as Record<string, unknown>;
  }
  /* c8 ignore next 4 */
  if (/^https?:\/\//.test(uri)) {
    throw new CompositionError(
      `HTTP/HTTPS URIs require a custom resolver in this environment: ${uri}`
    );
  }
  if (!fs.existsSync(uri)) {
    throw new CompositionError(`cannot resolve URI: ${JSON.stringify(uri)}`);
  }
  return yaml.load(fs.readFileSync(uri, "utf8")) as Record<string, unknown>;
}
