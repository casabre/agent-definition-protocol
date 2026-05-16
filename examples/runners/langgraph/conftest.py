import pytest
import yaml
from pathlib import Path

REPO_ROOT = Path(__file__).parents[3]


@pytest.fixture
def acme_manifest() -> dict:
    path = REPO_ROOT / "examples" / "acme-analytics" / "adp" / "agent.yaml"
    return yaml.safe_load(path.read_text())


@pytest.fixture
def mock_backend_factory():
    """Inject mock callables for all node kinds — no real LLM or API calls."""
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
            elif kind == "input":
                new_state["inputs"] = state.get("inputs", {})
            elif kind == "output":
                pass
            elif kind in ("retriever", "evaluator"):
                new_state["context"] = {
                    **state.get("context", {}),
                    node_id: {"content": f"mocked:{node_id}"},
                }
            return new_state
        return mock_callable
    return factory
