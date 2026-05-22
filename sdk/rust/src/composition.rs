use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde_json::Value as JsonValue;

use crate::adp::Adp;
use crate::validation::{validate_adp, validate_adp_semantics};

const MAX_DEPTH: usize = 10;

/// Resolve an ADP manifest at `path`, applying extends / import / overrides composition.
///
/// `resolver` is an optional closure that receives a URI string and returns YAML content.
/// When `None`, the function reads from the local filesystem.
///
/// Returns `Ok(Adp)` on success or `Err(Vec<String>)` with all collected errors.
pub fn resolve_adp(
    path: &str,
    resolver: Option<Box<dyn Fn(&str) -> Result<String, String>>>,
) -> Result<Adp, Vec<String>> {
    // When a resolver is provided we use the path as-is (it may be a virtual URI).
    // When reading from disk we canonicalize so that cycle detection works correctly.
    let abs = if resolver.is_some() {
        path.to_string()
    } else {
        canonicalize_path(path).map_err(|e| vec![e])?
    };
    let raw_yaml = load_uri(&abs, resolver.as_deref()).map_err(|e| vec![e])?;
    let raw: JsonValue = yaml_str_to_json(&raw_yaml).map_err(|e| vec![e])?;

    let has_resolver = resolver.is_some();
    let mut seen: HashSet<String> = HashSet::new();
    let merged =
        resolve_manifest(raw, &abs, &mut seen, 0, resolver.as_deref(), has_resolver)
            .map_err(|e| vec![e])?;

    // Deserialize to Adp via JSON round-trip
    let adp: Adp = serde_json::from_value(merged)
        .map_err(|e| vec![format!("deserialization error: {e}")])?;

    let mut errors: Vec<String> = Vec::new();

    if let Err(e) = validate_adp(&adp) {
        errors.push(e.to_string());
    }
    errors.extend(validate_adp_semantics(&adp));

    if errors.is_empty() {
        Ok(adp)
    } else {
        Err(errors)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn resolve_manifest(
    data: JsonValue,
    base_uri: &str,
    seen: &mut HashSet<String>,
    depth: usize,
    resolver: Option<&dyn Fn(&str) -> Result<String, String>>,
    has_resolver: bool,
) -> Result<JsonValue, String> {
    if depth > MAX_DEPTH {
        return Err(format!("extends chain depth exceeded {MAX_DEPTH}"));
    }
    if seen.contains(base_uri) {
        return Err(format!("circular extends detected: {base_uri}"));
    }
    seen.insert(base_uri.to_string());

    let mut merged: JsonValue = JsonValue::Object(serde_json::Map::new());

    // 1. Resolve extends base first
    if let Some(extends_uri) = data.get("extends").and_then(|v| v.as_str()) {
        let abs_uri = resolve_uri_impl(extends_uri, base_uri, !has_resolver)?;
        let base_yaml = load_uri(&abs_uri, resolver)?;
        let base_raw: JsonValue = yaml_str_to_json(&base_yaml)?;
        let base_resolved =
            resolve_manifest(base_raw, &abs_uri, seen, depth + 1, resolver, has_resolver)?;
        merged = deep_merge(merged, base_resolved);
    }

    // 2. Apply local fields (excluding composition keys)
    let local: serde_json::Map<String, JsonValue> = data
        .as_object()
        .map(|m| {
            m.iter()
                .filter(|(k, _)| !matches!(k.as_str(), "extends" | "import" | "overrides"))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .unwrap_or_default();
    merged = deep_merge(merged, JsonValue::Object(local));

    // 3. Additive import merge
    if let Some(imports) = data.get("import").and_then(|v| v.as_array()) {
        for entry in imports {
            let from_uri = entry
                .get("from")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "import entry missing 'from' field".to_string())?;
            let module_uri = resolve_uri_impl(from_uri, base_uri, !has_resolver)?;
            let module_yaml = load_uri(&module_uri, resolver)?;
            let mut module_raw: JsonValue = yaml_str_to_json(&module_yaml)?;

            // Filter to requested sections if present
            if let Some(sections) = entry.get("sections").and_then(|v| v.as_array()) {
                if !sections.is_empty() {
                    let section_keys: HashSet<&str> =
                        sections.iter().filter_map(|s| s.as_str()).collect();
                    if let Some(obj) = module_raw.as_object_mut() {
                        obj.retain(|k, _| section_keys.contains(k.as_str()));
                    }
                }
            }

            merged = additive_merge(merged, module_raw);
        }
    }

    // 4. Apply overrides
    if let Some(overrides) = data.get("overrides").and_then(|v| v.as_array()) {
        for ov in overrides {
            let path = ov
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "override entry missing 'path' field".to_string())?;
            let op = ov
                .get("op")
                .and_then(|v| v.as_str())
                .unwrap_or("set");
            let value = ov.get("value").cloned();
            merged = apply_override(merged, path, op, value)?;
        }
    }

    seen.remove(base_uri);
    Ok(merged)
}

/// RFC 7396 deep merge: overlay wins; null removes key; arrays replace.
fn deep_merge(base: JsonValue, overlay: JsonValue) -> JsonValue {
    match (base, overlay) {
        (JsonValue::Object(mut base_map), JsonValue::Object(overlay_map)) => {
            for (k, v) in overlay_map {
                if v.is_null() {
                    base_map.remove(&k);
                } else if let (Some(JsonValue::Object(_)), JsonValue::Object(_)) =
                    (base_map.get(&k), &v)
                {
                    let base_val = base_map.remove(&k).unwrap();
                    base_map.insert(k, deep_merge(base_val, v));
                } else {
                    base_map.insert(k, v);
                }
            }
            JsonValue::Object(base_map)
        }
        (_, overlay) => overlay,
    }
}

/// Additive merge: arrays append; objects recurse; scalars: module wins.
fn additive_merge(base: JsonValue, module: JsonValue) -> JsonValue {
    match (base, module) {
        (JsonValue::Object(mut base_map), JsonValue::Object(module_map)) => {
            for (k, v) in module_map {
                let merged = match base_map.remove(&k) {
                    Some(JsonValue::Array(mut a)) => {
                        if let JsonValue::Array(b) = v {
                            a.extend(b);
                            JsonValue::Array(a)
                        } else {
                            v // module wins
                        }
                    }
                    Some(e @ JsonValue::Object(_)) => {
                        if v.is_object() {
                            additive_merge(e, v)
                        } else {
                            v // module wins
                        }
                    }
                    Some(_) => v,   // scalar existing: module wins
                    None => v,      // key not in base: take from module
                };
                base_map.insert(k, merged);
            }
            JsonValue::Object(base_map)
        }
        // When either side is not an Object, module wins
        (_, m) => m,
    }
}

fn apply_override(
    mut data: JsonValue,
    path: &str,
    op: &str,
    value: Option<JsonValue>,
) -> Result<JsonValue, String> {
    if !path.starts_with('/') {
        return Err(format!("override path must start with '/': {path:?}"));
    }

    let segments: Vec<String> = path
        .trim_start_matches('/')
        .split('/')
        .map(unescape_pointer)
        .collect();

    if op == "delete" {
        let node = navigate_mut(&mut data, &segments[..segments.len() - 1], path, true)?;
        if let Some(n) = node {
            let last = &segments[segments.len() - 1];
            if let JsonValue::Object(ref mut map) = n {
                map.remove(last);
            }
            // ignore missing — no-op
        }
        return Ok(data);
    }

    // For set/append we need a mutable reference to the parent
    {
        let node = navigate_mut(&mut data, &segments[..segments.len() - 1], path, false)?
            .ok_or_else(|| format!("override: path '{path}' parent not found"))?;
        let last = &segments[segments.len() - 1];

        match op {
            "set" => {
                match node {
                    JsonValue::Object(ref mut map) => {
                        if !map.contains_key(last) {
                            return Err(format!("override set: path '{path}' does not exist"));
                        }
                        map.insert(last.clone(), value.unwrap_or(JsonValue::Null));
                    }
                    JsonValue::Array(ref mut arr) => {
                        let idx = parse_index(last, path)?;
                        arr[idx] = value.unwrap_or(JsonValue::Null);
                    }
                    _ => return Err(format!("override set: cannot navigate path '{path}'")),
                }
            }
            "append" => {
                let target = pointer_get_mut(node, last, path)?;
                match target {
                    JsonValue::Array(ref mut arr) => {
                        arr.push(value.unwrap_or(JsonValue::Null));
                    }
                    _ => {
                        return Err(format!(
                            "override append: path '{path}' does not resolve to an array"
                        ))
                    }
                }
            }
            _ => return Err(format!("unknown override op: {op:?}")),
        }
    }

    Ok(data)
}

/// Navigate into `data` following all but the last segment. Returns `None` only
/// when `allow_missing` is true and a segment is absent.
fn navigate_mut<'a>(
    data: &'a mut JsonValue,
    segments: &[String],
    path: &str,
    allow_missing: bool,
) -> Result<Option<&'a mut JsonValue>, String> {
    let mut current = data;
    for seg in segments {
        match current {
            JsonValue::Object(ref mut map) => {
                if allow_missing && !map.contains_key(seg) {
                    return Ok(None);
                }
                current = map
                    .get_mut(seg)
                    .ok_or_else(|| format!("override: path segment '{seg}' not found (path: '{path}')"))?;
            }
            JsonValue::Array(ref mut arr) => {
                let idx = parse_index(seg, path)?;
                current = arr
                    .get_mut(idx)
                    .ok_or_else(|| format!("override: array index {idx} out of bounds (path: '{path}')"))?;
            }
            _ => return Err(format!("override: cannot navigate into scalar at path '{path}'")),
        }
    }
    Ok(Some(current))
}

