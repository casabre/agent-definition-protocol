"""Comprehensive validation tests for ADP SDK."""

from pathlib import Path
import pytest
from adp_sdk.adp_model import ADP, RuntimeModel, RuntimeEntry, FlowModel, EvaluationModel
from adp_sdk.validation import validate_adp, validate_adp_semantics


def test_validate_minimal_valid_adp():
    """Test validation of minimal valid ADP manifest."""
    adp = ADP(
        adp_version="0.1.0",
        id="agent.test",
        runtime=RuntimeModel(execution=[
            RuntimeEntry(backend="python", id="py", entrypoint="main:app")
        ]),
        flow=FlowModel(),
        evaluation=EvaluationModel(),
    )
    errors = validate_adp(adp)
    assert len(errors) == 0, f"Expected no errors, got: {errors}"


def test_validate_full_adp_with_optional_fields():
    """Test validation of ADP with all optional fields."""
    adp = ADP(
        adp_version="0.1.0",
        id="agent.full",
        name="Full Agent",
        description="A complete agent example",
        runtime=RuntimeModel(execution=[
            RuntimeEntry(
                backend="docker",
                id="docker-backend",
                image="acme/agent:1.0.0",
                entrypoint=["python", "-m", "app"],
                env={"LOG_LEVEL": "info"}
            )
        ]),
        flow=FlowModel(),
        evaluation=EvaluationModel(),
    )
    errors = validate_adp(adp)
    assert len(errors) == 0, f"Expected no errors, got: {errors}"
    assert adp.name == "Full Agent"
    assert adp.description == "A complete agent example"


def test_validate_missing_required_fields():
    """Test validation fails when required fields are missing."""
    # Missing runtime.execution
    adp = ADP(
        adp_version="0.1.0",
        id="agent.test",
        runtime=RuntimeModel(execution=[]),
        flow=FlowModel(),
        evaluation=EvaluationModel(),
    )
    errors = validate_adp(adp)
    assert len(errors) > 0, f"Expected validation errors for empty execution, got: {errors}"
    # Error message indicates empty array: "[] should be non-empty"
    assert any("non-empty" in err.lower() or "empty" in err.lower() or "minimum" in err.lower() for err in errors), \
        f"Expected validation error for empty execution array, got: {errors}"


def test_validate_invalid_adp_version():
    """Test validation fails with invalid adp_version."""
    adp = ADP(
        adp_version="9.9.9",  # Not in schema enum (0.1.0, 0.2.0, 0.3.0)
        id="agent.test",
        runtime=RuntimeModel(execution=[
            RuntimeEntry(backend="python", id="py", entrypoint="main:app")
        ]),
        flow=FlowModel(),
        evaluation=EvaluationModel(),
    )
    errors = validate_adp(adp)
    assert len(errors) > 0, "Expected validation errors for invalid version"
    assert any("9.9.9" in err or "not one of" in err or "enum" in err.lower() for err in errors)


def test_validate_v0_1_0_adp():
    """Test validation of ADP v0.1.0 manifest."""
    adp = ADP(
        adp_version="0.1.0",
        id="agent.v0.1.0",
        runtime=RuntimeModel(execution=[
            RuntimeEntry(backend="python", id="py", entrypoint="main:app")
        ]),
        flow={
            "id": "test.flow",
            "graph": {
                "nodes": [
                    {"id": "input", "kind": "input"},
                    {"id": "output", "kind": "output"}
                ],
                "edges": [],
                "start_nodes": ["input"],
                "end_nodes": ["output"]
            }
        },
        evaluation=EvaluationModel(),
    )
    errors = validate_adp(adp)
    assert len(errors) == 0, f"Expected no errors for v0.1.0, got: {errors}"


def test_validate_empty_id():
    """Test validation fails with empty id."""
    adp = ADP(
        adp_version="0.1.0",
        id="",  # Empty ID
        runtime=RuntimeModel(execution=[
            RuntimeEntry(backend="python", id="py", entrypoint="main:app")
        ]),
        flow=FlowModel(),
        evaluation=EvaluationModel(),
    )
    errors = validate_adp(adp)
    assert len(errors) > 0, f"Expected validation errors for empty id, got: {errors}"
    # Error message indicates empty string: "'' should be non-empty" or mentions minLength
    assert any("non-empty" in err.lower() or "minlength" in err.lower() or "minimum" in err.lower() for err in errors), \
        f"Expected validation error for empty id, got: {errors}"


