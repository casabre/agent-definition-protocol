#!/usr/bin/env python3
"""
ESP Runner Conformance Harness

Validates the esp-conformance-fixtures.yaml format and ADR D1–D8 compliance
in --dry-run mode, or runs scenarios against a real runner via --adapter.

Usage:
  python3 scripts/esp-runner-harness.py --dry-run
  python3 scripts/esp-runner-harness.py --adapter my_module.MyRunnerAdapter
"""
from __future__ import annotations

import argparse
import importlib
import sys
from abc import ABC, abstractmethod
from pathlib import Path

import yaml

FIXTURES_PATH = Path(__file__).parent / "esp-conformance-fixtures.yaml"
REQUIRED_NODE_KINDS = {"input", "llm", "tool", "router", "retriever", "evaluator", "output", "subflow"}
REQUIRED_SCENARIO_KEYS = {
    "id", "node_kind", "description", "manifest_fragment",
    "invocation", "expected_state_transitions", "expected_output",
}


class RunnerAdapter(ABC):
    @abstractmethod
    def execute_node(self, node: dict, state: dict, manifest: dict) -> dict:
        """Execute a single node and return the updated state."""


def load_fixtures() -> list[dict]:
    data = yaml.safe_load(FIXTURES_PATH.read_text())
    return data.get("scenarios", [])


def _find_node_id_by_kind(scenario: dict, kind: str) -> str:
    nodes = (
        scenario.get("manifest_fragment", {})
        .get("flow", {})
        .get("graph", {})
        .get("nodes", [])
    )
    for node in nodes:
        if node.get("kind") == kind:
            return node.get("id", "")
    return ""


def validate_fixture_structure(scenarios: list[dict]) -> list[str]:
    errors = []
    for s in scenarios:
        sid = s.get("id", "<unknown>")
        missing = REQUIRED_SCENARIO_KEYS - set(s.keys())
        if missing:
            errors.append(f"Scenario '{sid}' missing required keys: {sorted(missing)}")
        if "invocation" in s and "inputs" not in s.get("invocation", {}):
            errors.append(f"Scenario '{sid}': invocation must have 'inputs' key")
        transitions = s.get("expected_state_transitions", [])
        if not isinstance(transitions, list):
            errors.append(f"Scenario '{sid}': expected_state_transitions must be a list")
            continue
        for i, t in enumerate(transitions):
            if "after_node" not in t:
                errors.append(f"Scenario '{sid}' transition[{i}] missing 'after_node'")
            if "state" not in t:
                errors.append(f"Scenario '{sid}' transition[{i}] missing 'state'")
    return errors


def check_all_node_kinds_present(scenarios: list[dict]) -> list[str]:
    found = {s.get("node_kind") for s in scenarios}
    missing = REQUIRED_NODE_KINDS - found
    if missing:
        return [f"Missing scenarios for node kinds: {sorted(missing)}"]
    return []


