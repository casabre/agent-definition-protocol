"""Tests for ADP composition: extends, import, overrides."""
from __future__ import annotations

import pytest

from adp_sdk.composition import CompositionError, resolve_adp

FIXTURES = None


def _fixtures_dir():
    from pathlib import Path
    return (Path(__file__).resolve().parents[4] / "fixtures" / "composition")


def _make_resolver(files: dict) -> object:
    """Return a callable resolver backed by a dict mapping URI → YAML string."""
    def resolver(uri: str) -> str:
        if uri not in files:
            raise CompositionError(f"resolver: unknown URI: {uri!r}")
        return files[uri]

    return resolver


_BASE_YAML = """\
adp_version: "0.2.0"
id: "base"
runtime:
  execution:
    - { id: "py", backend: "python", entrypoint: "app:main" }
  models:
    - { id: "gpt4", provider: "openai", model: "gpt-4o-mini" }
flow:
  id: "base.flow"
  graph:
    nodes:
      - { id: "input",  kind: "input" }
      - { id: "output", kind: "output" }
    edges: []
    start_nodes: ["input"]
    end_nodes:   ["output"]
guardrails:
  input:
    - { id: "pii", provider: "guardrails-ai", policy_ref: "./pii.rail", mode: "block" }
  on_violation: "block"
evaluation:
  suites:
    - id: "safety"
      metrics:
        - { id: "safety-judge", type: "llm_judge", threshold: 0.9 }
"""

_CHILD_YAML = """\
adp_version: "0.2.0"
id: "child"
extends: "/base.yaml"
flow:
  id: "child.flow"
  graph:
    nodes:
      - { id: "in",  kind: "input" }
      - { id: "out", kind: "output" }
    edges: []
    start_nodes: ["in"]
    end_nodes:   ["out"]
"""

_CHILD_OVERRIDE_EVAL_YAML = """\
adp_version: "0.2.0"
id: "child-override-eval"
extends: "/base.yaml"
flow:
  id: "child2.flow"
  graph:
    nodes:
      - { id: "in",  kind: "input" }
      - { id: "out", kind: "output" }
    edges: []
    start_nodes: ["in"]
    end_nodes:   ["out"]
evaluation:
  suites:
    - id: "new-suite"
      metrics:
        - { id: "new-metric", type: "deterministic", threshold: 1.0 }
"""

_MODULE_YAML = """\
id: "evals-module"
evaluation:
  suites:
    - id: "accuracy"
      metrics:
        - { id: "factuality", type: "llm_judge", threshold: 0.85 }
"""

_IMPORT_YAML = """\
adp_version: "0.2.0"
id: "with-import"
runtime:
  execution:
    - { id: "py", backend: "python", entrypoint: "app:main" }
flow:
  id: "import.flow"
  graph:
    nodes:
      - { id: "input",  kind: "input" }
      - { id: "output", kind: "output" }
    edges: []
    start_nodes: ["input"]
    end_nodes:   ["output"]
evaluation:
  suites:
    - id: "local-suite"
      metrics:
        - { id: "local-metric", type: "deterministic", threshold: 1.0 }
import:
  - id: "evals"
    from: "/module.yaml"
"""

_OVERRIDE_YAML = """\
adp_version: "0.2.0"
id: "override-test"
extends: "/base.yaml"
overrides:
  - { path: "/id", value: "override-test", op: "set" }
"""

_IMPORT_SECTIONS_YAML = """\
adp_version: "0.2.0"
id: "sections-test"
runtime:
  execution:
    - { id: "py", backend: "python", entrypoint: "app:main" }
flow:
  id: "sec.flow"
  graph:
    nodes:
      - { id: "input",  kind: "input" }
      - { id: "output", kind: "output" }
    edges: []
    start_nodes: ["input"]
    end_nodes:   ["output"]
evaluation:
  suites:
    - id: "base-suite"
      metrics:
        - { id: "b", type: "deterministic", threshold: 1.0 }
import:
  - id: "evals"
    from: "/module.yaml"
    sections: ["evaluation"]
"""

_CYCLE_A_YAML = "adp_version: '0.2.0'\nid: 'cycle-a'\nextends: '/cycle_b.yaml'\n"
_CYCLE_B_YAML = "adp_version: '0.2.0'\nid: 'cycle-b'\nextends: '/cycle_a.yaml'\n"


def test_extends_merges_objects():
    resolver = _make_resolver({"/base.yaml": _BASE_YAML, "/child.yaml": _CHILD_YAML})
    adp = resolve_adp("/child.yaml", resolver=resolver)
    assert adp.id == "child"
    assert adp.guardrails is not None, "base guardrails should be inherited"
    assert len(adp.guardrails.input) == 1