def test_validate_multiple_backends():
    """Test validation with multiple runtime backends."""
    adp = ADP(
        adp_version="0.1.0",
        id="agent.multi",
        runtime=RuntimeModel(execution=[
            RuntimeEntry(backend="docker", id="docker", image="acme/agent:1.0"),
            RuntimeEntry(backend="python", id="python", entrypoint="main:app"),
            RuntimeEntry(backend="wasm", id="wasm", module="agent.wasm"),
        ]),
        flow=FlowModel(),
        evaluation=EvaluationModel(),
    )
    errors = validate_adp(adp)
    assert len(errors) == 0, f"Expected no errors for multi-backend, got: {errors}"
    assert len(adp.runtime.execution) == 3


def test_validate_from_file(tmp_path: Path):
    """Test loading and validating ADP from file."""
    agent_yaml = tmp_path / "agent.yaml"
    agent_yaml.write_text("""
adp_version: "0.1.0"
id: "agent.file"
name: "File Agent"
runtime:
  execution:
    - backend: "python"
      id: "py"
      entrypoint: "main:app"
flow: {}
evaluation: {}
""")
    adp = ADP.from_file(agent_yaml)
    assert adp.id == "agent.file"
    assert adp.name == "File Agent"
    errors = validate_adp(adp)
    assert len(errors) == 0, f"Expected no errors, got: {errors}"


def test_validate_to_yaml_roundtrip(tmp_path: Path):
    """Test round-trip: load -> validate -> save -> load."""
    original_yaml = """
adp_version: "0.1.0"
id: "agent.roundtrip"
runtime:
  execution:
    - backend: "python"
      id: "py"
      entrypoint: "main:app"
flow: {}
evaluation: {}
"""
    agent_yaml = tmp_path / "agent.yaml"
    agent_yaml.write_text(original_yaml)
    
    # Load
    adp1 = ADP.from_file(agent_yaml)
    errors1 = validate_adp(adp1)
    assert len(errors1) == 0
    
    # Save
    output_yaml = tmp_path / "output.yaml"
    adp1.to_yaml(output_yaml)
    assert output_yaml.exists()
    
    # Reload
    adp2 = ADP.from_file(output_yaml)
    errors2 = validate_adp(adp2)
    assert len(errors2) == 0
    assert adp1.id == adp2.id
    assert adp1.adp_version == adp2.adp_version


def test_validate_fixture_file():
    """Test validation against fixture file."""
    fixture_path = Path(__file__).resolve().parents[2].parent / "fixtures" / "adp_full.yaml"
    if not fixture_path.exists():
        pytest.skip(f"Fixture not found: {fixture_path}")
    
    adp = ADP.from_file(fixture_path)
    errors = validate_adp(adp)
    assert len(errors) == 0, f"Fixture should be valid, got errors: {errors}"
    assert adp.id == "fixture.acme.full"


def test_validate_v0_1_0_fixture():
    """Test validation against v0.1.0 fixture file."""
    fixture_path = Path(__file__).resolve().parents[2].parent / "fixtures" / "adp_v0.1.0.yaml"
    if not fixture_path.exists():
        pytest.skip(f"Fixture not found: {fixture_path}")
    
    adp = ADP.from_file(fixture_path)
    errors = validate_adp(adp)
    assert len(errors) == 0, f"v0.1.0 fixture should be valid, got errors: {errors}"
    assert adp.id == "fixture.acme.v0.1.0"
    assert adp.adp_version == "0.1.0"


def test_validate_backend_types():
    """Test validation with different backend types."""
    backends = ["docker", "wasm", "python", "typescript", "binary", "custom"]

    for backend in backends:
        adp = ADP(
            adp_version="0.1.0",
            id=f"agent.{backend}",
            runtime=RuntimeModel(execution=[
                RuntimeEntry(backend=backend, id=f"{backend}-id")
            ]),
            flow=FlowModel(),
            evaluation=EvaluationModel(),
        )
        errors = validate_adp(adp)
        # Backend type validation may or may not be in schema, so just check it doesn't crash
        assert isinstance(errors, list), f"Validation should return list for backend {backend}"