fn pointer_get_mut<'a>(
    node: &'a mut JsonValue,
    segment: &str,
    path: &str,
) -> Result<&'a mut JsonValue, String> {
    match node {
        JsonValue::Object(ref mut map) => map
            .get_mut(segment)
            .ok_or_else(|| format!("override: path segment '{segment}' not found (path: '{path}')")),
        JsonValue::Array(ref mut arr) => {
            let idx = parse_index(segment, path)?;
            arr.get_mut(idx)
                .ok_or_else(|| format!("override: array index {idx} out of bounds (path: '{path}')"))
        }
        _ => Err(format!("override: cannot navigate into scalar at '{path}'")),
    }
}

fn parse_index(s: &str, path: &str) -> Result<usize, String> {
    s.parse::<usize>()
        .map_err(|_| format!("override: array index '{s}' is not an integer (path: '{path}')"))
}

fn unescape_pointer(s: &str) -> String {
    s.replace("~1", "/").replace("~0", "~")
}

fn canonicalize_path(path: &str) -> Result<String, String> {
    let p = Path::new(path);
    if p.is_absolute() {
        Ok(path.to_string())
    } else {
        std::fs::canonicalize(p)
            .map(|pb| pb.to_string_lossy().into_owned())
            .map_err(|e| format!("cannot canonicalize path '{path}': {e}"))
    }
}

