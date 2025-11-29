"""Schema validation utilities for ADP and ACS."""
from __future__ import annotations

from pathlib import Path
from typing import Any

import jsonschema
import yaml


class ValidationError(Exception):
    """Raised when validation fails."""


def load_schema(schema_path: Path) -> dict[str, Any]:
    """Load a JSON Schema from disk."""
    return yaml.safe_load(Path(schema_path).read_text())


def validate_file(document_path: Path, schema_path: Path) -> None:
    """Validate a YAML or JSON file against a schema."""
    data = yaml.safe_load(Path(document_path).read_text())
    schema = load_schema(schema_path)
    try:
        jsonschema.validate(instance=data, schema=schema)
    except jsonschema.ValidationError as exc:  # pragma: no cover - simple surface
        raise ValidationError(str(exc)) from exc