def test_extends_id_list_unknown_entry_appended():
    """Id-keyed merge: child suite with unknown id is appended; base suite is kept."""
    resolver = _make_resolver({"/base.yaml": _BASE_YAML, "/child_override_eval.yaml": _CHILD_OVERRIDE_EVAL_YAML})
    adp = resolve_adp("/child_override_eval.yaml", resolver=resolver)
    data = adp.model_dump(by_alias=True, exclude_none=True)
    suite_ids = [s["id"] for s in data.get("evaluation", {}).get("suites", [])]
    assert "safety" in suite_ids, "base safety suite must be kept (unmatched base entries retained)"
    assert "new-suite" in suite_ids, "child new-suite must be appended (unknown id)"


def test_extends_cycle_detection():
    resolver = _make_resolver({"/cycle_a.yaml": _CYCLE_A_YAML, "/cycle_b.yaml": _CYCLE_B_YAML})
    with pytest.raises(CompositionError, match="circular"):
        resolve_adp("/cycle_a.yaml", resolver=resolver)


def test_extends_unresolvable_uri_raises():
    resolver = _make_resolver({"/child.yaml": _CHILD_YAML})
    with pytest.raises(CompositionError):
        resolve_adp("/child.yaml", resolver=resolver)


def test_import_additive_arrays():
    resolver = _make_resolver({"/import.yaml": _IMPORT_YAML, "/module.yaml": _MODULE_YAML})
    adp = resolve_adp("/import.yaml", resolver=resolver)
    eval_data = adp.model_dump(by_alias=True, exclude_none=True).get("evaluation", {})
    suite_ids = [s["id"] for s in eval_data.get("suites", [])]
    assert "local-suite" in suite_ids, "local suite must be preserved"
    assert "accuracy" in suite_ids, "imported suite must be appended"


def test_import_sections_filter():
    resolver = _make_resolver({"/sections.yaml": _IMPORT_SECTIONS_YAML, "/module.yaml": _MODULE_YAML})
    adp = resolve_adp("/sections.yaml", resolver=resolver)
    eval_data = adp.model_dump(by_alias=True, exclude_none=True).get("evaluation", {})
    suite_ids = [s["id"] for s in eval_data.get("suites", [])]
    assert "accuracy" in suite_ids


def test_overrides_set_value():
    resolver = _make_resolver({"/base.yaml": _BASE_YAML, "/override.yaml": _OVERRIDE_YAML})
    adp = resolve_adp("/override.yaml", resolver=resolver)
    assert adp.id == "override-test"


def test_overrides_delete_existing():
    yaml_src = """\
adp_version: "0.2.0"
id: "del-test"
extends: "/base.yaml"
overrides:
  - { path: "/guardrails/on_violation", op: "delete" }
"""
    resolver = _make_resolver({"/base.yaml": _BASE_YAML, "/del.yaml": yaml_src})
    adp = resolve_adp("/del.yaml", resolver=resolver)
    assert adp.guardrails is not None
    assert adp.guardrails.on_violation is None


def test_overrides_delete_missing_is_noop():
    yaml_src = """\
adp_version: "0.2.0"
id: "del-missing"
extends: "/base.yaml"
overrides:
  - { path: "/does_not_exist", op: "delete" }
"""
    resolver = _make_resolver({"/base.yaml": _BASE_YAML, "/del_missing.yaml": yaml_src})
    adp = resolve_adp("/del_missing.yaml", resolver=resolver)
    assert adp.id == "del-missing"


def test_overrides_set_missing_path_raises():
    yaml_src = """\
adp_version: "0.2.0"
id: "set-missing"
extends: "/base.yaml"
overrides:
  - { path: "/nonexistent_key", value: "x", op: "set" }
"""
    resolver = _make_resolver({"/base.yaml": _BASE_YAML, "/set_missing.yaml": yaml_src})
    with pytest.raises(CompositionError):
        resolve_adp("/set_missing.yaml", resolver=resolver)


def test_full_pipeline():
    resolver = _make_resolver({"/base.yaml": _BASE_YAML, "/child.yaml": _CHILD_YAML})
    adp = resolve_adp("/child.yaml", resolver=resolver)
    assert adp.id == "child"
    assert adp.adp_version in ("0.1.0", "0.2.0")


def test_fixture_files_resolve():
    """Smoke test: the on-disk fixture files resolve without error."""
    fixtures = _fixtures_dir()
    child = fixtures / "comp_child.yaml"
    if child.exists():
        adp = resolve_adp(child)
        assert adp.id == "fixture.comp.child"


