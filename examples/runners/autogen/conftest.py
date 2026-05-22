import pytest
from pathlib import Path
import sys

REPO_ROOT = Path(__file__).parents[3]


@pytest.fixture(scope="session")
def billing_manifest() -> dict:
    sys.path.insert(0, str(REPO_ROOT / "sdk" / "python"))
    from adp_sdk.composition import resolve_adp
    adp = resolve_adp(REPO_ROOT / "examples" / "composition" / "billing-variant.yaml")
    return adp.model_dump(by_alias=True, exclude_none=True)


@pytest.fixture
def simple_manifest() -> dict:
    """A minimal 3-node manifest for basic tests that don't need composition."""
    return {
        "adp_version": "0.2.0",
        "id": "test.agent",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {
            "id": "test.flow",
            "graph": {
                "nodes": [
                    {"id": "input", "kind": "input"},
                    {"id": "llm", "kind": "llm", "model_ref": "gpt4"},
                    {"id": "output", "kind": "output"},
                ],
                "edges": [{"from": "input", "to": "llm"}, {"from": "llm", "to": "output"}],
                "start_nodes": ["input"],
                "end_nodes": ["output"],
            },
        },
        "evaluation": {"suites": [{"id": "basic", "metrics": [{"id": "m", "type": "deterministic", "threshold": True}]}]},
    }


@pytest.fixture
def router_manifest() -> dict:
    return {
        "adp_version": "0.2.0",
        "id": "test.router",
        "runtime": {"execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}]},
        "flow": {
            "id": "router.flow",
            "graph": {
                "nodes": [
                    {"id": "input", "kind": "input"},
                    {"id": "decide", "kind": "router", "strategy": "conditional"},
                    {"id": "path_a", "kind": "llm"},
                    {"id": "output", "kind": "output"},
                ],
                "edges": [
                    {"from": "input", "to": "decide"},
                    {"from": "decide", "to": "path_a", "condition": "inputs.x == 1"},
                    {"from": "path_a", "to": "output"},
                ],
                "start_nodes": ["input"],
                "end_nodes": ["output"],
            },
        },
        "evaluation": {"suites": [{"id": "basic", "metrics": [{"id": "m", "type": "deterministic", "threshold": True}]}]},
    }
