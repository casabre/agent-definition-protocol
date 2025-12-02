"""Comprehensive validation tests for ADP SDK."""

from pathlib import Path
import pytest
from adp_sdk.adp_model import ADP, RuntimeModel, RuntimeEntry, FlowModel, EvaluationModel
from adp_sdk.validation import validate_adp


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
        adp_version="0.3.0",  # Invalid version (not 0.1.0 or 0.2.0)
        id="agent.test",
        runtime=RuntimeModel(execution=[
            RuntimeEntry(backend="python", id="py", entrypoint="main:app")
        ]),
        flow=FlowModel(),
        evaluation=EvaluationModel(),
    )
    errors = validate_adp(adp)
    assert len(errors) > 0, "Expected validation errors for invalid version"
    assert any("0.1.0" in err or "0.2.0" in err or "version" in err.lower() or "enum" in err.lower() for err in errors)


def test_validate_v0_2_0_adp():
    """Test validation of ADP v0.2.0 with ESP features."""
    adp = ADP(
        adp_version="0.2.0",
        id="agent.v0.2.0",
        runtime=RuntimeModel(execution=[
            RuntimeEntry(backend="python", id="py", entrypoint="main:app")
        ]),
        flow={
            "id": "test.flow",
            "graph": {
                "nodes": [
                    {"id": "input", "kind": "input"},
                    {"id": "llm", "kind": "llm", "model_ref": "primary"},
                    {"id": "tool", "kind": "tool", "tool_ref": "api"},
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
    assert len(errors) == 0, f"Expected no errors for v0.2.0, got: {errors}"


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


def test_validate_v0_2_0_fixture():
    """Test validation against v0.2.0 fixture file."""
    fixture_path = Path(__file__).resolve().parents[2].parent / "fixtures" / "adp_v0.2.0.yaml"
    if not fixture_path.exists():
        pytest.skip(f"Fixture not found: {fixture_path}")
    
    adp = ADP.from_file(fixture_path)
    errors = validate_adp(adp)
    assert len(errors) == 0, f"v0.2.0 fixture should be valid, got errors: {errors}"
    assert adp.id == "fixture.acme.v0.2.0"
    assert adp.adp_version == "0.2.0"
    # Verify v0.2.0 features are present
    flow_data = adp.flow if isinstance(adp.flow, dict) else adp.flow.model_dump()
    if isinstance(flow_data, dict) and "graph" in flow_data:
        nodes = flow_data.get("graph", {}).get("nodes", [])
        # Check for tool_ref and model_ref
        has_tool_ref = any(node.get("tool_ref") for node in nodes if isinstance(node, dict))
        has_model_ref = any(node.get("model_ref") for node in nodes if isinstance(node, dict))
        assert has_tool_ref or has_model_ref, "v0.2.0 fixture should have tool_ref or model_ref"


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

