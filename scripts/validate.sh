#!/usr/bin/env bash
set -euo pipefail

# Validate ADP, runtime, flow, evaluation, minimal, and composition examples against schemas
python - <<'PY'
import json
from pathlib import Path

import yaml
from jsonschema import Draft202012Validator, RefResolver

root = Path(__file__).resolve().parents[1]
schemas = {
    "adp": root / "schemas" / "adp.schema.json",
    "runtime": root / "schemas" / "runtime.schema.json",
    "flow": root / "schemas" / "flow.schema.json",
    "evaluation": root / "schemas" / "evaluation.schema.json",
}
examples = {
    "adp": [
        root / "examples" / "adp" / "acme-full-agent.yaml",
        root / "examples" / "composition" / "acme-inheritance-example.yaml",
        root / "examples" / "minimal" / "acme-minimal.yaml",
    ],
    "runtime": [root / "examples" / "runtime" / "acme-runtime-example.yaml"],
    "flow": [root / "examples" / "flow" / "acme-flow-example.yaml"],
    "evaluation": [root / "examples" / "evaluation" / "acme-eval-suite.yaml"],
}

store = {p.resolve().as_uri(): json.loads(p.read_text()) for p in schemas.values()}

for name, schema_path in schemas.items():
    schema = json.loads(schema_path.read_text())
    base_uri = schema_path.resolve().as_uri()
    resolver = RefResolver(base_uri=base_uri, referrer=schema, store=store)
    validator = Draft202012Validator(schema, resolver=resolver)
    for doc in examples[name]:
        data = yaml.safe_load(doc.read_text())
        validator.validate(instance=data)
        print(f"Validated {doc.relative_to(root)} against {schema_path.name}")
PY