def check_adr_rules(scenarios: list[dict]) -> list[str]:
    """Hardcoded ADR D1–D7 compliance rules — not parsed from the ADR doc."""
    errors = []
    for s in scenarios:
        sid = s.get("id", "<unknown>")
        kind = s.get("node_kind")
        transitions = s.get("expected_state_transitions", [])

        if kind == "input":
            input_node_id = _find_node_id_by_kind(s, "input")
            for t in transitions:
                if t.get("after_node") == input_node_id:
                    inputs = t.get("state", {}).get("inputs", {})
                    if not isinstance(inputs, dict) or len(inputs) == 0:
                        errors.append(
                            f"[D1] input scenario '{sid}': state.inputs must be a "
                            f"non-empty dict after input node '{input_node_id}' fires"
                        )

        elif kind == "llm":
            llm_node_id = _find_node_id_by_kind(s, "llm")
            for t in transitions:
                if t.get("after_node") == llm_node_id:
                    context = t.get("state", {}).get("context", {})
                    if llm_node_id not in context:
                        errors.append(
                            f"[D2] llm scenario '{sid}': state.context['{llm_node_id}'] "
                            f"must exist after llm node fires"
                        )

        elif kind == "tool":
            tool_node_id = _find_node_id_by_kind(s, "tool")
            for t in transitions:
                if t.get("after_node") == tool_node_id:
                    tool_responses = t.get("state", {}).get("tool_responses", {})
                    responses = tool_responses.get(tool_node_id)
                    if responses is None or not isinstance(responses, list):
                        errors.append(
                            f"[D3] tool scenario '{sid}': state.tool_responses['{tool_node_id}'] "
                            f"must be a list after tool node fires"
                        )

        elif kind == "router":
            edges = (
                s.get("manifest_fragment", {})
                .get("flow", {})
                .get("graph", {})
                .get("edges", [])
            )
            if not any("condition" in e for e in edges):
                errors.append(
                    f"[D4] router scenario '{sid}': manifest_fragment.flow.graph.edges "
                    f"must include at least one conditional edge"
                )
            router_node_id = _find_node_id_by_kind(s, "router")
            for t in transitions:
                if t.get("after_node") == router_node_id:
                    context = t.get("state", {}).get("context", {})
                    if router_node_id in context:
                        errors.append(
                            f"[D4] router scenario '{sid}': state.context must NOT "
                            f"contain router node id '{router_node_id}' — router writes no state"
                        )

        elif kind == "retriever":
            retriever_node_id = _find_node_id_by_kind(s, "retriever")
            for t in transitions:
                if t.get("after_node") == retriever_node_id:
                    context = t.get("state", {}).get("context", {})
                    if retriever_node_id not in context:
                        errors.append(
                            f"[D5] retriever scenario '{sid}': state.context['{retriever_node_id}'] "
                            f"must exist after retriever fires"
                        )

        elif kind == "evaluator":
            eval_node_id = _find_node_id_by_kind(s, "evaluator")
            for t in transitions:
                if t.get("after_node") == eval_node_id:
                    context = t.get("state", {}).get("context", {})
                    node_ctx = context.get(eval_node_id, {})
                    if "passed" not in node_ctx:
                        errors.append(
                            f"[D6] evaluator scenario '{sid}': state.context['{eval_node_id}'] "
                            f"must have 'passed' key after evaluator fires"
                        )

        elif kind == "output":
            if s.get("expected_output") is None:
                errors.append(
                    f"[D7] output scenario '{sid}': expected_output must be non-null"
                )

        elif kind == "subflow":
            subflow_node_id = _find_node_id_by_kind(s, "subflow")
            for t in transitions:
                if t.get("after_node") == subflow_node_id:
                    context = t.get("state", {}).get("context", {})
                    if subflow_node_id not in context:
                        errors.append(
                            f"[D8] subflow scenario '{sid}': state.context['{subflow_node_id}'] "
                            f"must exist after subflow fires"
                        )
                    elif not isinstance(context.get(subflow_node_id), dict):
                        errors.append(
                            f"[D8] subflow scenario '{sid}': state.context['{subflow_node_id}'] "
                            f"must be an object"
                        )

    return errors


def run_dry(scenarios: list[dict]) -> int:
    all_errors: list[str] = []
    all_errors.extend(validate_fixture_structure(scenarios))
    all_errors.extend(check_all_node_kinds_present(scenarios))
    all_errors.extend(check_adr_rules(scenarios))
    if all_errors:
        for e in all_errors:
            print(f"ERROR: {e}", file=sys.stderr)
        return 1
    print(f"OK: --dry-run passed ({len(scenarios)} scenarios, all ADR D1–D8 rules satisfied)")
    return 0


def _states_match(actual: dict, expected: dict) -> bool:
    for key, value in expected.items():
        if key not in actual:
            return False
        if isinstance(value, dict):
            if not isinstance(actual[key], dict):
                return False
            if not _states_match(actual[key], value):
                return False
        elif actual[key] != value:
            return False
    return True


def run_with_adapter(scenarios: list[dict], adapter_path: str) -> int:
    module_path, cls_name = adapter_path.rsplit(".", 1)
    module = importlib.import_module(module_path)
    adapter: RunnerAdapter = getattr(module, cls_name)()
    failures = 0
    for s in scenarios:
        sid = s.get("id")
        manifest = s.get("manifest_fragment", {})
        state: dict = {
            "inputs": s.get("invocation", {}).get("inputs", {}),
            "context": {},
            "memory": {},
            "tool_responses": {},
        }
        nodes = manifest.get("flow", {}).get("graph", {}).get("nodes", [])
        node_map = {n["id"]: n for n in nodes}
        for transition in s.get("expected_state_transitions", []):
            node_id = transition.get("after_node")
            node = node_map.get(node_id, {"id": node_id})
            state = adapter.execute_node(node, state, manifest)
            expected = transition.get("state", {})
            if not _states_match(state, expected):
                print(f"FAIL [{sid}] after node '{node_id}': state mismatch")
                print(f"  expected: {expected}")
                print(f"  actual:   {state}")
                failures += 1
    if failures == 0:
        print(f"OK: all {len(scenarios)} scenarios passed with adapter '{adapter_path}'")
    return 1 if failures else 0


def main() -> int:
    parser = argparse.ArgumentParser(description="ESP Runner Conformance Harness")
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument(
        "--dry-run", action="store_true",
        help="Validate fixture format and ADR D1–D7 compliance without a real runner",
    )
    group.add_argument(
        "--adapter", metavar="MODULE.CLASS",
        help="Run scenarios against an adapter (e.g. my_runner.MyAdapter)",
    )
    args = parser.parse_args()
    scenarios = load_fixtures()
    if args.dry_run:
        return run_dry(scenarios)
    return run_with_adapter(scenarios, args.adapter)


if __name__ == "__main__":
    sys.exit(main())