def test_resolve_adp_raises_on_invalid_merged_result():
    """resolve_adp raises CompositionError when the merged manifest fails validation."""
    # Produce a merged YAML with an invalid id (empty string → fails schema validation)
    invalid_yaml = """\
adp_version: "0.1.0"
id: ""
runtime:
  execution:
    - { id: "py", backend: "python", entrypoint: "app:main" }
flow: {}
evaluation: {}
"""
    resolver = _make_resolver({"/invalid.yaml": invalid_yaml})
    with pytest.raises(CompositionError, match="invalid"):
        resolve_adp("/invalid.yaml", resolver=resolver)


def test_extends_depth_exceeded():
    """resolve_adp raises CompositionError when extends chain depth > 10."""
    # Build a chain: d12 extends d11 extends … extends d1 (12 levels → depth 11 at d1 → triggers)
    files: dict = {}
    for i in range(1, 13):
        if i == 1:
            content = """\
adp_version: "0.2.0"
id: "depth1"
runtime:
  execution:
    - { id: "py", backend: "python", entrypoint: "app:main" }
flow: {}
evaluation: {}
"""
        else:
            content = f"""\
adp_version: "0.2.0"
id: "depth{i}"
extends: "/d{i - 1}.yaml"
"""
        files[f"/d{i}.yaml"] = content
    resolver = _make_resolver(files)
    with pytest.raises(CompositionError, match="depth"):
        resolve_adp("/d12.yaml", resolver=resolver)


def test_deep_merge_null_removes_key():
    """_deep_merge: a null overlay value removes the key from the result."""
    from adp_sdk.composition import _deep_merge
    base = {"a": 1, "b": {"c": 2}}
    result = _deep_merge(base, {"a": None})
    assert "a" not in result
    assert result["b"]["c"] == 2


def test_deep_merge_nested_dicts():
    """_deep_merge: nested dict values are merged recursively (both have the key as dict)."""
    from adp_sdk.composition import _deep_merge
    base = {"outer": {"a": 1, "b": 2}}
    overlay = {"outer": {"b": 99, "c": 3}}
    result = _deep_merge(base, overlay)
    assert result == {"outer": {"a": 1, "b": 99, "c": 3}}


def test_additive_merge_scalar_collision():
    """_additive_merge: scalar collision → module value wins."""
    from adp_sdk.composition import _additive_merge
    base = {"x": 1}
    module = {"x": 99}
    result = _additive_merge(base, module)
    assert result["x"] == 99


def test_override_delete_navigates_missing_segment():
    """override delete with a missing intermediate segment is a no-op (returns unchanged data)."""
    from adp_sdk.composition import _apply_override
    data = {"a": {"b": "value"}}
    result = _apply_override(data, {"path": "/missing/b", "op": "delete"})
    assert result == data


def test_override_delete_removes_from_dict():
    """override delete on a dict key removes the key."""
    from adp_sdk.composition import _apply_override
    data = {"a": {"b": "value", "c": "keep"}}
    result = _apply_override(data, {"path": "/a/b", "op": "delete"})
    assert "b" not in result["a"]
    assert result["a"]["c"] == "keep"


def test_override_set_on_list_index():
    """override set with a list index replaces that list element."""
    from adp_sdk.composition import _apply_override
    data = {"items": ["a", "b", "c"]}
    result = _apply_override(data, {"path": "/items/1", "value": "X", "op": "set"})
    assert result["items"] == ["a", "X", "c"]


def test_override_set_on_non_navigable_raises():
    """override set into a scalar node raises CompositionError."""
    from adp_sdk.composition import _apply_override, CompositionError
    data = {"scalar": "value"}
    with pytest.raises(CompositionError, match="cannot navigate"):
        _apply_override(data, {"path": "/scalar/nested", "value": "x", "op": "set"})


def test_override_append():
    """override append adds value to an array."""
    from adp_sdk.composition import _apply_override
    data = {"items": ["a", "b"]}
    result = _apply_override(data, {"path": "/items", "value": "c", "op": "append"})
    assert result["items"] == ["a", "b", "c"]


def test_override_append_on_non_list_raises():
    """override append on a non-list raises CompositionError."""
    from adp_sdk.composition import _apply_override, CompositionError
    data = {"scalar": "value"}
    with pytest.raises(CompositionError, match="append"):
        _apply_override(data, {"path": "/scalar", "value": "x", "op": "append"})


