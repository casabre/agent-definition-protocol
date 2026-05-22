#!/usr/bin/env bash
set -euo pipefail

PYTHON_BIN="${PYTHON_BIN:-python}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
if [ -x "$SCRIPT_DIR/../.venv/bin/python" ]; then
  PYTHON_BIN="$SCRIPT_DIR/../.venv/bin/python"
fi

# Validate ADP, runtime, flow, evaluation, ACS, and adpkg-metadata manifests against schemas.
"$PYTHON_BIN" - <<'PY'
import json
import warnings
from pathlib import Path

warnings.filterwarnings("ignore", category=DeprecationWarning, message=".*RefResolver is deprecated.*")
import yaml
from jsonschema import Draft202012Validator
from referencing import Registry, Resource

here = Path(__file__).resolve()
root = here.parent.parent if here.exists() else Path.cwd()
schemas = {
    "adp": root / "schemas" / "adp.schema.json",
    "runtime": root / "schemas" / "runtime.schema.json",
    "flow": root / "schemas" / "flow.schema.json",
    "evaluation": root / "schemas" / "evaluation.schema.json",
    "acs": root / "schemas" / "acs.schema.json",
    "adpkg_metadata": root / "schemas" / "adpkg-metadata-v0.1.schema.json",
    "testing": root / "schemas" / "testing.schema.json",
}
examples = {
    "adp": [
        root / "examples" / "adp" / "acme-full-agent.yaml",
        root / "examples" / "minimal" / "acme-minimal.yaml",
        root / "examples" / "acme-analytics" / "adp" / "agent.yaml",
        root / "samples" / "python" / "langgraph" / "adp" / "agent.yaml",
        root / "fixtures" / "adp_full.yaml",
        root / "fixtures" / "adp_v0.1.0.yaml",
    ],
    "runtime": [root / "examples" / "runtime" / "acme-runtime-example.yaml"],
    "flow": [root / "examples" / "flow" / "acme-flow-example.yaml", root / "samples" / "python" / "langgraph" / "flow.yaml"],
    "evaluation": [root / "examples" / "evaluation" / "acme-eval-suite.yaml"],
    "acs": [root / "examples" / "acme-analytics" / "acs" / "container.yaml"],
    "adpkg_metadata": [root / "fixtures" / "adpkg_metadata_minimal.yaml"],
    "testing": [],  # tested separately below (x_testing block extraction)
}
negative = {
    "adp": [
        root / "fixtures" / "negative" / "invalid_adp_missing_runtime.yaml",
        root / "fixtures" / "negative" / "invalid_model_structured_output.yaml",
    ],
    "flow": [root / "fixtures" / "negative" / "invalid_flow_missing_id.yaml"],
    "evaluation": [root / "fixtures" / "negative" / "invalid_eval_bad_threshold.yaml"],
}

# Build a shared referencing Registry so that cross-schema $ref and internal
# #/definitions/... fragments all resolve correctly with Draft202012Validator.
resources = []
for p in schemas.values():
    sc = json.loads(p.read_text())
    resources.append(Resource.from_contents(sc))
registry = Registry().with_resources([(r.id(), r) for r in resources if r.id()])

for name, schema_path in schemas.items():
    schema = json.loads(schema_path.read_text())
    validator = Draft202012Validator(schema, registry=registry)
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

# 8 runner scenarios (D1–D8 including subflow)
"$PYTHON_BIN" scripts/esp-runner-harness.py --dry-run

echo "=== Composition smoke test ==="
"$PYTHON_BIN" -c "
import sys; sys.path.insert(0, 'sdk/python')
from adp_sdk.composition import resolve_adp
adp = resolve_adp('examples/composition/billing-variant.yaml')
assert adp.id == 'agent.acme.billing', f'unexpected id: {adp.id}'
print('OK: composition resolved', adp.id)
"

echo "=== x_testing block validation ==="
"$PYTHON_BIN" -c "
import yaml, json, sys, warnings
warnings.filterwarnings('ignore', category=DeprecationWarning, message='.*RefResolver is deprecated.*')
from jsonschema import Draft202012Validator
schema = json.loads(open('schemas/testing.schema.json').read())
data = yaml.safe_load(open('examples/testing/billing-test.yaml').read())
Draft202012Validator(schema).validate(data.get('x_testing', {}))
print('OK: x_testing block valid against testing.schema.json')

# Also validate the positive checkers fixture
data = yaml.safe_load(open('fixtures/testing_checkers.yaml').read())
Draft202012Validator(schema).validate(data.get('x_testing', {}))
print('OK: testing_checkers.yaml valid against testing.schema.json')

# Negative: aut adapter.type: http no longer valid
data = yaml.safe_load(open('fixtures/negative/invalid_testing_aut_http_type.yaml').read())
try:
    Draft202012Validator(schema).validate(data.get('x_testing', {}))
    raise SystemExit('ERROR: invalid_testing_aut_http_type.yaml should have failed')
except Exception as e:
    if 'SystemExit' in type(e).__name__:
        raise
    print('OK: correctly rejected invalid_testing_aut_http_type.yaml')
"