def test_semantic_validation_passes_for_full_fixture():
    """validate_adp_semantics returns no errors for a valid full fixture."""
    fixture_path = Path(__file__).resolve().parents[2].parent / "fixtures" / "adp_full.yaml"
    if not fixture_path.exists():
        pytest.skip(f"Fixture not found: {fixture_path}")
    adp = ADP.from_file(fixture_path)
    errors = validate_adp_semantics(adp)
    assert errors == [], f"Expected no semantic errors, got: {errors}"


def test_semantic_validation_rejects_dangling_edge():
    """validate_adp_semantics detects an edge referencing a nonexistent node."""
    fixture_path = Path(__file__).resolve().parents[2].parent / "fixtures" / "semantic" / "sem_neg_edge_dangling.yaml"
    if not fixture_path.exists():
        pytest.skip(f"Fixture not found: {fixture_path}")
    adp = ADP.from_file(fixture_path)
    errors = validate_adp_semantics(adp)
    assert any("ghost" in e for e in errors), f"Expected dangling edge error mentioning 'ghost', got: {errors}"


def test_semantic_validation_rejects_duplicate_node():
    """validate_adp_semantics detects duplicate node IDs."""
    fixture_path = Path(__file__).resolve().parents[2].parent / "fixtures" / "semantic" / "sem_neg_duplicate_node.yaml"
    if not fixture_path.exists():
        pytest.skip(f"Fixture not found: {fixture_path}")
    adp = ADP.from_file(fixture_path)
    errors = validate_adp_semantics(adp)
    assert any("duplicate" in e for e in errors), f"Expected duplicate node error, got: {errors}"


def test_semantic_validation_rejects_bad_suite_ref():
    """validate_adp_semantics detects suite_ref pointing to a nonexistent suite."""
    fixture_path = Path(__file__).resolve().parents[2].parent / "fixtures" / "semantic" / "sem_neg_suite_ref.yaml"
    if not fixture_path.exists():
        pytest.skip(f"Fixture not found: {fixture_path}")
    adp = ADP.from_file(fixture_path)
    errors = validate_adp_semantics(adp)
    assert any("suite_ref" in e for e in errors), f"Expected suite_ref error, got: {errors}"


def test_semantic_validation_rejects_bad_model_ref():
    """validate_adp_semantics detects model_ref pointing to a nonexistent model."""
    fixture_path = Path(__file__).resolve().parents[2].parent / "fixtures" / "semantic" / "sem_neg_model_ref.yaml"
    if not fixture_path.exists():
        pytest.skip(f"Fixture not found: {fixture_path}")
    adp = ADP.from_file(fixture_path)
    errors = validate_adp_semantics(adp)
    assert any("model_ref" in e for e in errors), f"Expected model_ref error, got: {errors}"


def test_semantic_validation_rejects_bad_runtime_ref():
    """validate_adp_semantics detects runtime_ref pointing to a nonexistent execution entry."""
    fixture_path = Path(__file__).resolve().parents[2].parent / "fixtures" / "semantic" / "sem_neg_runtime_ref.yaml"
    if not fixture_path.exists():
        pytest.skip(f"Fixture not found: {fixture_path}")
    adp = ADP.from_file(fixture_path)
    errors = validate_adp_semantics(adp)
    assert any("runtime_ref" in e for e in errors), f"Expected runtime_ref error, got: {errors}"


def test_validate_conformance_class_full_rejects_empty_flow():
    """validate_adp returns error when conformance_class=full but flow is empty."""
    adp = ADP(
        adp_version="0.1.0",
        id="agent.full-empty",
        conformance_class="full",
        runtime=RuntimeModel(execution=[
            RuntimeEntry(backend="python", id="py", entrypoint="main:app")
        ]),
        flow=FlowModel(),
        evaluation=EvaluationModel(),
    )
    errors = validate_adp(adp)
    assert any("full" in e and "flow" in e for e in errors), f"Expected conformance_class 'full' error, got: {errors}"


