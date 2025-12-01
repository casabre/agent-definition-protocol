from __future__ import annotations

import json
from pathlib import Path
from typing import List

import yaml
from jsonschema import Draft202012Validator, RefResolver

from .adp_model import ADP

# repo_root/sdk/python/adp_sdk -> parents[3] == repo root
SCHEMA_DIR = Path(__file__).resolve().parents[3] / "schemas"


def _load_schema(name: str) -> dict:
    return json.loads((SCHEMA_DIR / name).read_text())


def validate_adp(adp: ADP) -> List[str]:
    """Validate an ADP model against the JSON Schema."""
    schema = _load_schema("adp.schema.json")
    base_uri = (SCHEMA_DIR / "adp.schema.json").resolve().as_uri()
    store = {
        (SCHEMA_DIR / "runtime.schema.json").resolve().as_uri(): _load_schema("runtime.schema.json"),
        (SCHEMA_DIR / "flow.schema.json").resolve().as_uri(): _load_schema("flow.schema.json"),
        (SCHEMA_DIR / "evaluation.schema.json").resolve().as_uri(): _load_schema("evaluation.schema.json"),
    }
    resolver = RefResolver(base_uri=base_uri, referrer=schema, store=store)
    validator = Draft202012Validator(schema, resolver=resolver)
    errors = [e.message for e in validator.iter_errors(adp.model_dump())]
    return errors
