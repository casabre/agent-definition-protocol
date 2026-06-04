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
    "memory.schema.json",
    "workspace.schema.json",
    "sandbox.schema.json",
    "artifacts.schema.json",
    "observability.schema.json",
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

    data = adp.model_dump(exclude_none=True, mode='json')

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
    for tool_list_key in ("mcp_servers", "http_apis", "sql_functions", "sandbox"):
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

    # =========================================================================
    # v0.3.0 Semantic Validation Checks (15-35b)
    # =========================================================================

    # --- Loop Checks (15-16) ---

    # Check 15: loop.body_nodes[] must reference known node IDs in flow.graph.nodes[]
    loop_nodes = [n for n in nodes if n.get("kind") == "loop"]
    for loop_node in loop_nodes:
        body_nodes = loop_node.get("body_nodes", [])
        for body_node_id in body_nodes:
            if body_node_id not in node_ids:
                errors.append(
                    f"loop node '{loop_node.get('id')}': body_nodes references "
                    f"'{body_node_id}' which is not found in graph.nodes"
                )

    # Check 15b: loop.body_nodes[] must contain at least 2 nodes connected by
    # at least one edge in flow.graph.edges[]
    for loop_node in loop_nodes:
        body_nodes = loop_node.get("body_nodes", [])
        if len(body_nodes) >= 2:
            # Build adjacency from edges
            edge_map: dict[str, set[str]] = {}
            for edge in edges:
                frm = edge.get("from", "")
                to = edge.get("to", "")
                if frm not in edge_map:
                    edge_map[frm] = set()
                edge_map[frm].add(to)
            # Check if any body node connects to another body node
            has_connection = False
            for node_id in body_nodes:
                if node_id in edge_map:
                    connected = edge_map[node_id] & set(body_nodes)
                    if connected:
                        has_connection = True
                        break
            if not has_connection:
                errors.append(
                    f"loop node '{loop_node.get('id')}': body_nodes "
                    f"[{', '.join(body_nodes)}] must contain at least 2 nodes "
                    f"connected by at least one edge"
                )

    # Check 16: Loop node MUST NOT reference itself (directly or transitively) in body_nodes
    for loop_node in loop_nodes:
        loop_id = loop_node.get("id", "")
        body_nodes = loop_node.get("body_nodes", [])
        if loop_id in body_nodes:
            errors.append(
                f"loop node '{loop_id}': body_nodes MUST NOT reference the loop node itself"
            )
        # Check transitive: if any body_node is a loop that has this loop in its body_nodes
        # This is a simplified check - full transitive would need graph traversal
        for body_node_id in body_nodes:
            body_node = next((n for n in nodes if n.get("id") == body_node_id), None)
            if body_node and body_node.get("kind") == "loop":
                nested_body = body_node.get("body_nodes", [])
                if loop_id in nested_body:
                    errors.append(
                        f"loop node '{loop_id}': circular loop reference detected "
                        f"with '{body_node_id}'"
                    )

    # --- Tools Policy Checks (17, 29) ---

    # Check 17: policy.cache.key_fields[] entries MUST use dot-path notation
    # (no $ prefix, no bracket notation)
    _DOT_PATH_RE = re.compile(r'^[a-zA-Z_][a-zA-Z0-9_]*(\.[a-zA-Z_][a-zA-Z0-9_]*)*$')
    for tool_list_key in ("mcp_servers", "http_apis", "sql_functions", "sandbox"):
        for tool in tools.get(tool_list_key, []):
            policy = tool.get("policy", {})
            if policy:
                cache = policy.get("cache", {})
                if cache:
                    key_fields = cache.get("key_fields", [])
                    for field in key_fields:
                        if not _DOT_PATH_RE.match(field):
                            errors.append(
                                f"tool '{tool.get('id', '?')}': "
                                f"cache.key_fields entry '{field}' must use dot-path notation"
                            )

    # Check 29: Any tool with load_strategy: "on_demand" MUST have a non-empty description
    for tool_list_key in ("mcp_servers", "http_apis", "sql_functions"):
        for tool in tools.get(tool_list_key, []):
            if tool.get("load_strategy") == "on_demand":
                desc = tool.get("description", "")
                if not desc.strip():
                    errors.append(
                        f"tool '{tool.get('id', '?')}': "
                        f"load_strategy 'on_demand' requires a non-empty description"
                    )

    # --- Memory Checks (18-21c, 24) ---

    memory = data.get("memory", {})

    # Check 18: memory.stores[] IDs must be unique (post-composition)
    if isinstance(memory, dict):
        stores = memory.get("stores", [])
        store_ids = [s.get("id") for s in stores if isinstance(s, dict)]
        seen = set()
        for sid in store_ids:
            if sid in seen:
                errors.append(f"memory: duplicate store id '{sid}'")
            seen.add(sid)

    # Check 19: memory.operations[].store_ref must reference a known stores[].id
    operations = memory.get("operations", [])
    store_ids_set = set(store_ids)
    for op in operations:
        if isinstance(op, dict):
            store_ref = op.get("store_ref")
            if store_ref and store_ref not in store_ids_set:
                errors.append(
                    f"memory.operations: store_ref '{store_ref}' not found in memory.stores"
                )

    # Check 20: memory.context_assembly.order[].store_ref must reference a known stores[].id
    context_assembly = memory.get("context_assembly", {})
    if isinstance(context_assembly, dict):
        order = context_assembly.get("order", [])
        for item in order:
            if isinstance(item, dict):
                store_ref = item.get("store_ref")
                if store_ref and store_ref not in store_ids_set:
                    errors.append(
                        f"memory.context_assembly.order: store_ref '{store_ref}' "
                        f"not found in memory.stores"
                    )

    # Check 21: memory.working.summary_model_ref (when present) must reference
    # a known runtime.models[].id
    working = memory.get("working", {})
    if isinstance(working, dict):
        summary_model_ref = working.get("summary_model_ref")
        if summary_model_ref and summary_model_ref not in model_ids:
            errors.append(
                f"memory.working.summary_model_ref '{summary_model_ref}' "
                f"not found in runtime.models"
            )

    # Check 21b: memory.working.summary_model_ref MUST be present when
    # memory.working.strategy = "summary"
    strategy = working.get("strategy")
    if strategy == "summary" and not working.get("summary_model_ref"):
        errors.append(
            "memory.working: summary_model_ref MUST be present when strategy is 'summary'"
        )

    # Check 21c: memory.working.compaction_threshold_tokens (when present) MUST be
    # <= memory.working.max_tokens
    compaction = working.get("compaction_threshold_tokens")
    max_tokens = working.get("max_tokens")
    if compaction is not None and max_tokens is not None:
        if compaction > max_tokens:
            errors.append(
                f"memory.working: compaction_threshold_tokens ({compaction}) "
                f"MUST be <= max_tokens ({max_tokens})"
            )

    # Check 24: memory.context_assembly.static_injection[].path (when source: "file")
    # must be a relative path without .. traversal; must also reference a declared workspace
    static_injections = context_assembly.get("static_injection", [])
    has_workspace = bool(data.get("workspace"))
    for si in static_injections:
        if isinstance(si, dict) and si.get("source") == "file":
            path = si.get("path", "")
            if ".." in path or path.startswith("/"):
                errors.append(
                    f"memory.context_assembly.static_injection: path '{path}' "
                    f"must be a relative path without .. traversal"
                )
            if not has_workspace:
                errors.append(
                    f"memory.context_assembly.static_injection: path '{path}' "
                    f"requires a workspace section to be declared"
                )

    # --- Guardrails Checks (22-23, 30) ---

    guardrails_v3 = data.get("guardrails", {})

    # Check 22: guardrails.interrupts[].tool_refs[] must reference known tool IDs
    interrupts = guardrails_v3.get("interrupts", [])
    all_tool_ids_all_types: set = set()
    for tool_list_key in ("mcp_servers", "http_apis", "sql_functions", "sandbox"):
        for tool in tools.get(tool_list_key, []):
            tid = tool.get("id")
            if tid:
                all_tool_ids_all_types.add(tid)
    for interrupt in interrupts:
        if isinstance(interrupt, dict):
            tool_refs = interrupt.get("tool_refs", [])
            for tool_ref in tool_refs:
                if tool_ref not in all_tool_ids_all_types:
                    errors.append(
                        f"guardrails.interrupts: tool_ref '{tool_ref}' "
                        f"not found in tools"
                    )

    # Check 22b: guardrails.interrupts[].execution_mode MUST NOT be set when mode: "pause_and_notify"
    for interrupt in interrupts:
        if isinstance(interrupt, dict):
            mode = interrupt.get("mode")
            execution_mode = interrupt.get("execution_mode")
            if mode == "pause_and_notify" and execution_mode is not None:
                errors.append(
                    f"guardrails.interrupts '{interrupt.get('id', '?')}': "
                    f"execution_mode MUST NOT be set when mode is 'pause_and_notify'"
                )

    # Check 23: guardrails.cost.interrupt_ref (when present) must reference a known
    # guardrails.interrupts[].id
    cost = guardrails_v3.get("cost", {})
    if isinstance(cost, dict):
        interrupt_ref = cost.get("interrupt_ref")
        interrupt_ids = {i.get("id") for i in interrupts if isinstance(i, dict)}
        if interrupt_ref and interrupt_ref not in interrupt_ids:
            errors.append(
                f"guardrails.cost.interrupt_ref '{interrupt_ref}' "
                f"not found in guardrails.interrupts"
            )

    # Check 30: guardrails.cost.downgrade_model_ref MUST be present when
    # on_threshold_exceeded: "downgrade"; it MUST reference a known runtime.models[].id
    on_threshold = cost.get("on_threshold_exceeded")
    downgrade_ref = cost.get("downgrade_model_ref")
    if on_threshold == "downgrade":
        if not downgrade_ref:
            errors.append(
                "guardrails.cost: downgrade_model_ref MUST be present "
                "when on_threshold_exceeded is 'downgrade'"
            )
        elif downgrade_ref not in model_ids:
            errors.append(
                f"guardrails.cost.downgrade_model_ref '{downgrade_ref}' "
                f"not found in runtime.models"
            )

    # --- Workspace Checks (25-26, 31) ---

    workspace = data.get("workspace", {})

    # Check 25: workspace.permissions.write[] paths MUST NOT escape workspace.root (no .. traversal)
    permissions = workspace.get("permissions", {})
    if isinstance(permissions, dict):
        write_paths = permissions.get("write", [])
        for path in write_paths:
            if ".." in path:
                errors.append(
                    f"workspace.permissions.write: path '{path}' MUST NOT escape workspace.root"
                )

    # Check 25b: Exactly one of workspace.root or workspace.root_env_var MUST be present
    # (only if workspace section exists)
    if workspace:
        root = workspace.get("root")
        root_env_var = workspace.get("root_env_var")
        if root is not None and root_env_var is not None:
            errors.append(
                "workspace: exactly one of 'root' or 'root_env_var' MUST be present, not both"
            )
        if root is None and root_env_var is None:
            errors.append(
                "workspace: exactly one of 'root' or 'root_env_var' MUST be present"
            )

    # Check 26: workspace.git.auto_commit: true requires workspace.git.enabled: true
    git = workspace.get("git", {})
    if isinstance(git, dict):
        if git.get("auto_commit") and not git.get("enabled"):
            errors.append(
                "workspace.git: auto_commit requires enabled to be true"
            )

    # Check 31: workspace.mounts[].id values must be unique;
    # workspace.mounts[].target paths MUST NOT escape workspace.root
    mounts = workspace.get("mounts", [])
    mount_ids = []
    for mount in mounts:
        if isinstance(mount, dict):
            mount_ids.append(mount.get("id"))
            target = mount.get("target", "")
            if ".." in target:
                errors.append(
                    f"workspace.mounts: target path '{target}' MUST NOT escape workspace.root"
                )
    if len(mount_ids) != len(set(mount_ids)):
        seen = set()
        for mid in mount_ids:
            if mid in seen:
                errors.append(f"workspace.mounts: duplicate mount id '{mid}'")
            seen.add(mid)

    # --- Sandbox Checks (27-28, 32) ---

    sandbox_tools = tools.get("sandbox", [])

    # Check 27: tools.sandbox[].policy.timeout_ms MUST be present
    for sandbox in sandbox_tools:
        if isinstance(sandbox, dict):
            policy = sandbox.get("policy", {})
            if isinstance(policy, dict) and "timeout_ms" not in policy:
                errors.append(
                    f"tools.sandbox '{sandbox.get('id', '?')}': "
                    f"policy.timeout_ms MUST be present (no unbounded sandbox execution)"
                )

    # Check 28: tools.sandbox[].mounts[].source: "workspace" requires a workspace
    # section to be declared
    has_workspace_declared = bool(data.get("workspace"))
    for sandbox in sandbox_tools:
        if isinstance(sandbox, dict):
            sandbox_mounts = sandbox.get("mounts", [])
            for mount in sandbox_mounts:
                if isinstance(mount, dict) and mount.get("source") == "workspace":
                    if not has_workspace_declared:
                        errors.append(
                            f"tools.sandbox '{sandbox.get('id', '?')}': "
                            f"mounts[].source 'workspace' requires a workspace section"
                        )

    # Check 32: tools.sandbox[].snapshot.enabled: true with provider: "custom"
    # emits a WARNING
    for sandbox in sandbox_tools:
        if isinstance(sandbox, dict):
            snapshot = sandbox.get("snapshot", {})
            provider = sandbox.get("provider")
            if isinstance(snapshot, dict) and snapshot.get("enabled"):
                if provider == "custom":
                    errors.append(
                        f"WARNING: tools.sandbox '{sandbox.get('id', '?')}': "
                        f"snapshot.enabled with provider 'custom' may not be supported"
                    )

    # --- Artifacts Checks (33-34) ---

    artifacts = data.get("artifacts", {})

    # Check 33: artifacts.stores[].id must be unique
    if isinstance(artifacts, dict):
        artifact_stores = artifacts.get("stores", [])
        artifact_store_ids = []
        for store in artifact_stores:
            if isinstance(store, dict):
                artifact_store_ids.append(store.get("id"))
        if len(artifact_store_ids) != len(set(artifact_store_ids)):
            seen = set()
            for sid in artifact_store_ids:
                if sid in seen:
                    errors.append(f"artifacts.stores: duplicate store id '{sid}'")
                seen.add(sid)

    # Check 34: nodes[].params.artifact.store_ref must reference a known
    # artifacts.stores[].id
    artifact_store_ids_set = set(artifact_store_ids)
    for node in nodes:
        if isinstance(node, dict):
            params = node.get("params", {})
            if isinstance(params, dict):
                artifact = params.get("artifact", {})
                if isinstance(artifact, dict):
                    store_ref = artifact.get("store_ref")
                    if store_ref and store_ref not in artifact_store_ids_set:
                        errors.append(
                            f"node '{node.get('id')}' params.artifact.store_ref "
                            f"'{store_ref}' not found in artifacts.stores"
                        )

    # --- Observability Checks (35-35b) ---

    observability = data.get("observability", {})

    # Check 35: observability.tracing.trace_events[] entries must be from the valid enum
    _VALID_TRACE_EVENTS = {
        "model_request", "tool_call", "flow_node", "loop_iteration",
        "interrupt", "cost_check", "artifact_write"
    }
    if isinstance(observability, dict):
        tracing = observability.get("tracing", {})
        if isinstance(tracing, dict):
            trace_events = tracing.get("trace_events", [])
            for event in trace_events:
                if event not in _VALID_TRACE_EVENTS:
                    errors.append(
                        f"observability.tracing.trace_events: '{event}' is not a valid trace event"
                    )

    # Check 35b: observability.cost_reporting.model_refs[] (when present) must
    # reference known runtime.models[].id values
    cost_reporting = observability.get("cost_reporting", {})
    if isinstance(cost_reporting, dict):
        model_refs = cost_reporting.get("model_refs", [])
        for model_ref in model_refs:
            if model_ref not in model_ids:
                errors.append(
                    f"observability.cost_reporting.model_refs: '{model_ref}' "
                    f"not found in runtime.models"
                )

    # --- AgentSpec Interop Checks (AS-1, AS-2) ---

    interop = data.get("interop", {})
    agentspec = interop.get("agentspec", {}) if isinstance(interop, dict) else {}

    # Check AS-1: interop.agentspec.node_map keys must match node IDs in flow.graph.nodes
    if isinstance(agentspec, dict):
        node_map = agentspec.get("node_map", {})
        if isinstance(node_map, dict):
            for mapped_node_id in node_map:
                if mapped_node_id not in node_ids:
                    errors.append(
                        f"interop.agentspec.node_map: key '{mapped_node_id}' "
                        f"does not match any node id in flow.graph.nodes"
                    )

    # Check AS-2: interop.agentspec.llm_map[].backend_id must match runtime.execution[].id
    if isinstance(agentspec, dict):
        llm_map = agentspec.get("llm_map", [])
        if isinstance(llm_map, list):
            runtime_backend_ids = {
                entry.get("id")
                for entry in data.get("runtime", {}).get("execution", [])
                if isinstance(entry, dict)
            }
            for binding in llm_map:
                if isinstance(binding, dict):
                    backend_id = binding.get("backend_id")
                    if backend_id and backend_id not in runtime_backend_ids:
                        errors.append(
                            f"interop.agentspec.llm_map: backend_id '{backend_id}' "
                            f"does not match any id in runtime.execution"
                        )

    # Check AS-3: interop.agentspec.ref MUST NOT contain path traversal sequences
    if isinstance(agentspec, dict):
        ref = agentspec.get("ref", "")
        if ref and ".." in ref:
            errors.append(
                f"interop.agentspec.ref '{ref}' MUST NOT contain path traversal sequences (..)"
            )

    return errors