def test_semantic_validation_check12_hook_node_filter():
    """Check 12: node_filter referencing a nonexistent node ID is detected."""
    fixture_path = Path(__file__).resolve().parents[2].parent / "fixtures" / "semantic" / "sem_neg_hook_node_filter.yaml"
    if not fixture_path.exists():
        pytest.skip(f"Fixture not found: {fixture_path}")
    adp = ADP.from_file(fixture_path)
    errors = validate_adp_semantics(adp)
    assert any("node_filter" in e for e in errors), f"Expected node_filter error, got: {errors}"


def test_semantic_validation_check13_subagent_ref():
    """Check 13: subflow adp_ref that doesn't resolve to a known subagents[] entry is detected."""
    fixture_path = Path(__file__).resolve().parents[2].parent / "fixtures" / "semantic" / "sem_neg_subagent_ref.yaml"
    if not fixture_path.exists():
        pytest.skip(f"Fixture not found: {fixture_path}")
    adp = ADP.from_file(fixture_path)
    errors = validate_adp_semantics(adp)
    assert any("adp_ref" in e for e in errors), f"Expected adp_ref error, got: {errors}"


def test_semantic_validation_check14_evaluator_ref():
    """Check 14: evaluator_ref not matching any x_testing evaluator/judge ID is detected."""
    fixture_path = Path(__file__).resolve().parents[2].parent / "fixtures" / "semantic" / "sem_neg_evaluator_ref.yaml"
    if not fixture_path.exists():
        pytest.skip(f"Fixture not found: {fixture_path}")
    adp = ADP.from_file(fixture_path)
    errors = validate_adp_semantics(adp)
    assert any("evaluator_ref" in e for e in errors), f"Expected evaluator_ref error, got: {errors}"


def test_validate_conformance_class_full_rejects_empty_evaluation():
    """validate_adp returns error when conformance_class=full but evaluation is empty."""
    adp = ADP(
        adp_version="0.1.0",
        id="agent.full-no-eval",
        conformance_class="full",
        runtime=RuntimeModel(execution=[
            RuntimeEntry(backend="python", id="py", entrypoint="main:app")
        ]),
        flow={
            "id": "f",
            "graph": {
                "nodes": [{"id": "n", "kind": "input"}],
                "edges": [],
                "start_nodes": ["n"],
                "end_nodes": ["n"],
            }
        },
        evaluation=EvaluationModel(),
    )
    errors = validate_adp(adp)
    assert any("full" in e and "evaluation" in e for e in errors), \
        f"Expected conformance_class 'full' + empty evaluation error, got: {errors}"


def test_semantic_validation_warns_on_unresolved_extends():
    """validate_adp_semantics warns when manifest still has extends/import fields."""
    adp = ADP(
        adp_version="0.1.0",
        id="agent.unresolved",
        extends="/some/base.yaml",
        runtime=RuntimeModel(execution=[
            RuntimeEntry(backend="python", id="py", entrypoint="main:app")
        ]),
        flow=FlowModel(),
        evaluation=EvaluationModel(),
    )
    errors = validate_adp_semantics(adp)
    assert any("WARNING" in e and "extends" in e for e in errors), \
        f"Expected unresolved composition warning, got: {errors}"


def test_semantic_validation_duplicate_node_id():
    """validate_adp_semantics detects duplicate node IDs directly (no fixture needed)."""
    adp = ADP(
        adp_version="0.1.0",
        id="agent.dup-node",
        runtime=RuntimeModel(execution=[
            RuntimeEntry(backend="python", id="py", entrypoint="main:app")
        ]),
        flow={
            "id": "f",
            "graph": {
                "nodes": [{"id": "dup", "kind": "input"}, {"id": "dup", "kind": "output"}],
                "edges": [],
                "start_nodes": ["dup"],
                "end_nodes": ["dup"],
            }
        },
        evaluation=EvaluationModel(),
    )
    errors = validate_adp_semantics(adp)
    assert any("duplicate" in e for e in errors), f"Expected duplicate node error, got: {errors}"


