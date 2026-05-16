from __future__ import annotations

import json
import warnings
from pathlib import Path
from typing import List

# RefResolver is deprecated in jsonschema 4.18+; suppress until migration to referencing
with warnings.catch_warnings():
    warnings.filterwarnings(
        "ignore",
        category=DeprecationWarning,
        message=".*RefResolver is deprecated.*",
    )
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

    data = adp.model_dump(exclude_none=True)

    # Conformance class enforcement
    conformance_class = data.get("conformance_class")
    flow_data = data.get("flow", {})
    eval_data = data.get("evaluation", {})
    is_flow_empty = isinstance(flow_data, dict) and len(flow_data) == 0
    is_eval_empty = isinstance(eval_data, dict) and len(eval_data) == 0

    if conformance_class == "full" and is_flow_empty:
        return ["conformance_class 'full' declared but flow is empty"]
    if conformance_class == "full" and is_eval_empty:
        return ["conformance_class 'full' declared but evaluation is empty"]

    # Determine minimal mode: explicit declaration or inferred from empty content
    is_minimal = conformance_class == "minimal" or (
        conformance_class is None and (is_flow_empty or is_eval_empty)
    )
    is_minimal_flow = is_minimal and is_flow_empty
    is_minimal_eval = is_minimal and is_eval_empty

    validation_data = data.copy()
    if is_minimal_flow:
        validation_data["flow"] = {}
    if is_minimal_eval:
        validation_data["evaluation"] = {}

    errors = []
    for error in validator.iter_errors(validation_data):
        error_path = "/".join(str(p) for p in error.path)
        if is_minimal_flow and (
            "flow" in error_path.lower() or "flow" in error.message.lower()
        ):
            continue
        if is_minimal_eval and (
            "evaluation" in error_path.lower() or "evaluation" in error.message.lower()
        ):
            continue
        errors.append(error.message)

    if not is_minimal_flow:
        flow_schema = store[(SCHEMA_DIR / "flow.schema.json").resolve().as_uri()]
        flow_validator = Draft202012Validator(flow_schema, resolver=resolver)
        errors.extend(e.message for e in flow_validator.iter_errors(flow_data))

    if not is_minimal_eval:
        eval_schema = store[(SCHEMA_DIR / "evaluation.schema.json").resolve().as_uri()]
        eval_validator = Draft202012Validator(eval_schema, resolver=resolver)
        errors.extend(e.message for e in eval_validator.iter_errors(eval_data))

    return errors


def validate_adp_semantics(adp: ADP) -> List[str]:
    """Validate cross-schema semantic constraints not expressible in JSON Schema.

    Checks referential integrity: edge node refs, start/end node refs,
    duplicate node IDs, suite_ref, model_ref, runtime_ref.
    Returns a list of error strings (empty = valid).
    """
    data = adp.model_dump(exclude_none=True)
    errors: List[str] = []

    flow = data.get("flow", {})
    graph = flow.get("graph", {}) if isinstance(flow, dict) else {}
    nodes = graph.get("nodes", [])
    edges = graph.get("edges", [])

    node_ids: set = set()
    for node in nodes:
        nid = node.get("id", "")
        if nid in node_ids:
            errors.append(f"duplicate node id '{nid}' in graph.nodes")
        node_ids.add(nid)

    for edge in edges:
        frm = edge.get("from", "")
        to = edge.get("to", "")
        if frm not in node_ids:
            errors.append(f"edge from '{frm}' to '{to}': node '{frm}' not found in graph.nodes")
        if to not in node_ids:
            errors.append(f"edge from '{frm}' to '{to}': node '{to}' not found in graph.nodes")

    for nid in graph.get("start_nodes", []):
        if nid not in node_ids:
            errors.append(f"start_node '{nid}' not found in graph.nodes")
    for nid in graph.get("end_nodes", []):
        if nid not in node_ids:
            errors.append(f"end_node '{nid}' not found in graph.nodes")

    evaluation = data.get("evaluation", {})
    suites = evaluation.get("suites", []) if isinstance(evaluation, dict) else []
    suite_ids = {s.get("id") for s in suites}
    for node in nodes:
        suite_ref = node.get("suite_ref")
        if suite_ref and suite_ref not in suite_ids:
            errors.append(
                f"node '{node.get('id')}' suite_ref '{suite_ref}' not found in evaluation.suites"
            )

    runtime = data.get("runtime", {})
    models = runtime.get("models", [])
    if models:
        model_ids = {m.get("id") for m in models}
        for node in nodes:
            model_ref = node.get("model_ref")
            if model_ref and model_ref not in model_ids:
                errors.append(
                    f"node '{node.get('id')}' model_ref '{model_ref}' not found in runtime.models"
                )

    execution = runtime.get("execution", [])
    execution_ids = {e.get("id") for e in execution}
    for node in nodes:
        runtime_ref = node.get("runtime_ref")
        if runtime_ref and runtime_ref not in execution_ids:
            errors.append(
                f"node '{node.get('id')}' runtime_ref '{runtime_ref}' not found in runtime.execution"
            )

    return errors
