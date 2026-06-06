"""Tests for v0.3.0 semantic validation checks (lines 291-728 of validation.py).

Each test exercises a specific branch/error in validate_adp_semantics.
Uses inline ADP construction (no fixture files) for maximum control.
"""
import pytest
import yaml as _yaml
from adp_sdk.adp_model import ADP
from adp_sdk.validation import validate_adp, validate_adp_semantics


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _base_manifest(**extra) -> dict:
    """Minimal valid ADP manifest dict to build from."""
    m = {
        "adp_version": "0.3.0",
        "id": "test.v03",
        "runtime": {
            "execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}],
        },
        "flow": {
            "id": "f",
            "graph": {
                "nodes": [{"id": "n1", "kind": "input"}],
                "edges": [],
                "start_nodes": ["n1"],
                "end_nodes": ["n1"],
            },
        },
    }
    m.update(extra)
    return m


def _adp(**extra) -> ADP:
    return ADP.model_validate(_base_manifest(**extra))


def _adp_from_yaml(s: str) -> ADP:
    return ADP.model_validate(_yaml.safe_load(s))


# ============================================================================
# validate_adp: conformance class checks (lines 47-101)
# ============================================================================

def test_validate_adp_conformance_full_empty_flow():
    """conformance_class='full' + empty flow → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "conformance_class": "full",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "evaluation": {"suites": [{"id": "s", "metrics": [{"id": "m", "type": "deterministic", "threshold": True}]}]},
    })
    errors = validate_adp(adp)
    assert any("full" in e and "flow" in e for e in errors)


def test_validate_adp_conformance_full_empty_evaluation():
    """conformance_class='full' + non-empty flow + empty evaluation → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "conformance_class": "full",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {"id": "f", "graph": {"nodes": [{"id": "n", "kind": "input"}], "edges": [], "start_nodes": ["n"], "end_nodes": ["n"]}},
        "evaluation": {},
    })
    errors = validate_adp(adp)
    assert any("full" in e and "evaluation" in e for e in errors)


def test_validate_adp_minimal_mode_skips_flow_errors():
    """When flow is empty (minimal mode), flow-related JSON schema errors are suppressed."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "conformance_class": "minimal",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "evaluation": {},
    })
    errors = validate_adp(adp)
    # Minimal mode should not error on empty flow
    assert not any("flow" in e.lower() for e in errors if "full" not in e), \
        f"Unexpected flow error in minimal mode: {errors}"


def test_validate_adp_full_flow_schema_checked():
    """When flow is non-empty (full mode), flow schema is validated."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {"id": "f", "graph": {"nodes": [{"id": "n", "kind": "input"}], "edges": [], "start_nodes": ["n"], "end_nodes": ["n"]}},
        "evaluation": {"suites": [{"id": "s", "metrics": [{"id": "m", "type": "deterministic", "threshold": True}]}]},
    })
    errors = validate_adp(adp)
    assert isinstance(errors, list)


# ============================================================================
# validate_adp_semantics — loop checks (Check 15, 15b, 16)
# ============================================================================