def test_semantic_validation_edge_from_missing():
    """validate_adp_semantics detects edge where from-node doesn't exist."""
    adp = ADP(
        adp_version="0.1.0",
        id="agent.bad-edge-from",
        runtime=RuntimeModel(execution=[
            RuntimeEntry(backend="python", id="py", entrypoint="main:app")
        ]),
        flow={
            "id": "f",
            "graph": {
                "nodes": [{"id": "output", "kind": "output"}],
                "edges": [{"from": "ghost", "to": "output"}],
                "start_nodes": [],
                "end_nodes": ["output"],
            }
        },
        evaluation=EvaluationModel(),
    )
    errors = validate_adp_semantics(adp)
    assert any("ghost" in e for e in errors), f"Expected dangling edge error, got: {errors}"


def test_semantic_validation_edge_to_missing():
    """validate_adp_semantics detects edge where to-node doesn't exist."""
    adp = ADP(
        adp_version="0.1.0",
        id="agent.bad-edge-to",
        runtime=RuntimeModel(execution=[
            RuntimeEntry(backend="python", id="py", entrypoint="main:app")
        ]),
        flow={
            "id": "f",
            "graph": {
                "nodes": [{"id": "input", "kind": "input"}],
                "edges": [{"from": "input", "to": "nonexistent"}],
                "start_nodes": ["input"],
                "end_nodes": [],
            }
        },
        evaluation=EvaluationModel(),
    )
    errors = validate_adp_semantics(adp)
    assert any("nonexistent" in e for e in errors), f"Expected missing to-node error, got: {errors}"


def test_semantic_validation_bad_start_node():
    """validate_adp_semantics detects start_node not in graph.nodes."""
    adp = ADP(
        adp_version="0.1.0",
        id="agent.bad-start",
        runtime=RuntimeModel(execution=[
            RuntimeEntry(backend="python", id="py", entrypoint="main:app")
        ]),
        flow={
            "id": "f",
            "graph": {
                "nodes": [{"id": "real-node", "kind": "input"}],
                "edges": [],
                "start_nodes": ["ghost-start"],
                "end_nodes": ["real-node"],
            }
        },
        evaluation=EvaluationModel(),
    )
    errors = validate_adp_semantics(adp)
    assert any("start_node" in e and "ghost-start" in e for e in errors), \
        f"Expected start_node error, got: {errors}"


def test_semantic_validation_bad_end_node():
    """validate_adp_semantics detects end_node not in graph.nodes."""
    adp = ADP(
        adp_version="0.1.0",
        id="agent.bad-end",
        runtime=RuntimeModel(execution=[
            RuntimeEntry(backend="python", id="py", entrypoint="main:app")
        ]),
        flow={
            "id": "f",
            "graph": {
                "nodes": [{"id": "real-node", "kind": "output"}],
                "edges": [],
                "start_nodes": ["real-node"],
                "end_nodes": ["ghost-end"],
            }
        },
        evaluation=EvaluationModel(),
    )
    errors = validate_adp_semantics(adp)
    assert any("end_node" in e and "ghost-end" in e for e in errors), \
        f"Expected end_node error, got: {errors}"


def test_semantic_validation_empty_guardrail_policy_ref():
    """validate_adp_semantics detects guardrail with empty policy_ref."""
    import yaml as _yaml
    adp_yaml = """
adp_version: "0.1.0"
id: "agent.guardrail-empty-ref"
runtime:
  execution:
    - backend: python
      id: py
      entrypoint: main:app
flow: {}
evaluation: {}
guardrails:
  input:
    - id: rail1
      provider: guardrails-ai
      policy_ref: ""
      mode: block
  on_violation: block
"""
    adp = ADP.model_validate(_yaml.safe_load(adp_yaml))
    errors = validate_adp_semantics(adp)
    assert any("policy_ref" in e for e in errors), f"Expected policy_ref error, got: {errors}"


def test_semantic_validation_invalid_telemetry_attribute():
    """validate_adp_semantics detects invalid telemetry.required_attributes entry."""
    import yaml as _yaml
    adp_yaml = """
adp_version: "0.1.0"
id: "agent.telemetry"
runtime:
  execution:
    - backend: python
      id: py
      entrypoint: main:app
flow: {}
evaluation: {}
telemetry:
  required_attributes:
    - invalid_attr_name
"""
    adp = ADP.model_validate(_yaml.safe_load(adp_yaml))
    errors = validate_adp_semantics(adp)
    assert any("invalid_attr_name" in e for e in errors), \
        f"Expected telemetry attribute error, got: {errors}"


