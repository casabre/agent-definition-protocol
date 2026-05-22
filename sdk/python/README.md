# adp-sdk — Python

Python SDK for the [Agent Definition Protocol](https://github.com/casabre/agent-definition-protocol).

## Install

```bash
pip install adp-sdk
```

## Core API

```python
from adp_sdk import validate_adp, validate_adp_semantics, resolve_adp

# Validate a manifest against the JSON Schema
result = validate_adp("path/to/agent.yaml")

# Run cross-schema semantic checks (edge→node, model_ref, suite_ref, tool_ref, ...)
issues = validate_adp_semantics("path/to/agent.yaml")

# Resolve composition (extends / import / overrides) and return a validated ADP object
adp = resolve_adp("examples/composition/billing-variant.yaml")
manifest = adp.model_dump(by_alias=True, exclude_none=True)
```

## Framework integrations (optional)

Framework-specific converters live in `adp_sdk.integrations.*` as optional extras. Install only what you need:

```bash
pip install "adp-sdk[langgraph]"          # LangGraph
pip install "adp-sdk[autogen]"            # AutoGen (autogen_agentchat / pyautogen >= 0.4)
pip install "adp-sdk[crewai]"             # CrewAI >= 1.0
pip install "adp-sdk[semantic-kernel]"    # Semantic Kernel >= 1.3
pip install "adp-sdk[all-integrations]"   # All of the above
```

Import directly from the submodule:

```python
from adp_sdk.integrations.langgraph import build_langgraph_from_adp, adp_from_langgraph
from adp_sdk.integrations.autogen import build_autogen_from_adp
from adp_sdk.integrations.crewai import build_crewai_from_adp
from adp_sdk.integrations.semantic_kernel import build_sk_from_adp
```

Integrations are **not** exported from the top-level `adp_sdk` package — callers must import submodules explicitly. This prevents import errors when a framework is not installed.

### LangGraph

```python
from adp_sdk import resolve_adp
from adp_sdk.integrations.langgraph import build_langgraph_from_adp

adp = resolve_adp("agent.yaml")
graph, node_map = build_langgraph_from_adp(
    adp.model_dump(by_alias=True, exclude_none=True),
    backend_factory=None,  # provide a callable to inject real LLM backends
)
```

Supports round-trip: `adp_from_langgraph(graph, node_map, original_manifest)` reconstructs the ADP manifest from a compiled graph.

### AutoGen

Requires `pyautogen >= 0.4` (`autogen_agentchat` module). Non-router nodes → `AssistantAgent`; router nodes → additionally create a `SelectorGroupChat`.

```python
from adp_sdk.integrations.autogen import build_autogen_from_adp

agent_map, chat_sequence = build_autogen_from_adp(manifest)
```

### CrewAI

Requires `crewai >= 1.0`. Uses the Flows API (`@start`, `@listen`, `@router`). Predecessor detection is edge-driven.

```python
from adp_sdk.integrations.crewai import build_crewai_from_adp

flow_instance, agents = build_crewai_from_adp(manifest)
```

### Semantic Kernel

Requires `semantic-kernel >= 1.3`. Works in mock mode without SK installed (useful for testing).

```python
from adp_sdk.integrations.semantic_kernel import build_sk_from_adp

kernel, process_steps = build_sk_from_adp(manifest)
```

## Running tests

```bash
# Core SDK tests
pytest sdk/python/tests/ -q

# Integration tests (skip if framework not installed)
pytest sdk/python/tests/integrations/ -v
```
