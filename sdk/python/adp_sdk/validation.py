from __future__ import annotations

import json
import re
import urllib.parse as _urlparse
from pathlib import Path
from typing import List

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

from .adp_model import ADP

# repo_root/sdk/python/adp_sdk -> parents[3] == repo root
SCHEMA_DIR = Path(__file__).resolve().parents[3] / "schemas"

_SCHEMA_FILES = [
    "adp.schema.json",
    "runtime.schema.json",
    "flow.schema.json",
    "evaluation.schema.json",
]


def _load_schema(name: str) -> dict:
    return json.loads((SCHEMA_DIR / name).read_text())


def _build_registry() -> Registry:
    resources = []
    for name in _SCHEMA_FILES:
        contents = _load_schema(name)
        resources.append(Resource.from_contents(contents))
    return Registry().with_resources([(r.id(), r) for r in resources if r.id()])


def validate_adp(adp: ADP) -> List[str]:
    """Validate an ADP model against the JSON Schema.

    Supports both ADP-Minimal (allows empty flow/evaluation) and ADP-Full.
    """
    schema = _load_schema("adp.schema.json")
    registry = _build_registry()
    validator = Draft202012Validator(schema, registry=registry)

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
        flow_schema = _load_schema("flow.schema.json")
        flow_validator = Draft202012Validator(flow_schema, registry=registry)
        errors.extend(e.message for e in flow_validator.iter_errors(flow_data))

    if not is_minimal_eval:
        eval_schema = _load_schema("evaluation.schema.json")
        eval_validator = Draft202012Validator(eval_schema, registry=registry)
        errors.extend(e.message for e in eval_validator.iter_errors(eval_data))

    return errors


_GEN_AI_ATTR_RE = re.compile(r"^gen_ai\.[a-z0-9_.]+$|^x_[a-z0-9]+\.[a-z0-9_.]+$")
_KNOWN_COMPLIANCE_STANDARDS = {"gdpr", "hipaa", "soc2", "eu-ai-act", "iso-27001", "fedramp"}


def validate_adp_semantics(adp: ADP) -> List[str]:
    """Validate cross-schema semantic constraints not expressible in JSON Schema.

    Checks referential integrity, governance rules, and v0.2.0 constraints.
    Returns a list of error strings (empty = valid).
    """
    data = adp.model_dump(by_alias=True, exclude_none=True)
    errors: List[str] = []

    if data.get("extends") or data.get("import"):
        errors.append(
            "WARNING: manifest has unresolved composition fields (extends/import); "
            "semantic validation may be incomplete — call resolve_adp() first"
        )

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

    # Check 7: guardrail policy_ref must be non-empty
    guardrails = data.get("guardrails", {})
    for rail_list_key in ("input", "output"):
        for rail in guardrails.get(rail_list_key, []):
            if not rail.get("policy_ref", "").strip():
                errors.append(f"guardrail '{rail.get('id', '?')}': policy_ref is empty")

    # Check 8: telemetry.required_attributes must match gen_ai.* or x_<vendor>.*
    telemetry = data.get("telemetry", {})
    for attr in telemetry.get("required_attributes", []):
        if not _GEN_AI_ATTR_RE.match(attr):
            errors.append(
                f"telemetry.required_attributes: '{attr}' is not a valid "
                f"gen_ai.* or x_<vendor>.* attribute"
            )

    # Check 9: tool auth.env_var required when scheme != "none"
    tools = data.get("tools", {})
    for tool_list_key in ("mcp_servers", "http_apis", "sql_functions"):
        for tool in tools.get(tool_list_key, []):
            auth = tool.get("auth", {})
            if auth and auth.get("scheme", "none") != "none":
                if not auth.get("env_var", "").strip():
                    errors.append(
                        f"tool '{tool.get('id', '?')}': auth.env_var required when scheme is not 'none'"
                    )

    # Check 10: compliance standard must be known or x_<vendor>.*
    governance = data.get("governance", {})
    for entry in governance.get("compliance", []):
        standard = entry.get("standard", "")
        if standard not in _KNOWN_COMPLIANCE_STANDARDS and not standard.startswith("x_"):
            errors.append(
                f"compliance standard '{standard}' is unknown; "
                f"use x_<vendor>.<name> for custom standards"
            )

    # Check 11: tool_ref must reference an existing tool ID
    all_tool_ids: set = set()
    for tool_list_key in ("mcp_servers", "http_apis", "sql_functions"):
        for tool in tools.get(tool_list_key, []):
            tid = tool.get("id")
            if tid:
                all_tool_ids.add(tid)
    for node in nodes:
        tool_ref = node.get("tool_ref")
        if tool_ref and tool_ref not in all_tool_ids:
            errors.append(
                f"node '{node.get('id')}' tool_ref '{tool_ref}' not found in tools"
            )

    # Check 12: hook node_filter must reference existing flow node IDs
    hooks = data.get("hooks", [])
    for hook in hooks:
        for filter_id in hook.get("node_filter", []):
            if filter_id not in node_ids:
                errors.append(
                    f"hook event '{hook.get('event', '?')}' node_filter '{filter_id}' "
                    f"does not reference a known flow node"
                )

    # Check 13: subflow adp_ref must resolve to a known subagents[] entry if it looks like a catalog ID
    subagents = data.get("subagents", [])
    subagent_ids = {s.get("id") for s in subagents}
    for node in nodes:
        if node.get("kind") == "subflow":
            adp_ref = node.get("adp_ref", "")
            if adp_ref:
                # If it doesn't look like a URI or relative path, treat it as a catalog ID
                parsed = _urlparse.urlparse(adp_ref)
                is_uri_or_path = bool(parsed.scheme) or "/" in adp_ref or adp_ref.endswith(".yaml") or adp_ref.endswith(".json")
                if not is_uri_or_path and adp_ref not in subagent_ids:
                    errors.append(
                        f"subflow node '{node.get('id')}' adp_ref '{adp_ref}' "
                        f"does not resolve to a known subagents[] entry"
                    )

    # Check 14: evaluator_ref must resolve to a known x_testing evaluator
    x_testing = data.get("x_testing", {})
    testing_evaluator_ids = set()
    for ev in x_testing.get("evaluators", []):
        eid = ev.get("id")
        if eid:
            testing_evaluator_ids.add(eid)
    for judge in x_testing.get("judges", []):
        jid = judge.get("id")
        if jid:
            testing_evaluator_ids.add(jid)
    for suite in suites:
        for metric in suite.get("metrics", []):
            evaluator_ref = metric.get("evaluator_ref")
            if evaluator_ref and testing_evaluator_ids and evaluator_ref not in testing_evaluator_ids:
                errors.append(
                    f"evaluator '{metric.get('id', '?')}' evaluator_ref '{evaluator_ref}' "
                    f"does not resolve to a known x_testing evaluator"
                )

    # Deprecation warning: judges[] without evaluators[]
    if x_testing.get("judges") and not x_testing.get("evaluators"):
        errors.append(
            "WARNING: x_testing.judges[] is deprecated; migrate to x_testing.evaluators[]"
        )

    return errors