def test_semantic_validation_tool_auth_missing_env_var():
    """validate_adp_semantics detects tool with auth.scheme != 'none' but no env_var."""
    import yaml as _yaml
    adp_yaml = """
adp_version: "0.1.0"
id: "agent.tool-auth"
runtime:
  execution:
    - backend: python
      id: py
      entrypoint: main:app
flow: {}
evaluation: {}
tools:
  http_apis:
    - id: billing-api
      description: Billing
      base_url: https://billing.example
      auth:
        scheme: bearer
        env_var: ""
"""
    adp = ADP.model_validate(_yaml.safe_load(adp_yaml))
    errors = validate_adp_semantics(adp)
    assert any("env_var" in e for e in errors), f"Expected env_var error, got: {errors}"


def test_semantic_validation_unknown_compliance_standard():
    """validate_adp_semantics detects unknown compliance standard."""
    import yaml as _yaml
    adp_yaml = """
adp_version: "0.1.0"
id: "agent.compliance"
runtime:
  execution:
    - backend: python
      id: py
      entrypoint: main:app
flow: {}
evaluation: {}
governance:
  compliance:
    - standard: unknown-standard-xyz
      status: compliant
"""
    adp = ADP.model_validate(_yaml.safe_load(adp_yaml))
    errors = validate_adp_semantics(adp)
    assert any("unknown-standard-xyz" in e for e in errors), \
        f"Expected unknown compliance standard error, got: {errors}"


def test_semantic_validation_tool_ref_missing():
    """validate_adp_semantics detects node with tool_ref not in tools."""
    import yaml as _yaml
    adp_yaml = """
adp_version: "0.1.0"
id: "agent.tool-ref"
runtime:
  execution:
    - backend: python
      id: py
      entrypoint: main:app
flow:
  id: f
  graph:
    nodes:
      - id: input
        kind: input
      - id: fetch
        kind: tool
        tool_ref: nonexistent-tool
    edges: []
    start_nodes: [input]
    end_nodes: [fetch]
evaluation: {}
"""
    adp = ADP.model_validate(_yaml.safe_load(adp_yaml))
    errors = validate_adp_semantics(adp)
    assert any("tool_ref" in e for e in errors), f"Expected tool_ref error, got: {errors}"


def test_semantic_validation_judges_deprecation_warning():
    """validate_adp_semantics returns deprecation warning for judges[] without evaluators[]."""
    import yaml as _yaml
    adp_yaml = """
adp_version: "0.1.0"
id: "agent.judges"
runtime:
  execution:
    - backend: python
      id: py
      entrypoint: main:app
flow: {}
evaluation: {}
x_testing:
  judges:
    - id: j1
      type: llm_judge
      model: gpt-4o
"""
    adp = ADP.model_validate(_yaml.safe_load(adp_yaml))
    errors = validate_adp_semantics(adp)
    assert any("deprecated" in e.lower() or "judges" in e for e in errors), \
        f"Expected judges deprecation warning, got: {errors}"


def test_semantic_validation_evaluator_ref_via_judges():
    """validate_adp_semantics: evaluator_ref resolved via x_testing.judges[] IDs."""
    import yaml as _yaml
    # evaluator_ref matches a judge ID → no error
    adp_yaml = """
adp_version: "0.1.0"
id: "agent.judge-ref"
runtime:
  execution:
    - backend: python
      id: py
      entrypoint: main:app
flow: {}
evaluation:
  suites:
    - id: suite1
      metrics:
        - id: m1
          type: deterministic
          threshold: 1.0
          evaluator_ref: j1
x_testing:
  judges:
    - id: j1
      type: llm_judge
      model: gpt-4o
  evaluators:
    - id: ev1
      type: script
      runtime: python
      inline: "def evaluate(o, c): return True"
"""
    adp = ADP.model_validate(_yaml.safe_load(adp_yaml))
    errors = validate_adp_semantics(adp)
    # j1 is in judges AND evaluators are present, so evaluator_ref j1 should be resolved
    # The judge IDs are added to testing_evaluator_ids when evaluators also exist
    # evaluator_ref "j1" should be found in testing_evaluator_ids (populated from both)
    assert not any("evaluator_ref" in e for e in errors), \
        f"Expected evaluator_ref j1 to resolve via judges[], got: {errors}"

