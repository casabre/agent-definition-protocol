# ADP Framework Runner Examples

Each subdirectory contains a self-contained pytest suite demonstrating how to consume a composition-resolved ADP manifest in a specific framework.

All runners use `examples/composition/billing-variant.yaml` as the primary test manifest. Resolve it first:

```python
from adp_sdk import resolve_adp

adp = resolve_adp("examples/composition/billing-variant.yaml")
manifest = adp.model_dump(by_alias=True, exclude_none=True)
```

## Runner comparison

| Runner | Module | Entry point | ADP concepts covered | Direction |
|--------|--------|------------|---------------------|-----------|
| [LangGraph](langgraph/) | `adp_sdk.integrations.langgraph` | `build_langgraph_from_adp` | Nodes→StateGraph, conditional edges, composition, tool_ref | Round-trip |
| [AutoGen](autogen/) | `adp_sdk.integrations.autogen` | `build_autogen_from_adp` | Nodes→AssistantAgent, router→RoundRobinGroupChat | Import only |
| [CrewAI](crewai/) | `adp_sdk.integrations.crewai` | `build_crewai_from_adp` | Nodes→Agent/Task, router→@router, start_nodes→@start | Import only |
| [Semantic Kernel](semantic-kernel/) | `adp_sdk.integrations.semantic_kernel` | `build_sk_from_adp` | Nodes→KernelFunction/Plugin, KernelProcess, model_ref resolution | Import only |

**Import only** means ADP → framework. Export (framework → ADP) requires framework-specific introspection APIs not yet standardized; planned for v0.3.0.

## Import path

The conversion logic lives in the `adp_sdk.integrations` subpackage as optional extras:

```python
# Preferred: import from the SDK submodule
from adp_sdk.integrations.langgraph import build_langgraph_from_adp
from adp_sdk.integrations.autogen import build_autogen_from_adp
from adp_sdk.integrations.crewai import build_crewai_from_adp
from adp_sdk.integrations.semantic_kernel import build_sk_from_adp
```

The `build_adp_graph.py` file in each runner directory is a thin backward-compatible shim (`from adp_sdk.integrations.<module> import *`). Existing import patterns continue to work.

## Quick start

```bash
# LangGraph (round-trip)
cd examples/runners/langgraph
pip install -e "../../../sdk/python[langgraph]"
pytest -v

# AutoGen (import only, requires pyautogen>=0.4 / autogen_agentchat)
cd examples/runners/autogen
pip install -e "../../../sdk/python[autogen]"
pytest -v

# CrewAI (import only, requires crewai>=1.0)
cd examples/runners/crewai
pip install -e "../../../sdk/python[crewai]"
pytest -v

# Semantic Kernel (import only, requires semantic-kernel>=1.3)
cd examples/runners/semantic-kernel
pip install -e "../../../sdk/python[semantic-kernel]"
pytest -v
```

## Composition pre-step

All runners require composition resolution before building the framework graph. The `billing_manifest` session-scoped pytest fixture in each `conftest.py` calls `resolve_adp()` once per test session and shares the result.

```python
@pytest.fixture(scope="session")
def billing_manifest() -> dict:
    from adp_sdk.composition import resolve_adp
    adp = resolve_adp(REPO_ROOT / "examples/composition/billing-variant.yaml")
    return adp.model_dump(by_alias=True, exclude_none=True)
```

## ADP concepts demonstrated

| ADP Field | LangGraph | AutoGen | CrewAI | SK |
|-----------|-----------|---------|--------|-----|
| `flow.graph.nodes[]` | `StateGraph.add_node` | `AssistantAgent` | `Agent` | `KernelFunction` / `KernelPlugin` |
| `flow.graph.edges[]` | `add_edge` / `add_conditional_edges` | `RoundRobinGroupChat` sequence | `Task` dependency | `ProcessStep` transition |
| `flow.graph.start_nodes[]` | `set_entry_point` | first group chat | `@start` | process entry |
| `node.model_ref` | `ChatOpenAI(model=...)` | `model_client` | task LLM | `OpenAIChatCompletion` |
| `node.tool_ref` | `resolve_callable()` lookup | `FunctionCallingAgent` | `@tool` binding | `KernelPlugin` |
| `router` node | `add_conditional_edges` | `SelectorGroupChat` | `@router` | `OnEvent` |
| `extends` / `import` | `resolve_adp()` pre-step | `resolve_adp()` pre-step | `resolve_adp()` pre-step | `resolve_adp()` pre-step |
