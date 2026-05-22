import pytest
from pathlib import Path

REPO_ROOT = Path(__file__).parents[4]


@pytest.fixture(scope="session")
def billing_manifest() -> dict:
    from adp_sdk.composition import resolve_adp
    adp = resolve_adp(REPO_ROOT / "examples" / "composition" / "billing-variant.yaml")
    return adp.model_dump(by_alias=True, exclude_none=True)


@pytest.fixture
def simple_manifest() -> dict:
    return {
        "adp_version": "0.2.0",
        "id": "test.agent",
        "runtime": {
            "execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}],
            "models": [{"id": "gpt4", "provider": "openai", "model": "gpt-4o-mini"}],
        },
        "flow": {
            "id": "test.flow",
            "graph": {
                "nodes": [
                    {"id": "input", "kind": "input"},
                    {"id": "chat", "kind": "llm", "model_ref": "gpt4"},
                    {"id": "output", "kind": "output"},
                ],
                "edges": [{"from": "input", "to": "chat"}, {"from": "chat", "to": "output"}],
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


@pytest.fixture
def tool_manifest() -> dict:
    return {
        "adp_version": "0.2.0",
        "id": "test.tool",
        "runtime": {
            "execution": [{"id": "py", "backend": "python", "entrypoint": "app:main"}],
            "models": [{"id": "gpt4", "provider": "openai", "model": "gpt-4o-mini"}],
        },
        "flow": {
            "id": "tool.flow",
            "graph": {
                "nodes": [
                    {"id": "input", "kind": "input"},
                    {"id": "lookup", "kind": "tool", "tool_ref": "billing-api"},
                    {"id": "output", "kind": "output"},
                ],
                "edges": [{"from": "input", "to": "lookup"}, {"from": "lookup", "to": "output"}],
                "start_nodes": ["input"],
                "end_nodes": ["output"],
            },
        },
        "tools": {
            "http_apis": [{"id": "billing-api", "description": "Billing", "base_url": "https://billing.example"}]
        },
        "evaluation": {"suites": [{"id": "basic", "metrics": [{"id": "m", "type": "deterministic", "threshold": True}]}]},
    }


@pytest.fixture
def mock_backend_factory():
    def factory(node: dict, _runtime_entry: dict):
        node_id = node["id"]
        kind = node.get("kind", "")
        def mock_callable(state: dict) -> dict:
            new_state = {**state}
            if kind == "llm":
                new_state["context"] = {
                    **state.get("context", {}),
                    node_id: {"content": f"mocked:{node_id}"},
                }
            elif kind == "tool":
                responses = list(state.get("tool_responses", {}).get(node_id, []))
                responses.append({"result": f"tool_result:{node_id}"})
                new_state["tool_responses"] = {
                    **state.get("tool_responses", {}),
                    node_id: responses,
                }
            return new_state
        return mock_callable
    return factory