def test_override_unknown_op_raises():
    """Unknown override op raises CompositionError."""
    from adp_sdk.composition import _apply_override, CompositionError
    data = {"a": 1}
    with pytest.raises(CompositionError, match="unknown override op"):
        _apply_override(data, {"path": "/a", "value": 99, "op": "replace"})


def test_override_path_no_leading_slash_raises():
    """override path without leading '/' raises CompositionError."""
    from adp_sdk.composition import _apply_override, CompositionError
    data = {"a": 1}
    with pytest.raises(CompositionError, match="must start with"):
        _apply_override(data, {"path": "a", "value": 99, "op": "set"})


def test_pointer_get_list_index():
    """_pointer_get on a list with a numeric segment returns the element."""
    from adp_sdk.composition import _pointer_get
    node = ["x", "y", "z"]
    assert _pointer_get(node, "2", "/items/2") == "z"


def test_pointer_get_list_invalid_index_raises():
    """_pointer_get on a list with a non-integer segment raises CompositionError."""
    from adp_sdk.composition import _pointer_get, CompositionError
    node = ["x", "y"]
    with pytest.raises(CompositionError, match="not an integer"):
        _pointer_get(node, "notanint", "/items/notanint")


def test_pointer_get_non_navigable_raises():
    """_pointer_get on a scalar node raises CompositionError."""
    from adp_sdk.composition import _pointer_get, CompositionError
    with pytest.raises(CompositionError, match="cannot navigate"):
        _pointer_get("scalar_value", "key", "/some/path")


def test_to_index_invalid_raises():
    """_to_index raises CompositionError for a non-integer string."""
    from adp_sdk.composition import _to_index, CompositionError
    with pytest.raises(CompositionError, match="not an integer"):
        _to_index("abc", "/items/abc")


def test_load_uri_file_not_found():
    """_load_uri raises CompositionError for a non-existent local file."""
    from adp_sdk.composition import _load_uri, CompositionError
    with pytest.raises(CompositionError, match="cannot resolve URI"):
        _load_uri("/nonexistent_path_that_does_not_exist_xyz.yaml", None)


def test_additive_merge_new_key():
    """_additive_merge: new key in module is added to result (line 100 branch)."""
    from adp_sdk.composition import _additive_merge
    base = {"existing": 1}
    module = {"new_key": "value", "also_new": [1, 2, 3]}
    result = _additive_merge(base, module)
    assert result["existing"] == 1
    assert result["new_key"] == "value"
    assert result["also_new"] == [1, 2, 3]


def test_pointer_get_dict_missing_segment_raises():
    """_pointer_get on a dict with a missing key (not allow_missing) raises CompositionError."""
    from adp_sdk.composition import _pointer_get, CompositionError
    node = {"a": 1, "b": 2}
    with pytest.raises(CompositionError, match="not found"):
        _pointer_get(node, "missing_key", "/some/path", allow_missing=False)


def test_resolve_uri_file_scheme_returns_unchanged():
    """_resolve_uri with file:// scheme returns the URI unchanged."""
    from adp_sdk.composition import _resolve_uri
    uri = "file:///some/path/to/file.yaml"
    result = _resolve_uri(uri, "/base/path.yaml")
    assert result == uri


def test_resolve_uri_registry_scheme_raises():
    """_resolve_uri with registry:// scheme raises CompositionError."""
    from adp_sdk.composition import _resolve_uri, CompositionError
    with pytest.raises(CompositionError, match="registry://"):
        _resolve_uri("registry://some-agent/1.0", "/base/path.yaml")


# ---------------------------------------------------------------------------
# Id-keyed (structural) merge tests — local fields and _apply_patch
# ---------------------------------------------------------------------------

def test_local_object_deep_merge():
    """Local fields: object keys deep-merge; unmentioned keys are inherited."""
    from adp_sdk.composition import _apply_patch
    base = {"a": {"x": 1, "y": 2}, "b": "keep"}
    result = _apply_patch(base, {"a": {"x": 99}})
    assert result == {"a": {"x": 99, "y": 2}, "b": "keep"}


def test_local_adds_missing_key():
    """Local fields: key absent from base is added."""
    from adp_sdk.composition import _apply_patch
    base = {"existing": "value"}
    result = _apply_patch(base, {"new_key": {"nested": True}})
    assert result["new_key"] == {"nested": True}
    assert result["existing"] == "value"


def test_local_list_id_keyed_match():
    """Local id-carrying list: matched entry updated in-place; other base entries kept."""
    from adp_sdk.composition import _apply_patch
    base = {"models": [{"id": "gpt4", "model": "gpt-4"}, {"id": "claude", "model": "claude-3"}]}
    result = _apply_patch(base, {"models": [{"id": "gpt4", "model": "gpt-4o"}]})
    assert result["models"] == [{"id": "gpt4", "model": "gpt-4o"}, {"id": "claude", "model": "claude-3"}]


