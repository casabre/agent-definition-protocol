#!/usr/bin/env bash
set -euo pipefail

PYTHON_BIN="${PYTHON_BIN:-python}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
if [ -x "$SCRIPT_DIR/../.venv/bin/python" ]; then
  PYTHON_BIN="$SCRIPT_DIR/../.venv/bin/python"
fi

# Validate ADP, runtime, flow, evaluation, minimal, and sample manifests against schemas.
"$PYTHON_BIN" - <<'PY'
import json
from pathlib import Path

import yaml
from jsonschema import Draft202012Validator, RefResolver

here = Path(__file__).resolve()
root = here.parent.parent if here.exists() else Path.cwd()
schemas = {
    "adp": root / "schemas" / "adp.schema.json",
    "runtime": root / "schemas" / "runtime.schema.json",
    "flow": root / "schemas" / "flow.schema.json",
    "evaluation": root / "schemas" / "evaluation.schema.json",
}
examples = {
    "adp": [
        root / "examples" / "adp" / "acme-full-agent.yaml",
        root / "examples" / "minimal" / "acme-minimal.yaml",
        root / "examples" / "acme-analytics" / "adp" / "agent.yaml",
        root / "samples" / "python" / "langgraph" / "adp" / "agent.yaml",
        root / "fixtures" / "adp_full.yaml",
    ],
    "runtime": [root / "examples" / "runtime" / "acme-runtime-example.yaml"],
    "flow": [root / "examples" / "flow" / "acme-flow-example.yaml", root / "samples" / "python" / "langgraph" / "flow.yaml"],
    "evaluation": [root / "examples" / "evaluation" / "acme-eval-suite.yaml"],
}
negative = {
    "adp": [root / "fixtures" / "negative" / "invalid_adp_missing_runtime.yaml"],
    "flow": [root / "fixtures" / "negative" / "invalid_flow_missing_id.yaml"],
    "evaluation": [root / "fixtures" / "negative" / "invalid_eval_bad_threshold.yaml"],
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
    for doc in negative.get(name, []):
        data = yaml.safe_load(doc.read_text())
        try:
            validator.validate(instance=data)
        except Exception:
            print(f"Correctly failed {doc.relative_to(root)} against {schema_path.name}")
        else:
            raise SystemExit(f"Negative fixture unexpectedly passed: {doc}")
PY