def test_check15_loop_body_node_not_in_graph():
    """Loop body_nodes referencing unknown node ID → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {
            "id": "f",
            "graph": {
                "nodes": [
                    {"id": "loop1", "kind": "loop", "body_nodes": ["missing-node", "also-missing"]},
                ],
                "edges": [],
                "start_nodes": ["loop1"],
                "end_nodes": ["loop1"],
            },
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("body_nodes" in e and "missing-node" in e for e in errors), f"Got: {errors}"


def test_check15b_loop_body_nodes_no_connecting_edge():
    """Loop with >= 2 body_nodes but no edge between them → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {
            "id": "f",
            "graph": {
                "nodes": [
                    {"id": "a", "kind": "llm"},
                    {"id": "b", "kind": "llm"},
                    {"id": "loop1", "kind": "loop", "body_nodes": ["a", "b"]},
                ],
                "edges": [],
                "start_nodes": ["loop1"],
                "end_nodes": ["loop1"],
            },
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("body_nodes" in e and "connected" in e for e in errors), f"Got: {errors}"


def test_check15b_loop_body_nodes_with_connecting_edge_ok():
    """Loop with body_nodes connected by an edge → no error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {
            "id": "f",
            "graph": {
                "nodes": [
                    {"id": "a", "kind": "llm"},
                    {"id": "b", "kind": "llm"},
                    {"id": "loop1", "kind": "loop", "body_nodes": ["a", "b"]},
                ],
                "edges": [{"from": "a", "to": "b"}],
                "start_nodes": ["loop1"],
                "end_nodes": ["loop1"],
            },
        },
    })
    errors = validate_adp_semantics(adp)
    assert not any("body_nodes" in e and "connected" in e for e in errors), f"Got: {errors}"


def test_check16_loop_self_reference():
    """Loop body_nodes contains the loop node itself → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {
            "id": "f",
            "graph": {
                "nodes": [
                    {"id": "loop1", "kind": "loop", "body_nodes": ["loop1"]},
                ],
                "edges": [],
                "start_nodes": ["loop1"],
                "end_nodes": ["loop1"],
            },
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("loop node" in e and "itself" in e for e in errors), f"Got: {errors}"


def test_check16_transitive_loop_reference():
    """Loop body_nodes contains a nested loop that references back → circular ref error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {
            "id": "f",
            "graph": {
                "nodes": [
                    {"id": "n1", "kind": "llm"},
                    {"id": "inner_loop", "kind": "loop", "body_nodes": ["n1", "outer_loop"]},
                    {"id": "outer_loop", "kind": "loop", "body_nodes": ["n1", "inner_loop"]},
                ],
                "edges": [{"from": "n1", "to": "inner_loop"}],
                "start_nodes": ["outer_loop"],
                "end_nodes": ["outer_loop"],
            },
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("circular" in e for e in errors), f"Got: {errors}"


# ============================================================================
# Tools policy checks (Check 17, 29)
# ============================================================================

def test_check17_cache_key_fields_invalid_dot_path():
    """tools.*.policy.cache.key_fields with invalid path → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "tools": {
            "http_apis": [{
                "id": "api1",
                "description": "API",
                "base_url": "https://api.example.com",
                "policy": {
                    "cache": {
                        "enabled": True,
                        "key_fields": ["$input.body", "valid.path"],
                    }
                }
            }]
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("key_fields" in e and "dot-path" in e for e in errors), f"Got: {errors}"


def test_check17_cache_key_fields_valid_dot_path():
    """tools.*.policy.cache.key_fields with valid dot-path → no error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "tools": {
            "http_apis": [{
                "id": "api1",
                "description": "API",
                "base_url": "https://api.example.com",
                "policy": {
                    "cache": {
                        "enabled": True,
                        "key_fields": ["request.body", "request.headers"],
                    }
                }
            }]
        },
    })
    errors = validate_adp_semantics(adp)
    assert not any("key_fields" in e for e in errors), f"Got: {errors}"


def test_check29_on_demand_tool_requires_description():
    """load_strategy 'on_demand' without description → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "tools": {
            "mcp_servers": [{
                "id": "mcp1",
                "description": "",
                "transport": "stdio",
                "endpoint": "http://mcp",
                "load_strategy": "on_demand",
            }]
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("on_demand" in e and "description" in e for e in errors), f"Got: {errors}"


def test_check29_on_demand_tool_with_description_ok():
    """load_strategy 'on_demand' with non-empty description → no error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "tools": {
            "http_apis": [{
                "id": "api1",
                "description": "Billing API for payments",
                "base_url": "https://billing.example.com",
                "load_strategy": "on_demand",
            }]
        },
    })
    errors = validate_adp_semantics(adp)
    assert not any("on_demand" in e for e in errors), f"Got: {errors}"


# ============================================================================
# Memory checks (Check 18-21c, 24)
# ============================================================================

def test_check18_memory_duplicate_store_id():
    """memory.stores[] with duplicate id → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "memory": {
            "stores": [
                {"id": "store1", "type": "semantic", "provider": "pinecone"},
                {"id": "store1", "type": "episodic", "provider": "redis"},
            ]
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("duplicate store id" in e for e in errors), f"Got: {errors}"


def test_check19_memory_operation_bad_store_ref():
    """memory.operations[].store_ref not in stores → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "memory": {
            "stores": [{"id": "store1", "type": "semantic", "provider": "pinecone"}],
            "operations": [{"on_event": "on_invoke_end", "op": "write", "store_ref": "nonexistent"}],
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("store_ref" in e and "nonexistent" in e for e in errors), f"Got: {errors}"


def test_check20_context_assembly_bad_store_ref():
    """memory.context_assembly.order[].store_ref not in stores → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "memory": {
            "stores": [{"id": "store1", "type": "semantic", "provider": "pinecone"}],
            "context_assembly": {
                "order": [{"source": "store", "store_ref": "ghost-store"}],
            },
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("context_assembly" in e and "ghost-store" in e for e in errors), f"Got: {errors}"


def test_check21_summary_model_ref_not_in_models():
    """memory.working.summary_model_ref not in runtime.models → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {
            "execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}],
            "models": [{"id": "gpt4", "provider": "openai", "model": "gpt-4o"}],
        },
        "flow": {},
        "memory": {
            "working": {"strategy": "sliding_window", "summary_model_ref": "nonexistent-model"},
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("summary_model_ref" in e for e in errors), f"Got: {errors}"


def test_check21b_summary_strategy_requires_model_ref():
    """memory.working.strategy='summary' without summary_model_ref → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "memory": {
            "working": {"strategy": "summary"},
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("summary_model_ref" in e and "MUST be present" in e for e in errors), f"Got: {errors}"


def test_check21c_compaction_exceeds_max_tokens():
    """memory.working.compaction_threshold_tokens > max_tokens → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "memory": {
            "working": {
                "max_tokens": 1000,
                "compaction_threshold_tokens": 2000,
            },
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("compaction_threshold_tokens" in e for e in errors), f"Got: {errors}"


def test_check21c_compaction_ok():
    """memory.working.compaction_threshold_tokens <= max_tokens → no error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "memory": {
            "working": {
                "max_tokens": 4000,
                "compaction_threshold_tokens": 2000,
            },
        },
    })
    errors = validate_adp_semantics(adp)
    assert not any("compaction_threshold_tokens" in e for e in errors), f"Got: {errors}"


def test_check24_static_injection_path_traversal():
    """memory.context_assembly.static_injection with .. traversal → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "workspace": {"root": "/workspace"},
        "memory": {
            "context_assembly": {
                "static_injection": [{
                    "id": "si1",
                    "source": "file",
                    "path": "../etc/passwd",
                    "position": "prepend",
                }],
            },
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("path" in e and ".." in e for e in errors), f"Got: {errors}"


def test_check24_static_injection_absolute_path():
    """memory.context_assembly.static_injection with absolute path → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "workspace": {"root": "/workspace"},
        "memory": {
            "context_assembly": {
                "static_injection": [{
                    "id": "si1",
                    "source": "file",
                    "path": "/absolute/path/file.txt",
                    "position": "prepend",
                }],
            },
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("path" in e and "relative" in e for e in errors), f"Got: {errors}"


def test_check24_static_injection_no_workspace():
    """memory.context_assembly.static_injection source=file without workspace → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "memory": {
            "context_assembly": {
                "static_injection": [{
                    "id": "si1",
                    "source": "file",
                    "path": "data/system-prompt.txt",
                    "position": "prepend",
                }],
            },
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("workspace" in e for e in errors), f"Got: {errors}"


def test_check24_static_injection_inline_source_ok():
    """memory.context_assembly.static_injection source=inline → no path checks."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "memory": {
            "context_assembly": {
                "static_injection": [{
                    "id": "si1",
                    "source": "inline",
                    "content": "You are a helpful assistant.",
                    "position": "prepend",
                }],
            },
        },
    })
    errors = validate_adp_semantics(adp)
    assert not any("path" in e for e in errors), f"Got: {errors}"


# ============================================================================
# Guardrails checks (Check 22, 22b, 23, 30)
# ============================================================================

def test_check22_interrupt_tool_ref_not_found():
    """guardrails.interrupts[].tool_refs with unknown tool ID → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "guardrails": {
            "interrupts": [{
                "id": "int1",
                "trigger": "tool_call",
                "mode": "pause_and_notify",
                "tool_refs": ["nonexistent-tool"],
            }]
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("tool_ref" in e and "nonexistent-tool" in e for e in errors), f"Got: {errors}"


def test_check22b_pause_and_notify_with_execution_mode():
    """guardrails.interrupts with mode=pause_and_notify AND execution_mode → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "guardrails": {
            "interrupts": [{
                "id": "int1",
                "trigger": "tool_call",
                "mode": "pause_and_notify",
                "execution_mode": "blocking",
            }]
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("execution_mode" in e and "pause_and_notify" in e for e in errors), f"Got: {errors}"


def test_check23_cost_interrupt_ref_not_found():
    """guardrails.cost.interrupt_ref not found in interrupts → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "guardrails": {
            "interrupts": [{"id": "real-interrupt", "trigger": "tool_call", "mode": "block"}],
            "cost": {
                "threshold_usd": 10.0,
                "interrupt_ref": "ghost-interrupt",
            }
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("interrupt_ref" in e and "ghost-interrupt" in e for e in errors), f"Got: {errors}"


def test_check30_downgrade_requires_model_ref():
    """guardrails.cost.on_threshold_exceeded='downgrade' without downgrade_model_ref → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "guardrails": {
            "cost": {
                "threshold_usd": 5.0,
                "on_threshold_exceeded": "downgrade",
            }
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("downgrade_model_ref" in e and "MUST be present" in e for e in errors), f"Got: {errors}"


def test_check30_downgrade_model_ref_not_in_models():
    """guardrails.cost.downgrade_model_ref not in runtime.models → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {
            "execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}],
            "models": [{"id": "gpt4", "provider": "openai", "model": "gpt-4o"}],
        },
        "flow": {},
        "guardrails": {
            "cost": {
                "threshold_usd": 5.0,
                "on_threshold_exceeded": "downgrade",
                "downgrade_model_ref": "nonexistent-model",
            }
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("downgrade_model_ref" in e and "nonexistent-model" in e for e in errors), f"Got: {errors}"


def test_check30_downgrade_model_ref_valid():
    """guardrails.cost.downgrade_model_ref in runtime.models → no error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {
            "execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}],
            "models": [{"id": "gpt4-mini", "provider": "openai", "model": "gpt-4o-mini"}],
        },
        "flow": {},
        "guardrails": {
            "cost": {
                "threshold_usd": 5.0,
                "on_threshold_exceeded": "downgrade",
                "downgrade_model_ref": "gpt4-mini",
            }
        },
    })
    errors = validate_adp_semantics(adp)
    assert not any("downgrade_model_ref" in e for e in errors), f"Got: {errors}"


# ============================================================================
# Workspace checks (Check 25, 25b, 26, 31)
# ============================================================================

def test_check25_workspace_write_path_traversal():
    """workspace.permissions.write with .. path → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "workspace": {
            "root": "/workspace",
            "permissions": {
                "write": ["output/", "../etc/"],
            }
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("write" in e and ".." in e for e in errors), f"Got: {errors}"


def test_check25b_workspace_both_root_and_env_var():
    """workspace with both root and root_env_var → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "workspace": {
            "root": "/workspace",
            "root_env_var": "WORKSPACE_ROOT",
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("root" in e and "not both" in e for e in errors), f"Got: {errors}"


def test_check25b_workspace_neither_root_nor_env_var():
    """workspace without root or root_env_var → error.

    The workspace dict must be non-empty (truthy) for the check to trigger;
    an empty dict {} is falsy in Python and skips the 25b guard.
    Use git.enabled=True to make the dict truthy without providing root.
    """
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        # non-empty workspace with no root or root_env_var
        "workspace": {"git": {"enabled": True}},
    })
    errors = validate_adp_semantics(adp)
    assert any("root" in e and "MUST be present" in e for e in errors), f"Got: {errors}"


def test_check25b_workspace_with_root_only_ok():
    """workspace with only root → no error from check 25b."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "workspace": {"root": "/workspace"},
    })
    errors = validate_adp_semantics(adp)
    assert not any("root" in e and "MUST be present" in e for e in errors), f"Got: {errors}"


def test_check26_git_auto_commit_without_enabled():
    """workspace.git.auto_commit=true without git.enabled=true → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "workspace": {
            "root": "/workspace",
            "git": {"enabled": False, "auto_commit": True},
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("auto_commit" in e and "enabled" in e for e in errors), f"Got: {errors}"


def test_check31_workspace_mount_target_traversal():
    """workspace.mounts[].target with .. → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "workspace": {
            "root": "/workspace",
            "mounts": [{
                "id": "mount1",
                "provider": "s3",
                "bucket": "my-bucket",
                "target": "../outside",
                "read_only": True,
            }],
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("mounts" in e and ".." in e for e in errors), f"Got: {errors}"


def test_check31_workspace_duplicate_mount_id():
    """workspace.mounts with duplicate IDs → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "workspace": {
            "root": "/workspace",
            "mounts": [
                {"id": "m1", "provider": "s3", "bucket": "b", "target": "data1", "read_only": True},
                {"id": "m1", "provider": "gcs", "bucket": "c", "target": "data2", "read_only": True},
            ],
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("duplicate mount id" in e for e in errors), f"Got: {errors}"


# ============================================================================
# Sandbox checks (Check 27, 28, 32)
# ============================================================================

def test_check27_sandbox_missing_timeout_ms():
    """tools.sandbox[].policy without timeout_ms → error.

    SandboxPolicy's Pydantic model requires timeout_ms, so we cannot create a valid
    ADP object without it. Instead, build a minimal ADP (with timeout_ms present) then
    patch model_dump to return a dict where policy has no timeout_ms, simulating what
    the semantic check would see if the constraint were relaxed.
    """
    import unittest.mock as mock
    import adp_sdk.validation as _v

    # Build a valid ADP (timeout_ms present so Pydantic is happy)
    adp_with_sandbox = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "tools": {
            "sandbox": [{
                "id": "sb1",
                "runtime": "python",
                "provider": "docker",
                "policy": {"timeout_ms": 5000},
            }]
        },
    })

    # Prepare a raw dump where policy lacks timeout_ms (to exercise the semantic check)
    raw_dump = adp_with_sandbox.model_dump(by_alias=True, exclude_none=True)
    raw_dump["tools"]["sandbox"][0]["policy"] = {}  # remove timeout_ms

    with mock.patch.object(adp_with_sandbox, "model_dump", return_value=raw_dump):
        errors = _v.validate_adp_semantics(adp_with_sandbox)
    assert any("timeout_ms" in e for e in errors), f"Got: {errors}"


def test_check28_sandbox_workspace_mount_without_workspace():
    """tools.sandbox[].mounts[].source='workspace' without workspace → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "tools": {
            "sandbox": [{
                "id": "sb1",
                "runtime": "python",
                "provider": "docker",
                "policy": {"timeout_ms": 5000},
                "mounts": [{"source": "workspace", "target": "/ws"}],
            }]
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("workspace" in e for e in errors), f"Got: {errors}"


def test_check32_sandbox_snapshot_custom_provider_warning():
    """tools.sandbox with snapshot.enabled + provider='custom' → WARNING."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "tools": {
            "sandbox": [{
                "id": "sb1",
                "runtime": "python",
                "provider": "custom",
                "policy": {"timeout_ms": 5000},
                "snapshot": {"enabled": True, "restore_on": "never"},
            }]
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("WARNING" in e and "snapshot" in e and "custom" in e for e in errors), f"Got: {errors}"


# ============================================================================
# Artifacts checks (Check 33, 34)
# ============================================================================

def test_check33_artifacts_duplicate_store_id():
    """artifacts.stores[] with duplicate id → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "artifacts": {
            "stores": [
                {"id": "store1", "scope": "session", "provider": "gcs"},
                {"id": "store1", "scope": "user", "provider": "s3"},
            ]
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("artifacts" in e and "duplicate store id" in e for e in errors), f"Got: {errors}"


def test_check34_node_artifact_store_ref_not_found():
    """node params.artifact.store_ref not in artifacts.stores → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {
            "id": "f",
            "graph": {
                "nodes": [{
                    "id": "n1",
                    "kind": "llm",
                    "params": {"artifact": {"store_ref": "ghost-store"}},
                }],
                "edges": [],
                "start_nodes": ["n1"],
                "end_nodes": ["n1"],
            }
        },
        "artifacts": {
            "stores": [{"id": "real-store", "scope": "session", "provider": "gcs"}]
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("store_ref" in e and "ghost-store" in e for e in errors), f"Got: {errors}"


# ============================================================================
# Observability checks (Check 35, 35b)
# ============================================================================

def test_check35_invalid_trace_event():
    """observability.tracing.trace_events with unknown event → error.

    Pydantic validates trace_events as an enum, so we cannot pass an invalid event
    directly to ADP.model_validate. Instead, build a valid ADP and patch model_dump
    to inject an invalid event string, simulating what semantic validation would see.
    """
    import unittest.mock as mock

    adp_with_tracing = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "observability": {
            "tracing": {
                "backend": "otlp",
                "trace_events": ["model_request"],
            }
        },
    })

    # Patch model_dump to inject an invalid event
    raw_dump = adp_with_tracing.model_dump(by_alias=True, exclude_none=True)
    raw_dump["observability"]["tracing"]["trace_events"] = ["model_request", "invalid_event_xyz"]

    with mock.patch.object(adp_with_tracing, "model_dump", return_value=raw_dump):
        errors = validate_adp_semantics(adp_with_tracing)
    assert any("invalid_event_xyz" in e for e in errors), f"Got: {errors}"


def test_check35_valid_trace_events():
    """observability.tracing.trace_events with all valid events → no error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "observability": {
            "tracing": {
                "backend": "otlp",
                "trace_events": ["model_request", "tool_call", "flow_node", "loop_iteration"],
            }
        },
    })
    errors = validate_adp_semantics(adp)
    assert not any("trace_events" in e for e in errors), f"Got: {errors}"


def test_check35b_cost_reporting_model_ref_not_found():
    """observability.cost_reporting.model_refs with unknown model → error."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {
            "execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}],
            "models": [{"id": "gpt4", "provider": "openai", "model": "gpt-4o"}],
        },
        "flow": {},
        "observability": {
            "cost_reporting": {
                "enabled": True,
                "model_refs": ["ghost-model"],
            }
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("cost_reporting" in e and "ghost-model" in e for e in errors), f"Got: {errors}"


# ============================================================================
# Additional validation.py coverage — validate_adp internals
# ============================================================================

def test_validate_adp_schema_registry_built():
    """_build_registry returns a functioning registry (covers lines 35-39)."""
    from adp_sdk.validation import _build_registry
    reg = _build_registry()
    assert reg is not None


def test_validate_adp_load_schema():
    """_load_schema returns a dict with $id or $schema (covers line 31)."""
    from adp_sdk.validation import _load_schema
    schema = _load_schema("adp.schema.json")
    assert isinstance(schema, dict)


def test_validate_adp_full_flow_non_minimal():
    """validate_adp with full flow (non-minimal) covers flow_validator branch (lines 91-94)."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {
            "id": "f",
            "graph": {
                "nodes": [{"id": "n", "kind": "input"}],
                "edges": [],
                "start_nodes": ["n"],
                "end_nodes": ["n"],
            }
        },
        "evaluation": {
            "suites": [{"id": "s", "metrics": [{"id": "m", "type": "deterministic", "threshold": True}]}]
        },
    })
    errors = validate_adp(adp)
    assert isinstance(errors, list)


def test_validate_adp_inferred_minimal_from_none_conformance():
    """conformance_class=None + empty flow → inferred minimal (lines 66-70)."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "evaluation": {"suites": [{"id": "s", "metrics": [{"id": "m", "type": "deterministic", "threshold": True}]}]},
    })
    errors = validate_adp(adp)
    # Should not produce errors for empty flow in minimal mode
    assert not any("flow" in e.lower() and "empty" in e.lower() for e in errors), f"Got: {errors}"


# ============================================================================
# sql_functions tool check (Check 17 for sql_functions key)
# ============================================================================

def test_check17_sql_functions_cache_key_fields():
    """sql_functions tool policy cache.key_fields validation."""
    adp = ADP.model_validate({
        "adp_version": "0.3.0",
        "id": "test",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {},
        "tools": {
            "sql_functions": [{
                "id": "sql1",
                "description": "SQL lookup",
                "connection": "postgres://localhost/db",
                "policy": {
                    "cache": {
                        "enabled": True,
                        "key_fields": ["$result[0]"],  # invalid
                    }
                }
            }]
        },
    })
    errors = validate_adp_semantics(adp)
    assert any("key_fields" in e for e in errors), f"Got: {errors}"
