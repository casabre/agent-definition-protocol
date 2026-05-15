#!/usr/bin/env python3
"""
Verify that flow.schema.json accepts a minimal manifest for each of the
7 in-scope node kinds and that the per-node fields required by the ESP
node semantics ADR are present in the schema.

This script catches spec/schema drift without requiring a running runner.
It does NOT test execution semantics.
"""
import json
import sys
import warnings
from pathlib import Path

warnings.filterwarnings("ignore", category=DeprecationWarning, message=".*RefResolver is deprecated.*")
from jsonschema import Draft202012Validator, RefResolver

ROOT = Path(__file__).resolve().parent.parent
FLOW_SCHEMA_PATH = ROOT / "schemas" / "flow.schema.json"

# One minimal flow manifest per in-scope node kind.
# Each manifest must validate against flow.schema.json.
NODE_FIXTURES = {
    "input": {
        "id": "test.input",
        "graph": {
            "nodes": [
                {"id": "start", "kind": "input"},
                {"id": "done", "kind": "output"},
            ],
            "edges": [{"from": "start", "to": "done"}],
            "start_nodes": ["start"],
            "end_nodes": ["done"],
        },
    },
    "output": {
        "id": "test.output",
        "graph": {
            "nodes": [
                {"id": "start", "kind": "input"},
                {"id": "done", "kind": "output", "output_ref": "answer"},
            ],
            "edges": [{"from": "start", "to": "done"}],
            "start_nodes": ["start"],
            "end_nodes": ["done"],
        },
    },
    "llm": {
        "id": "test.llm",
        "graph": {
            "nodes": [
                {"id": "start", "kind": "input"},
                {"id": "answer", "kind": "llm", "model_ref": "primary"},
                {"id": "done", "kind": "output", "output_ref": "answer"},
            ],
            "edges": [
                {"from": "start", "to": "answer"},
                {"from": "answer", "to": "done"},
            ],
            "start_nodes": ["start"],
            "end_nodes": ["done"],
        },
    },
    "tool": {
        "id": "test.tool",
        "graph": {
            "nodes": [
                {"id": "start", "kind": "input"},
                {"id": "fetch", "kind": "tool", "tool_ref": "my-api"},
                {"id": "done", "kind": "output"},
            ],
            "edges": [
                {"from": "start", "to": "fetch"},
                {"from": "fetch", "to": "done"},
            ],
            "start_nodes": ["start"],
            "end_nodes": ["done"],
        },
    },
    "router": {
        "id": "test.router",
        "graph": {
            "nodes": [
                {"id": "start", "kind": "input"},
                {"id": "route", "kind": "router", "strategy": "conditional"},
                {"id": "done", "kind": "output"},
            ],
            "edges": [
                {"from": "start", "to": "route"},
                {"from": "route", "to": "done", "condition": "$.context.score > 0"},
            ],
            "start_nodes": ["start"],
            "end_nodes": ["done"],
        },
    },
    "retriever": {
        "id": "test.retriever",
        "graph": {
            "nodes": [
                {"id": "start", "kind": "input"},
                {"id": "retrieve", "kind": "retriever", "memory_ref": "vector-store"},
                {"id": "done", "kind": "output"},
            ],
            "edges": [
                {"from": "start", "to": "retrieve"},
                {"from": "retrieve", "to": "done"},
            ],
            "start_nodes": ["start"],
            "end_nodes": ["done"],
        },
    },
    "evaluator": {
        "id": "test.evaluator",
        "graph": {
            "nodes": [
                {"id": "start", "kind": "input"},
                {"id": "check", "kind": "evaluator", "suite_ref": "groundedness", "blocking": False},
                {"id": "done", "kind": "output"},
            ],
            "edges": [
                {"from": "start", "to": "check"},
                {"from": "check", "to": "done"},
            ],
            "start_nodes": ["start"],
            "end_nodes": ["done"],
        },
    },
}

# Fields that the ADR requires to be present in the schema node definition.
# Keyed by ADR decision; each value is the schema property name.
REQUIRED_SCHEMA_FIELDS = {
    "D2 llm writes to context[node.id]": "model_ref",
    "D3 tool appends to tool_responses[node.id]": "tool_ref",
    "D4 router strategy enum": "strategy",
    "D5 retriever uses memory_ref": "memory_ref",
    "D6 evaluator uses suite_ref": "suite_ref",
    "D6 evaluator blocking field": "blocking",
    "D7 output uses output_ref": "output_ref",
}


def load_validator():
    schema = json.loads(FLOW_SCHEMA_PATH.read_text())
    base_uri = FLOW_SCHEMA_PATH.resolve().as_uri()
    resolver = RefResolver(base_uri=base_uri, referrer=schema)
    return Draft202012Validator(schema, resolver=resolver), schema


def check_schema_fields(schema):
    """Assert that all ADR-required fields exist in the schema node definition."""
    node_def = schema.get("definitions", {}).get("node", {}).get("properties", {})
    failures = []
    for decision, field in REQUIRED_SCHEMA_FIELDS.items():
        if field not in node_def:
            failures.append(f"  MISSING field '{field}' in schema node definition  [{decision}]")
    return failures


def main():
    validator, schema = load_validator()
    errors = []

    # 1. Check that ADR-required fields are present in the schema
    field_errors = check_schema_fields(schema)
    if field_errors:
        errors.append("Schema field check failed:")
        errors.extend(field_errors)
    else:
        print("Schema field check: all ADR-required fields present")

    # 2. Validate each node kind fixture
    for kind, fixture in NODE_FIXTURES.items():
        validation_errors = list(validator.iter_errors(fixture))
        if validation_errors:
            errors.append(f"  FAIL  {kind}: {validation_errors[0].message}")
        else:
            print(f"  PASS  {kind}")

    if errors:
        print("\nFAILURES:")
        for e in errors:
            print(e)
        sys.exit(1)
    else:
        print("\nAll node kind fixtures validate. Schema is consistent with ADR.")


if __name__ == "__main__":
    main()
