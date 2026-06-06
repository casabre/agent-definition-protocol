use std::collections::{HashMap, HashSet};
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

    // 2. Apply local fields using id-keyed merge semantics:
    // objects deep-merge; id-carrying lists merge by id; other lists replace.
    let local: serde_json::Map<String, JsonValue> = data
        .as_object()
        .map(|m| {
            m.iter()
                .filter(|(k, _)| !matches!(k.as_str(), "extends" | "import" | "overrides"))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .unwrap_or_default();
    merged = apply_patch(merged, JsonValue::Object(local));

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

/// Apply structural patch: objects deep-merge; id-keyed lists merge by id; other lists replace.
fn apply_patch(base: JsonValue, patch: JsonValue) -> JsonValue {
    match (base, patch) {
        (JsonValue::Object(mut base_map), JsonValue::Object(patch_map)) => {
            for (k, v) in patch_map {
                if v.is_null() {
                    base_map.remove(&k);
                } else if v.is_object() {
                    let base_val = base_map
                        .remove(&k)
                        .unwrap_or_else(|| JsonValue::Object(serde_json::Map::new()));
                    base_map.insert(k, apply_patch(base_val, v));
                } else {
                    match v {
                        JsonValue::Array(patch_arr) => {
                            match base_map.remove(&k) {
                                Some(JsonValue::Array(base_arr)) => {
                                    if all_have_id(&patch_arr) {
                                        base_map.insert(k, id_keyed_merge(base_arr, patch_arr));
                                    } else {
                                        base_map.insert(k, JsonValue::Array(patch_arr));
                                    }
                                }
                                _ => {
                                    base_map.insert(k, JsonValue::Array(patch_arr));
                                }
                            }
                        }
                        other => {
                            base_map.insert(k, other);
                        }
                    }
                }
            }
            JsonValue::Object(base_map)
        }
        (_, patch) => patch,
    }
}

/// Merge two arrays by "id" field. Matched entries deep-patched; unknowns appended; unmatched base kept.
fn id_keyed_merge(base_list: Vec<JsonValue>, patch_list: Vec<JsonValue>) -> JsonValue {
    let mut result = base_list;
    let mut index: HashMap<String, usize> = HashMap::new();
    for (i, item) in result.iter().enumerate() {
        if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
            index.insert(id.to_string(), i);
        }
    }
    for patch_item in patch_list {
        let id_opt: Option<String> = patch_item
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        match id_opt {
            Some(id) => {
                if let Some(&idx) = index.get(&id) {
                    let base_entry = std::mem::replace(&mut result[idx], JsonValue::Null);
                    result[idx] = apply_patch(base_entry, patch_item);
                } else {
                    result.push(patch_item);
                }
            }
            None => result.push(patch_item),
        }
    }
    JsonValue::Array(result)
}

/// Returns true iff the list is non-empty and every element is an object with an "id" key.
fn all_have_id(list: &[JsonValue]) -> bool {
    !list.is_empty() && list.iter().all(|item| item.is_object() && item.get("id").is_some())
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

    #[test]
    fn test_override_delete() {
        let mut files: HashMap<String, String> = HashMap::new();
        let yaml = minimal_manifest("del-test").replace(
            "id: \"del-test\"\n",
            "id: \"del-test\"\noverrides:\n  - path: \"/conformance_class\"\n    op: \"delete\"\n",
        );
        files.insert("del-test".to_string(), yaml);
        let result = resolve_adp("del-test", Some(make_resolver(files)));
        assert!(result.is_ok(), "delete override should succeed: {:?}", result.err());
    }

    #[test]
    fn test_override_append() {
        let mut files: HashMap<String, String> = HashMap::new();
        let yaml = format!(
            r#"adp_version: "0.1.0"
id: "app-test"
runtime:
  execution:
    - id: "r1"
      backend: "python"
      entrypoint: "agent.main:app"
flow:
  id: "app-test.flow"
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
  - path: "/runtime/execution"
    op: "append"
    value:
      id: "r2"
      backend: "python"
      entrypoint: "agent.alt:app"
"#
        );
        files.insert("app-test".to_string(), yaml);
        let result = resolve_adp("app-test", Some(make_resolver(files)));
        assert!(result.is_ok(), "append override should succeed: {:?}", result.err());
    }

    #[test]
    fn test_override_unknown_op_returns_error() {
        let data = serde_json::json!({"id": "x", "runtime": {"execution": []}});
        let result = apply_override(data, "/id", "noop", Some(serde_json::json!("y")));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown override op"));
    }

    #[test]
    fn test_override_set_array_index() {
        let data = serde_json::json!({"items": ["a", "b", "c"]});
        let result = apply_override(data, "/items/1", "set", Some(serde_json::json!("X")));
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["items"][1], "X");
    }

    #[test]
    fn test_override_path_must_start_with_slash() {
        let data = serde_json::json!({"id": "x"});
        let result = apply_override(data, "id", "set", Some(serde_json::json!("y")));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must start with '/'"));
    }

    #[test]
    fn test_override_set_missing_key_returns_error() {
        let data = serde_json::json!({"id": "x"});
        let result = apply_override(data, "/nonexistent", "set", Some(serde_json::json!("y")));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_navigate_mut_into_array() {
        let mut data = serde_json::json!({"items": ["a", "b"]});
        let result = navigate_mut(&mut data, &["items".to_string(), "0".to_string()], "/items/0", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_navigate_mut_scalar_error() {
        let mut data = serde_json::json!({"val": 42});
        let result = navigate_mut(&mut data, &["val".to_string(), "sub".to_string()], "/val/sub", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot navigate into scalar"));
    }

    #[test]
    fn test_navigate_mut_allow_missing_returns_none() {
        let mut data = serde_json::json!({"a": {"b": 1}});
        let result = navigate_mut(&mut data, &["missing".to_string()], "/missing", true);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_navigate_mut_missing_key_error() {
        let mut data = serde_json::json!({"a": 1});
        let result = navigate_mut(&mut data, &["missing".to_string()], "/missing", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_navigate_mut_array_out_of_bounds() {
        let mut data = serde_json::json!({"items": ["a"]});
        let result = navigate_mut(&mut data, &["items".to_string(), "99".to_string()], "/items/99", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of bounds"));
    }

    #[test]
    fn test_pointer_get_mut_array() {
        let mut data = serde_json::json!({"items": ["x", "y"]});
        let node = data.as_object_mut().unwrap().get_mut("items").unwrap();
        let result = pointer_get_mut(node, "1", "/items/1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_pointer_get_mut_scalar_error() {
        let mut data = serde_json::json!(42_i64);
        let result = pointer_get_mut(&mut data, "x", "/x");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot navigate into scalar"));
    }

    #[test]
    fn test_pointer_get_mut_key_not_found() {
        let mut data = serde_json::json!({"a": 1});
        let result = pointer_get_mut(&mut data, "missing", "/missing");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_pointer_get_mut_array_out_of_bounds() {
        let mut data = serde_json::json!(["x"]);
        let result = pointer_get_mut(&mut data, "5", "/5");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of bounds"));
    }

    #[test]
    fn test_parse_index_non_integer() {
        let result = parse_index("abc", "/items/abc");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not an integer"));
    }

    #[test]
    fn test_unescape_pointer_tilde_sequences() {
        assert_eq!(unescape_pointer("a~1b"), "a/b");
        assert_eq!(unescape_pointer("a~0b"), "a~b");
        assert_eq!(unescape_pointer("~01"), "~1");
    }

    #[test]
    fn test_resolve_uri_impl_absolute_passthrough() {
        let result = resolve_uri_impl("https://example.com/base.yaml", "local/base", false);
        assert_eq!(result.unwrap(), "https://example.com/base.yaml");
    }

    #[test]
    fn test_resolve_uri_impl_registry_error() {
        let result = resolve_uri_impl("registry://foo/bar", "local/base", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("registry://"));
    }

    #[test]
    fn test_deep_merge_null_removes_key() {
        let base = serde_json::json!({"a": 1, "b": 2});
        let overlay = serde_json::json!({"b": null});
        let merged = deep_merge(base, overlay);
        assert!(merged.get("a").is_some());
        assert!(merged.get("b").is_none());
    }

    #[test]
    fn test_deep_merge_non_object() {
        let base = serde_json::json!([1, 2, 3]);
        let overlay = serde_json::json!([4, 5]);
        let merged = deep_merge(base, overlay);
        assert_eq!(merged, serde_json::json!([4, 5]));
    }

    #[test]
    fn test_additive_merge_array_with_non_array_module_wins() {
        let base = serde_json::json!({"items": ["a", "b"]});
        let module = serde_json::json!({"items": "not-an-array"});
        let merged = additive_merge(base, module);
        assert_eq!(merged["items"], "not-an-array");
    }

    #[test]
    fn test_additive_merge_object_with_non_object_module_wins() {
        let base = serde_json::json!({"nested": {"x": 1}});
        let module = serde_json::json!({"nested": "scalar"});
        let merged = additive_merge(base, module);
        assert_eq!(merged["nested"], "scalar");
    }

    #[test]
    fn test_additive_merge_non_object_module_wins() {
        let base = serde_json::json!([1, 2]);
        let module = serde_json::json!([3, 4]);
        let merged = additive_merge(base, module);
        assert_eq!(merged, serde_json::json!([3, 4]));
    }

    #[test]
    fn test_depth_exceeded_returns_error() {
        let mut files: HashMap<String, String> = HashMap::new();
        // Chain of 12 manifests: d0 extends d1 extends d2 ... extends d11
        for i in 0..12usize {
            let next = i + 1;
            let yaml = if i < 11 {
                format!(
                    r#"adp_version: "0.1.0"
id: "d{i}"
extends: "d{next}"
runtime:
  execution:
    - id: "r1"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes: [{{id: "n", kind: "input"}}]
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - id: "m"
          type: "deterministic"
          function: "noop"
          scoring: "boolean"
          threshold: true
"#
                )
            } else {
                minimal_manifest(&format!("d{i}"))
            };
            files.insert(format!("d{i}"), yaml);
        }
        let result = resolve_adp("d0", Some(make_resolver(files)));
        assert!(result.is_err());
        let combined = result.unwrap_err().join(" ");
        assert!(combined.contains("depth"), "Expected depth error, got: {combined}");
    }

    #[test]
    fn test_override_append_non_array_returns_error() {
        let data = serde_json::json!({"id": "x"});
        let result = apply_override(data, "/id", "append", Some(serde_json::json!("extra")));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not resolve to an array"));
    }

    #[test]
    fn test_resolve_adp_filesystem_happy_path() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, "{}",
            r#"adp_version: "0.1.0"
id: "fs-test"
runtime:
  execution:
    - id: "r1"
      backend: "python"
      entrypoint: "agent.main:app"
flow:
  id: "fs-test.flow"
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
        ).unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        let result = resolve_adp(&path, None);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
        assert_eq!(result.unwrap().id, "fs-test");
    }

    #[test]
    fn test_resolve_adp_filesystem_validation_error() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, "{}",
            r#"adp_version: "0.1.0"
id: "invalid-fs"
runtime:
  execution: []
flow: {}
evaluation: {}
"#
        ).unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        let result = resolve_adp(&path, None);
        assert!(result.is_err(), "Expected validation error for empty execution");
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_canonicalize_path_relative_existing() {
        let result = canonicalize_path(".");
        assert!(result.is_ok(), "canonicalize_path('.') should succeed");
    }

    #[test]
    fn test_canonicalize_path_relative_nonexistent() {
        let result = canonicalize_path("__adp_no_such_path_xyz_abc_123__");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot canonicalize"));
    }

    // covers additive_merge Some(_) arm (line 188): scalar existing value in base
    #[test]
    fn test_additive_merge_scalar_existing_module_wins() {
        let base = serde_json::json!({"name": "old-name", "count": 1});
        let module = serde_json::json!({"name": "new-name"});
        let merged = additive_merge(base, module);
        assert_eq!(merged["name"], "new-name");
        assert_eq!(merged["count"], 1);
    }

    // covers additive_merge None arm (line 189): key present in module but not in base
    #[test]
    fn test_additive_merge_new_key_from_module() {
        let base = serde_json::json!({"existing": "value"});
        let module = serde_json::json!({"new_key": "added"});
        let merged = additive_merge(base, module);
        assert_eq!(merged["new_key"], "added");
        assert_eq!(merged["existing"], "value");
    }

    // covers import section filter: no sections key (line 119 false branch)
    #[test]
    fn test_import_no_sections_key() {
        let mut files: HashMap<String, String> = HashMap::new();
        let main_yaml = r#"adp_version: "0.1.0"
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
    - id: "s1"
      metrics:
        - id: "m1"
          type: "deterministic"
          function: "noop"
          scoring: "boolean"
          threshold: true
import:
  - id: "mod"
    from: "module"
"#
        .to_string();
        let module_yaml = r#"id: "module""#.to_string();
        files.insert("main".to_string(), main_yaml);
        files.insert("module".to_string(), module_yaml);
        let result = resolve_adp("main", Some(make_resolver(files)));
        assert!(result.is_ok(), "import without sections should succeed: {:?}", result.err());
    }

    // covers import section filter: sections: [] empty array (line 118 false branch)
    #[test]
    fn test_import_empty_sections_array() {
        let mut files: HashMap<String, String> = HashMap::new();
        let main_yaml = r#"adp_version: "0.1.0"
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
    - id: "s1"
      metrics:
        - id: "m1"
          type: "deterministic"
          function: "noop"
          scoring: "boolean"
          threshold: true
import:
  - id: "mod"
    from: "module"
    sections: []
"#
        .to_string();
        let module_yaml = r#"evaluation:
  suites:
    - id: "s2"
      metrics:
        - id: "m2"
          type: "deterministic"
          function: "noop"
          scoring: "boolean"
          threshold: true
"#
        .to_string();
        files.insert("main".to_string(), main_yaml);
        files.insert("module".to_string(), module_yaml);
        let result = resolve_adp("main", Some(make_resolver(files)));
        assert!(result.is_ok(), "import with empty sections should succeed: {:?}", result.err());
    }

    // covers import section filter: non-object module_raw (line 117 false branch)
    #[test]
    fn test_import_sections_with_scalar_module() {
        let mut files: HashMap<String, String> = HashMap::new();
        let main_yaml = r#"adp_version: "0.1.0"
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
    - id: "s1"
      metrics:
        - id: "m1"
          type: "deterministic"
          function: "noop"
          scoring: "boolean"
          threshold: true
import:
  - id: "mod"
    from: "module"
    sections:
      - something
"#
        .to_string();
        // Module is a plain string (not an object) — as_object_mut() returns None
        let module_yaml = r#"just a string"#.to_string();
        files.insert("main".to_string(), main_yaml);
        files.insert("module".to_string(), module_yaml);
        // May succeed or fail validation; we just need the path to execute
        let _result = resolve_adp("main", Some(make_resolver(files)));
    }

    // covers apply_override delete: navigate_mut returns None (missing parent segment → line 299 None-branch)
    #[test]
    fn test_override_delete_missing_parent_is_noop() {
        let data = serde_json::json!({"a": 1});
        let result = apply_override(data, "/nonexistent/key", "delete", None);
        assert!(result.is_ok(), "delete with missing parent should be a no-op: {:?}", result);
    }

    // covers apply_override delete: target node is not an Object (line 224 false branch)
    #[test]
    fn test_override_delete_on_array_node_is_noop() {
        let data = serde_json::json!({"items": ["a", "b"]});
        let result = apply_override(data, "/items/1", "delete", None);
        assert!(result.is_ok());
    }

    // covers deep_merge recursive call (lines 155-157): both base and overlay have same key as Object
    #[test]
    fn test_deep_merge_nested_objects_recurse() {
        let base = serde_json::json!({"outer": {"a": 1, "b": 2}});
        let overlay = serde_json::json!({"outer": {"b": 99, "c": 3}});
        let result = deep_merge(base, overlay);
        assert_eq!(result["outer"]["a"], 1, "unpatched key must be kept");
        assert_eq!(result["outer"]["b"], 99, "patched key must be updated");
        assert_eq!(result["outer"]["c"], 3, "new key must be added via recursion");
    }

    // covers apply_patch `(_, patch) => patch` arm (line 204): patch is not an Object
    #[test]
    fn test_apply_patch_non_object_patch_wins() {
        let base = serde_json::json!({"a": 1});
        let result = apply_patch(base, serde_json::json!("scalar-wins"));
        assert_eq!(result, serde_json::json!("scalar-wins"));
    }

    // covers id_keyed_merge `None => result.push(patch_item)` (line 231): integer "id" field fails as_str()
    #[test]
    fn test_id_keyed_merge_non_string_id_appends() {
        let base_list = vec![serde_json::json!({"id": "existing", "val": "original"})];
        let patch_list = vec![serde_json::json!({"id": 42, "val": "int-id"})];
        let result = id_keyed_merge(base_list, patch_list);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2, "item with non-string id must be appended");
        assert_eq!(arr[0]["id"], "existing");
        assert_eq!(arr[1]["id"], 42);
    }

    // covers resolve_adp lines 44 (errors.push) and 51 (Err(errors)): validate_adp returns Err
    // for a manifest that deserializes successfully but has empty execution[]
    #[test]
    fn test_resolve_adp_validate_adp_error_path() {
        let mut files: HashMap<String, String> = HashMap::new();
        let yaml = r#"adp_version: "0.1.0"
id: "val-err-test"
runtime:
  execution: []
flow:
  id: "val-err.flow"
  graph:
    nodes: []
    edges: []
evaluation: {}
"#
        .to_string();
        files.insert("val-err-test".to_string(), yaml);
        let result = resolve_adp("val-err-test", Some(make_resolver(files)));
        assert!(result.is_err(), "empty execution should fail validate_adp");
        let errors = result.unwrap_err();
        let combined = errors.join(" ");
        assert!(combined.contains("execution"), "expected execution error, got: {combined}");
    }

    // covers apply_override set: parent node is scalar → _ arm (line 247)
    #[test]
    fn test_override_set_scalar_root_returns_error() {
        // Root is a scalar; navigate_mut with empty prefix returns the scalar itself as node.
        // match node { Object => ..., Array => ..., _ => Err } fires the _ arm.
        let data = serde_json::json!(42_i64);
        let result = apply_override(data, "/field", "set", Some(serde_json::json!("x")));
        assert!(result.is_err(), "set on scalar root should fail");
        assert!(result.unwrap_err().contains("cannot navigate path"));
    }

    // covers resolve_uri_impl with use_fs_canonicalize=true (lines 358-360)
    #[test]
    fn test_resolve_adp_filesystem_extends() {
        use std::io::Write;
        let dir = tempfile::TempDir::new().unwrap();
        let parent_path = dir.path().join("parent.yaml");
        let child_path = dir.path().join("child.yaml");

        std::fs::write(&parent_path, format!(r#"adp_version: "0.1.0"
id: "parent"
runtime:
  execution:
    - id: "r1"
      backend: "python"
      entrypoint: "agent.main:app"
flow:
  id: "parent.flow"
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
"#)).unwrap();

        // child extends parent via relative path — triggers resolve_uri_impl with canonicalize
        write!(
            std::fs::File::create(&child_path).unwrap(),
            r#"adp_version: "0.1.0"
id: "child"
extends: "parent.yaml"
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
        ).unwrap();

        let result = resolve_adp(child_path.to_str().unwrap(), None);
        assert!(result.is_ok(), "extends via filesystem should work: {:?}", result.err());
        assert_eq!(result.unwrap().id, "child");
    }

    // -----------------------------------------------------------------------
    // Id-keyed merge unit tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_apply_patch_object_deep_merge() {
        let base = serde_json::json!({"a": {"x": 1, "y": 2}, "b": "keep"});
        let patch = serde_json::json!({"a": {"x": 99}});
        let result = apply_patch(base, patch);
        assert_eq!(result["a"]["x"], 99);
        assert_eq!(result["a"]["y"], 2, "unpatched key must be kept");
        assert_eq!(result["b"], "keep");
    }

    #[test]
    fn test_apply_patch_adds_missing_key() {
        let base = serde_json::json!({"a": 1});
        let patch = serde_json::json!({"b": 2});
        let result = apply_patch(base, patch);
        assert_eq!(result["a"], 1);
        assert_eq!(result["b"], 2);
    }

    #[test]
    fn test_apply_patch_list_id_keyed_match() {
        let base = serde_json::json!({"models": [{"id": "gpt4", "model": "gpt-4"}, {"id": "claude", "model": "claude-3"}]});
        let patch = serde_json::json!({"models": [{"id": "gpt4", "model": "gpt-4o"}]});
        let result = apply_patch(base, patch);
        let models = result["models"].as_array().unwrap();
        assert_eq!(models.len(), 2, "both entries must be present");
        let gpt4 = models.iter().find(|m| m["id"] == "gpt4").unwrap();
        assert_eq!(gpt4["model"], "gpt-4o", "matched entry must be updated");
        let claude = models.iter().find(|m| m["id"] == "claude").unwrap();
        assert_eq!(claude["model"], "claude-3", "unmatched base entry must be kept");
    }

    #[test]
    fn test_apply_patch_list_id_keyed_new_entry() {
        let base = serde_json::json!({"models": [{"id": "gpt4", "model": "gpt-4"}]});
        let patch = serde_json::json!({"models": [{"id": "llama", "model": "llama-3"}]});
        let result = apply_patch(base, patch);
        let models = result["models"].as_array().unwrap();
        assert_eq!(models.len(), 2, "unknown id must be appended");
        assert!(models.iter().any(|m| m["id"] == "gpt4"));
        assert!(models.iter().any(|m| m["id"] == "llama"));
    }

    #[test]
    fn test_apply_patch_list_no_id_replaces() {
        let base = serde_json::json!({"tags": ["a", "b"]});
        let patch = serde_json::json!({"tags": ["c"]});
        let result = apply_patch(base, patch);
        let tags = result["tags"].as_array().unwrap();
        assert_eq!(tags.len(), 1, "list without id must replace entirely");
        assert_eq!(tags[0], "c");
    }

    #[test]
    fn test_apply_patch_null_removes_key() {
        let base = serde_json::json!({"a": 1, "b": 2});
        let patch = serde_json::json!({"b": null});
        let result = apply_patch(base, patch);
        assert_eq!(result["a"], 1);
        assert!(result.get("b").is_none() || result["b"].is_null());
    }

    #[test]
    fn test_extends_local_id_keyed_e2e() {
        // End-to-end: child uses local id-keyed list field to update one model entry.
        let mut files: HashMap<String, String> = HashMap::new();
        let base = r#"adp_version: "0.1.0"
id: "base"
runtime:
  execution:
    - id: "r1"
      backend: "python"
      entrypoint: "agent.main:app"
  models:
    - id: "gpt4"
      provider: "openai"
      model: "gpt-4"
    - id: "claude"
      provider: "anthropic"
      model: "claude-3"
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
"#.to_string();
        let child = r#"adp_version: "0.1.0"
id: "child"
extends: "base"
runtime:
  models:
    - id: "gpt4"
      model: "gpt-4o"
"#.to_string();
        files.insert("base".to_string(), base);
        files.insert("child".to_string(), child);

        let result = resolve_adp("child", Some(make_resolver(files)));
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
        let adp = result.unwrap();
        let models = adp.runtime.models.expect("models should be inherited");
        assert_eq!(models.len(), 2, "both base models must be present");
        let gpt4 = models.iter().find(|m| m.id == "gpt4").expect("gpt4 must be present");
        assert_eq!(gpt4.model, "gpt-4o", "gpt4 model must be updated");
        let claude = models.iter().find(|m| m.id == "claude").expect("claude must be present");
        assert_eq!(claude.model, "claude-3", "claude must be unchanged");
    }
}