def test_local_list_id_keyed_new_entry():
    """Local id-carrying list: unknown id appended; existing entries untouched."""
    from adp_sdk.composition import _apply_patch
    base = {"models": [{"id": "gpt4", "model": "gpt-4"}]}
    result = _apply_patch(base, {"models": [{"id": "new-model", "model": "llama-3"}]})
    assert len(result["models"]) == 2
    assert result["models"][0] == {"id": "gpt4", "model": "gpt-4"}
    assert result["models"][1] == {"id": "new-model", "model": "llama-3"}


def test_local_list_no_id_replaces():
    """Local list without ids: entire base list replaced."""
    from adp_sdk.composition import _apply_patch
    base = {"tags": ["a", "b", "c"]}
    result = _apply_patch(base, {"tags": ["x", "y"]})
    assert result["tags"] == ["x", "y"]


def test_local_null_removes_key():
    """Local null value removes a key from the merged result."""
    from adp_sdk.composition import _apply_patch
    base = {"keep": "yes", "remove": "this"}
    result = _apply_patch(base, {"remove": None})
    assert "remove" not in result
    assert result["keep"] == "yes"


def test_extends_local_field_and_overrides():
    """Extends + local field (id-keyed merge) + override: override wins on same key."""
    import yaml as _yaml
    from adp_sdk.composition import _resolve_manifest
    base_yaml = _yaml.dump({
        "adp_version": "0.3.0", "id": "base",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "a:b"}]},
        "telemetry": {"service_name": "original", "protocol": "grpc"},
    })
    manifest = {
        "adp_version": "0.3.0", "id": "child",
        "extends": "/base.yaml",
        "telemetry": {"service_name": "local-value"},
        "overrides": [{"path": "/telemetry/service_name", "value": "overridden", "op": "set"}],
    }
    def resolver(uri: str) -> str:
        return base_yaml
    result = _resolve_manifest(manifest, "/child.yaml", set(), 0, resolver)
    assert result["telemetry"]["service_name"] == "overridden"
    assert result["telemetry"]["protocol"] == "grpc"


def test_extends_local_field_different_keys():
    """Extends + local field on different key from override: both applied correctly."""
    import yaml as _yaml
    from adp_sdk.composition import _resolve_manifest
    base_yaml = _yaml.dump({
        "adp_version": "0.3.0", "id": "base",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "a:b"}]},
        "telemetry": {"service_name": "base-name", "protocol": "grpc"},
    })
    manifest = {
        "adp_version": "0.3.0", "id": "child",
        "extends": "/base.yaml",
        "telemetry": {"service_name": "local-name"},
        "overrides": [{"path": "/telemetry/protocol", "value": "http/protobuf", "op": "set"}],
    }
    def resolver(uri: str) -> str:
        return base_yaml
    result = _resolve_manifest(manifest, "/child.yaml", set(), 0, resolver)
    assert result["telemetry"]["service_name"] == "local-name"
    assert result["telemetry"]["protocol"] == "http/protobuf"


def test_extends_id_list_full_pipeline():
    """Extends + local id-list + import + overrides: all active, correct result."""
    import yaml as _yaml
    from adp_sdk.composition import _resolve_manifest
    base = {
        "adp_version": "0.3.0", "id": "base",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "a:b"}]},
        "telemetry": {"service_name": "base-svc", "protocol": "grpc"},
        "models": [{"id": "m1", "provider": "openai", "model": "gpt-4"}],
    }
    module = {"models": [{"id": "m2", "provider": "anthropic", "model": "claude-3"}]}

    files = {
        "/base.yaml": _yaml.dump(base),
        "/module.yaml": _yaml.dump(module),
    }

    manifest = {
        "adp_version": "0.3.0", "id": "child",
        "extends": "/base.yaml",
        "import": [{"id": "extra-models", "from": "/module.yaml"}],
        "telemetry": {"service_name": "local-svc"},
        "overrides": [{"path": "/telemetry/protocol", "value": "http/protobuf", "op": "set"}],
    }

    def resolver(uri: str) -> str:
        return files[uri]

    result = _resolve_manifest(manifest, "/child.yaml", set(), 0, resolver)
    assert result["telemetry"]["service_name"] == "local-svc"
    assert result["telemetry"]["protocol"] == "http/protobuf"
    assert any(m.get("id") == "m2" for m in result["models"])
