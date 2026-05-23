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


def test_extends_arrays_replace():
    """RFC 7396: evaluation.suites in child replaces base suites entirely (not appended)."""
    resolver = _make_resolver({"/base.yaml": _BASE_YAML, "/child_override_eval.yaml": _CHILD_OVERRIDE_EVAL_YAML})
    adp = resolve_adp("/child_override_eval.yaml", resolver=resolver)
    data = adp.model_dump(by_alias=True, exclude_none=True)
    suite_ids = [s["id"] for s in data.get("evaluation", {}).get("suites", [])]
    assert suite_ids == ["new-suite"], f"RFC 7396 must replace base array; got {suite_ids}"
    assert "safety" not in suite_ids, "base safety suite must be replaced, not appended"


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
