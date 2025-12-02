from __future__ import annotations

import json
from pathlib import Path
from typing import List

from jsonschema import Draft202012Validator, RefResolver

from .adp_model import ADP

# repo_root/sdk/python/adp_sdk -> parents[3] == repo root
SCHEMA_DIR = Path(__file__).resolve().parents[3] / "schemas"


def _load_schema(name: str) -> dict:
    return json.loads((SCHEMA_DIR / name).read_text())


def validate_adp(adp: ADP) -> List[str]:
    """Validate an ADP model against the JSON Schema.

    Supports both ADP-Minimal (allows empty flow/evaluation) and ADP-Full.
    """
    schema = _load_schema("adp.schema.json")
    base_uri = (SCHEMA_DIR / "adp.schema.json").resolve().as_uri()
    store = {
        (SCHEMA_DIR / "runtime.schema.json").resolve().as_uri(): _load_schema(
            "runtime.schema.json"
        ),
        (SCHEMA_DIR / "flow.schema.json").resolve().as_uri(): _load_schema(
            "flow.schema.json"
        ),
        (SCHEMA_DIR / "evaluation.schema.json").resolve().as_uri(): _load_schema(
            "evaluation.schema.json"
        ),
    }
    resolver = RefResolver(base_uri=base_uri, referrer=schema, store=store)
    validator = Draft202012Validator(schema, resolver=resolver)

    # Convert to dict for validation, excluding None values
    data = adp.model_dump(exclude_none=True)

    # Handle minimal conformance: empty flow/evaluation are allowed
    # Check if flow/evaluation are empty (minimal mode)
    flow_data = data.get("flow", {})
    is_minimal_flow = isinstance(flow_data, dict) and len(flow_data) == 0

    eval_data = data.get("evaluation", {})
    is_minimal_eval = isinstance(eval_data, dict) and len(eval_data) == 0

    # Create a modified copy of data for validation
    # Replace empty flow/evaluation with None to bypass schema validation
    validation_data = data.copy()
    if is_minimal_flow:
        # Temporarily remove flow from validation (will check separately)
        validation_data["flow"] = {}
    if is_minimal_eval:
        # Temporarily remove evaluation from validation (will check separately)
        validation_data["evaluation"] = {}

    # Validate ADP structure (excluding flow/evaluation if minimal)
    errors = []
    for error in validator.iter_errors(validation_data):
        error_path = "/".join(str(p) for p in error.path)
        # Skip flow/evaluation errors if they're empty (minimal mode)
        if is_minimal_flow and (
            "flow" in error_path.lower() or "flow" in error.message.lower()
        ):
            continue
        if is_minimal_eval and (
            "evaluation" in error_path.lower() or "evaluation" in error.message.lower()
        ):
            continue
        errors.append(error.message)

    # If flow/evaluation are not empty, validate them against their schemas
    if not is_minimal_flow:
        flow_schema = store[(SCHEMA_DIR / "flow.schema.json").resolve().as_uri()]
        flow_validator = Draft202012Validator(flow_schema, resolver=resolver)
        flow_errors = [e.message for e in flow_validator.iter_errors(flow_data)]
        errors.extend(flow_errors)

    if not is_minimal_eval:
        eval_schema = store[(SCHEMA_DIR / "evaluation.schema.json").resolve().as_uri()]
        eval_validator = Draft202012Validator(eval_schema, resolver=resolver)
        eval_errors = [e.message for e in eval_validator.iter_errors(eval_data)]
        errors.extend(eval_errors)

    return errors