/// Resolve a (possibly relative) URI against `base_uri`.
///
/// When `use_fs_canonicalize` is false we just do a string-level join without
/// hitting the filesystem (needed for virtual/in-memory resolver scenarios).
fn resolve_uri_impl(uri: &str, base_uri: &str, use_fs_canonicalize: bool) -> Result<String, String> {
    // Pass-through for absolute URIs
    if uri.starts_with("http://") || uri.starts_with("https://") || uri.starts_with("file://") {
        return Ok(uri.to_string());
    }
    if uri.starts_with("registry://") {
        return Err(format!(
            "registry:// URIs are not supported in v0.2.0; planned for v0.3.0: {uri:?}"
        ));
    }
    let base_path = PathBuf::from(base_uri);
    let parent = base_path.parent().unwrap_or_else(|| Path::new("."));
    let resolved = parent.join(uri);

    if use_fs_canonicalize {
        std::fs::canonicalize(&resolved)
            .map(|p| p.to_string_lossy().into_owned())
            .map_err(|e| format!("cannot resolve URI '{uri}' relative to '{base_uri}': {e}"))
    } else {
        // Normalize path separators but don't touch the filesystem
        Ok(resolved.to_string_lossy().into_owned())
    }
}


fn load_uri(uri: &str, resolver: Option<&dyn Fn(&str) -> Result<String, String>>) -> Result<String, String> {
    if let Some(r) = resolver {
        return r(uri);
    }
    std::fs::read_to_string(uri).map_err(|e| format!("cannot read '{uri}': {e}"))
}

