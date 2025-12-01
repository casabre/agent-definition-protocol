"""LangGraph bootstrapper that builds a StateGraph based on local ADP metadata."""
from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import yaml

try:
    from langgraph.graph import StateGraph
except ImportError:  # pragma: no cover - optional dependency
    StateGraph = None


def load_adp(path: Path | None = None) -> dict[str, Any]:
    """Load the ADP manifest adjacent to this module."""
    if path is None:
        path = Path(__file__).resolve().parents[3] / "adp" / "agent.yaml"
    return yaml.safe_load(path.read_text())


def build_graph() -> Any:
    """Build and compile a LangGraph StateGraph using ADP metadata for context."""
    if StateGraph is None:
        raise ImportError("langgraph is not installed")

    adp = load_adp()
    class State(dict):
        pass

    graph = StateGraph(State)

    def start(state: State) -> State:
        state["agent_id"] = adp.get("id")
        state["name"] = adp.get("name")
        system_prompt = (
            adp.get("prompts", {}).get("system")
            or "You are the Acme LangGraph sample agent."
        )
        state["system_prompt"] = system_prompt.strip()
        state["message"] = "Hello from LangGraph"
        return state

    graph.add_node("start", start)
    graph.set_entry_point("start")
    return graph.compile()


def main() -> Any:
    graph = build_graph()
    result = graph.invoke({})
    print(json.dumps(result, indent=2))
    return result


app = main

if __name__ == "__main__":  # pragma: no cover
    main()
