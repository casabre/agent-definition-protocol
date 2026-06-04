# ADP v0.3.0 Framework Adapters Specification

**Agent Definition Protocol — SDK Framework Adapter Modules v0.3.0**

> **Version**: v0.3.0  
> **Status**: Draft  
> **Editor**: ADP Working Group  
> **Last Updated**: 2026-05-25  

---

## Abstract

The existing `runtime.adapter_hints` block is passive YAML — it carries framework-specific key/value hints for runners and downstream tooling, but provides no executable conversion. This document adds **SDK-level adapter modules** that convert between an ADP manifest and framework-native configuration objects, enabling two workflows:

- **Export** (`manifest → framework config`): Teams author in ADP, then call `adapter.export(manifest)` to get a ready-to-use framework config
- **Import** (`framework config → manifest`): Teams with existing framework configs call `adapter.import_from(config)` to generate an ADP manifest for documentation, governance, or migration

---

## Table of Contents

1. [Adapter API Surface (Python)](#1-adapter-api-surface-python)
2. [Connection to `runtime.adapter_hints`](#2-connection-to-runtimeadapter_hints)
3. [Per-Framework Coverage](#3-per-framework-coverage)
4. [Translation Fidelity Contract](#4-translation-fidelity-contract)
5. [Optional Dependencies](#5-optional-dependencies)
6. [TypeScript Adapter Scope](#6-typescript-adapter-scope)
7. [Rust and Go: Adapter Stubs](#7-rust-and-go-adapter-stubs)
8. [Semantic Validation Checks](#8-semantic-validation-checks)

---

## 1. Adapter API Surface (Python)

```python
# sdk/python/adp_sdk/adapters/base.py
from abc import ABC, abstractmethod
from typing import Any
from adp_sdk.adp_model import Manifest

class AdapterBase(ABC):
    framework_id: str          # e.g. "langgraph", "autogen", "crewai"

    @abstractmethod
    def export(self, manifest: Manifest) -> dict[str, Any]:
        """Export ADP manifest → framework-native config dict.
        
        Reads manifest.runtime.adapter_hints[self.framework_id] for any
        framework-specific overrides (takes precedence over derived values).
        """

    @abstractmethod
    def import_from(self, config: dict[str, Any]) -> Manifest:
        """Import framework-native config → ADP manifest (best-effort).
        
        Fields with no ADP equivalent MUST be placed in manifest.extensions,
        never silently discarded (check 37 normative requirement).
        """

    def roundtrip_fidelity(self) -> dict[str, str]:
        """Returns coverage per ADP section: "faithful" | "lossy" | "unsupported"."""
```

```python
# sdk/python/adp_sdk/adapters/registry.py
class AdapterRegistry:
    _adapters: dict[str, type[AdapterBase]] = {}
    
    @classmethod
    def register(cls, adapter_class: type[AdapterBase]) -> None:
        cls._adapters[adapter_class.framework_id] = adapter_class
    
    @classmethod
    def get(cls, framework_id: str) -> AdapterBase:
        if framework_id not in cls._adapters:
            raise ValueError(f"Unknown framework: {framework_id}")
        return cls._adapters[framework_id]()
    
    @classmethod
    def available(cls) -> list[str]:
        return list(cls._adapters.keys())
```

---

## 2. Connection to `runtime.adapter_hints`

`runtime.adapter_hints` is the declarative bridge between the two layers:

```python
class LangGraphAdapter(AdapterBase):
    framework_id = "langgraph"

    def export(self, manifest: Manifest) -> dict:
        # 1. Derive base config from ADP primitives (faithful mapping)
        state_graph = self._build_state_graph(manifest.flow)
        
        # 2. Apply runtime.adapter_hints overrides (framework-specific)
        hints = (manifest.runtime.adapter_hints or {}).get("langgraph", {})
        if checkpointer := hints.get("checkpointer"):
            state_graph["checkpointer"] = checkpointer
        if memory_store := hints.get("memory_store"):
            state_graph["store"] = memory_store
        return state_graph
```

**Rule**: ADP primitives drive the base translation; `adapter_hints` provides opt-in overrides for framework-specific behavior that has no ADP-level encoding. This keeps ADP manifests portable while allowing fine-grained control.

---

## 3. Per-Framework Coverage

| Framework | Lang | Export coverage | Import coverage |
|---|---|---|---|
| **LangGraph** | Python, TS | graph → `StateGraph`; tools → `tool_node`; LLM → `ChatModel`; loop → `recursion_limit`; memory → checkpointer via adapter_hints | `StateGraph` topology → `flow.graph`; tools → `tools.http_apis`; checkpointer hint → `adapter_hints.langgraph` |
| **AutoGen** | Python | flow → `GroupChat`/`RoundRobin`; tools → `FunctionTool`; loop `max_iterations` → `MaxMessageTermination`; cost → `TokenUsageTermination` | `GroupChat` config → `flow.graph`; termination conditions → `loop.termination` |
| **CrewAI** | Python | agents → subflow nodes; tools → `tools.http_apis`; `crew.process` → routing strategy; rate_limit → `max_rpm` | Crew YAML → `flow.graph` + tools; agent config → node params |
| **LlamaIndex** | Python | tools → `QueryEngineTool`; memory → `ChatMemoryBuffer`/`VectorMemoryBuffer`; model_ref → LLM binding | pipeline steps → `flow.graph`; index config → `memory.stores` |
| **Google ADK** | Python | agents → flow nodes; tools → `FunctionTool`; artifacts → `artifacts.stores`; session service → `memory.stores` | `Agent` config → `flow.graph`; artifacts → `artifacts.stores`; eval → evaluator nodes |
| **OpenAI Agents SDK** | Python, TS | agents → flow/subflow; tools → `tools.http_apis`/`mcp_servers`; handoffs → edges; guardrails → `guardrails.interrupts`; tracing → `observability` | `Agent` config → flow + tools; handoff graph → edges; input guardrails → interrupts |
| **Pydantic AI** | Python | agents → flow nodes; tools → `tools.http_apis`; deps type → params; model → `model_ref` | Agent/tool config → `flow.graph`; model settings → `runtime.models` |
| **Semantic Kernel** | Python | plugins/functions → tools; AI services → `runtime.models`; `ProcessStep` → flow nodes; loop → loop node | Plugin manifest → tools; kernel config → runtime |

---

## 4. Translation Fidelity Contract

| ADP section | Fidelity | Behavior when adapter_hints absent |
|---|---|---|
| `flow.graph` (nodes + edges) | **Faithful** | Direct mapping to framework graph topology |
| `tools` (http_apis, mcp_servers, sql_functions) | **Faithful** | Mapped to framework tool type |
| `runtime.models[]` | **Faithful** | Mapped to framework LLM binding |
| `tools.policy` | **Lossy** | retry/timeout emitted as wrapper; rate_limit/cache skipped if framework lacks support |
| `memory.stores[]` | **Lossy** | Type + scope hint used; provider config from adapter_hints |
| `memory.working` | **Lossy** | Strategy mapped to framework memory class; window_size/max_tokens where supported |
| `loop.termination` | **Lossy** | `max_iterations` faithful; condition expression may need manual adaptation |
| `guardrails.interrupts[]` | **Lossy** | HITL intent mapped; notification channel not auto-translated |
| `workspace` | **Unsupported (most)** | Emitted into manifest `extensions`; adapter emits warning |
| `tools.sandbox[]` | **Unsupported (most)** | Emitted into manifest `extensions`; adapter emits warning |
| `artifacts` | **Framework-specific** | Google ADK: faithful; others: emitted into `extensions` |
| `observability` | **Faithful (OTel)** | OTel backend mapped; framework-specific backends via adapter_hints |

**`import_from` guarantee**: untranslatable fields MUST land in `manifest.extensions`, never be silently discarded. This is the normative requirement enforced by check 37.

---

## 5. Optional Dependencies (Python packaging)

Adapter modules are optional extras — users install only what they need:

```toml
# pyproject.toml extras
[project.optional-dependencies]
langgraph = ["langgraph>=0.2"]
autogen = ["pyautogen>=0.3"]
crewai = ["crewai>=0.60"]
llamaindex = ["llama-index>=0.12"]
google-adk = ["google-adk>=1.0"]
openai-agents = ["openai-agents>=0.1"]
pydantic-ai = ["pydantic-ai>=0.1"]
semantic-kernel = ["semantic-kernel>=1.0"]
adapters = ["langgraph>=0.2", "pyautogen>=0.3", "crewai>=0.60", ...]
```

Core `adp-sdk` imports no framework adapter dependencies. Importing an adapter without the optional extra installed raises `ImportError` with a clear install hint.

---

## 6. TypeScript Adapter Scope

TypeScript hosts adapters only for frameworks with mature JS SDKs:
- `sdk/typescript/src/adapters/langgraph.ts` — LangGraph JS (`@langchain/langgraph`)
- `sdk/typescript/src/adapters/openai_agents.ts` — OpenAI Agents SDK JS (`openai`)

Both implement a `IAdapter` interface mirroring the Python `AdapterBase` contract.

---

## 7. Rust and Go: Adapter Stubs

Rust and Go are execution and validation SDKs, not orchestration frameworks. For these SDKs, adapters are thin stubs that:

1. Parse `runtime.adapter_hints` into a typed struct
2. Expose a `GetHints(frameworkID string) map[string]interface{}` (Go) / `fn get_hints(framework_id: &str) -> Option<&Value>` (Rust) for runner use
3. Do NOT implement full export/import logic — that is Python/TypeScript territory

---

## 8. Semantic Validation Checks

- **Check 36**: `runtime.adapter_hints` keys MUST be from the known framework enum: `langgraph | autogen | crewai | llamaindex | google_adk | openai_agents | pydantic_ai | semantic_kernel`. Unknown keys emit a **warning** (not error — forward compat for new frameworks).
- **Check 37**: Normative contract on adapter implementations — `import_from()` MUST place any fields with no ADP equivalent into `manifest.extensions`. Validators emit a warning when `extensions` is absent on a manifest produced by `import_from()` that contained untranslatable content.

---

**Expert skills applied**: `role-senior-software-engineer`, `role-senior-agentic-ai-developer`

*This document is part of the ADP v0.3.0 specification. See [adp-v0.3.0.md](adp-v0.3.0.md) for the master specification.*