fn yaml_str_to_json(yaml: &str) -> Result<JsonValue, String> {
    let val: serde_yaml::Value =
        serde_yaml::from_str(yaml).map_err(|e| format!("YAML parse error: {e}"))?;
    serde_json::to_value(&val).map_err(|e| format!("YAML-to-JSON conversion error: {e}"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_resolver(files: HashMap<String, String>) -> Box<dyn Fn(&str) -> Result<String, String>> {
        Box::new(move |uri: &str| {
            files
                .get(uri)
                .cloned()
                .ok_or_else(|| format!("resolver: unknown URI '{uri}'"))
        })
    }

    /// Returns a minimal valid YAML manifest string.
    ///
    /// Uses a proper flow (id + graph) and evaluation (suites) to satisfy the
    /// JSON Schema, matching the patterns in tests/validation.rs.
    fn minimal_manifest(id: &str) -> String {
        format!(
            r#"adp_version: "0.1.0"
id: "{id}"
runtime:
  execution:
    - id: "r1"
      backend: "python"
      entrypoint: "agent.main:app"
flow:
  id: "{id}.flow"
  graph:
    nodes:
      - id: "n1"
        kind: "input"
    edges: []
    start_nodes: ["n1"]
    end_nodes: ["n1"]
evaluation:
  suites:
    - id: "s1"
      metrics:
        - id: "m1"
          type: "deterministic"
          function: "noop"
          scoring: "boolean"
          threshold: true
"#
        )
    }

    #[test]
    fn test_basic_extends_merge() {
        let mut files: HashMap<String, String> = HashMap::new();

        // Base manifest adds a model; child inherits it via extends.
        let base = format!(
            r#"adp_version: "0.1.0"
id: "base"
runtime:
  execution:
    - id: "r1"
      backend: "python"
      entrypoint: "agent.main:app"
  models:
    - id: "m1"
      provider: "openai"
      model: "gpt-4"
flow:
  id: "base.flow"
  graph:
    nodes:
      - id: "n1"
        kind: "input"
    edges: []
    start_nodes: ["n1"]
    end_nodes: ["n1"]
evaluation:
  suites:
    - id: "s1"
      metrics:
        - id: "m1"
          type: "deterministic"
          function: "noop"
          scoring: "boolean"
          threshold: true
"#
        );

        let child = format!(
            r#"adp_version: "0.1.0"
id: "child"
extends: "base"
runtime:
  execution:
    - id: "r1"
      backend: "python"
      entrypoint: "agent.main:app"
flow:
  id: "child.flow"
  graph:
    nodes:
      - id: "n1"
        kind: "input"
    edges: []
    start_nodes: ["n1"]
    end_nodes: ["n1"]
evaluation:
  suites:
    - id: "s1"
      metrics:
        - id: "m1"
          type: "deterministic"
          function: "noop"
          scoring: "boolean"
          threshold: true
"#
        );

        files.insert("base".to_string(), base);
        files.insert("child".to_string(), child);

        let resolver = make_resolver(files);
        let result = resolve_adp("child", Some(resolver));
        // The child should resolve successfully, inheriting models from base
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
        let adp = result.unwrap();
        assert_eq!(adp.id, "child");
        // Models inherited from base (child YAML doesn't declare models)
        assert!(adp.runtime.models.is_some(), "models should be inherited from base");
        let models = adp.runtime.models.unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "m1");
    }

    #[test]
    fn test_cycle_detection() {
        let mut files: HashMap<String, String> = HashMap::new();
        files.insert("a".to_string(), minimal_manifest("a").replace(
            r#"id: "a""#,
            "id: \"a\"\nextends: \"b\"",
        ));
        files.insert("b".to_string(), minimal_manifest("b").replace(
            r#"id: "b""#,
            "id: \"b\"\nextends: \"a\"",
        ));

        let resolver = make_resolver(files);
        let result = resolve_adp("a", Some(resolver));
        assert!(result.is_err());
        let errors = result.unwrap_err();
        let combined = errors.join(" ");
        assert!(
            combined.contains("circular"),
            "Expected cycle error, got: {combined}"
        );
    }

    #[test]
    fn test_import_additive_arrays() {
        let mut files: HashMap<String, String> = HashMap::new();
        // main has suite-a; module has suite-b — import should append
        let main_yaml = format!(
            r#"adp_version: "0.1.0"
id: "main"
runtime:
  execution:
    - id: "r1"
      backend: "python"
      entrypoint: "agent.main:app"
flow:
  id: "main.flow"
  graph:
    nodes:
      - id: "n1"
        kind: "input"
    edges: []
    start_nodes: ["n1"]
    end_nodes: ["n1"]
evaluation:
  suites:
    - id: "suite-a"
      metrics:
        - id: "ma"
          type: "deterministic"
          function: "noop"
          scoring: "boolean"
          threshold: true
import:
  - id: "mod"
    from: "module"
    sections:
      - evaluation
"#
        );
        let module_yaml = r#"id: "module"
evaluation:
  suites:
    - id: "suite-b"
      metrics:
        - id: "mb"
          type: "deterministic"
          function: "noop"
          scoring: "boolean"
          threshold: true
"#
        .to_string();
        files.insert("main".to_string(), main_yaml);
        files.insert("module".to_string(), module_yaml);

        let resolver = make_resolver(files);
        let result = resolve_adp("main", Some(resolver));
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
        let adp = result.unwrap();
        // evaluation.suites should contain both suite-a (local) and suite-b (imported)
        let eval_val = serde_json::to_value(&adp.evaluation).unwrap();
        let suites = eval_val["suites"].as_array().expect("suites should be an array");
        let ids: Vec<&str> = suites
            .iter()
            .filter_map(|s| s["id"].as_str())
            .collect();
        assert!(ids.contains(&"suite-a"), "Missing suite-a; got: {ids:?}");
        assert!(ids.contains(&"suite-b"), "Missing suite-b; got: {ids:?}");
    }

    #[test]
    fn test_override_set() {
        let mut files: HashMap<String, String> = HashMap::new();
        // Build manifest with an override that renames the id
        let yaml = format!(
            r#"adp_version: "0.1.0"
id: "ov-test"
runtime:
  execution:
    - id: "r1"
      backend: "python"
      entrypoint: "agent.main:app"
flow:
  id: "ov.flow"
  graph:
    nodes:
      - id: "n1"
        kind: "input"
    edges: []
    start_nodes: ["n1"]
    end_nodes: ["n1"]
evaluation:
  suites:
    - id: "s1"
      metrics:
        - id: "m1"
          type: "deterministic"
          function: "noop"
          scoring: "boolean"
          threshold: true
overrides:
  - path: "/id"
    op: "set"
    value: "ov-test-overridden"
"#
        );
        files.insert("ov-test".to_string(), yaml);

        let resolver = make_resolver(files);
        let result = resolve_adp("ov-test", Some(resolver));
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
        let adp = result.unwrap();
        assert_eq!(adp.id, "ov-test-overridden");
    }
}
