"""Schema validation utilities for ADP and ACS."""
from __future__ import annotations

from pathlib import Path
from typing import Any

import yaml
from jsonschema import Draft202012Validator, RefResolver


class ValidationError(Exception):
    """Raised when validation fails."""


def load_schema(schema_path: Path) -> dict[str, Any]:
    """Load a JSON Schema from disk."""
    return yaml.safe_load(Path(schema_path).read_text())


def validate_file(document_path: Path, schema_path: Path) -> None:
    """Validate a YAML or JSON file against a schema."""
    data = yaml.safe_load(Path(document_path).read_text())
    schema = load_schema(schema_path)
    base_uri = schema_path.resolve().as_uri()
    schema["$id"] = base_uri
    store: dict[str, Any] = {}
    refs = ["runtime.schema.json", "flow.schema.json", "evaluation.schema.json", "acs.schema.json"]
    for ref_name in refs:
        ref_path = schema_path.parent / ref_name
        if ref_path.exists():
            store[ref_path.resolve().as_uri()] = load_schema(ref_path)
    resolver = RefResolver(base_uri=base_uri, referrer=schema, store=store)
    validator = Draft202012Validator(schema, resolver=resolver)
    try:
        validator.validate(instance=data)
    except Exception as exc:  # pragma: no cover - simple surface
        raise ValidationError(str(exc)) from exc
