# ADP → Semantic Kernel Runner

Demonstrates importing an ADP manifest into a Semantic Kernel `Kernel` + process step structure.

**Import only.** Export (SK → ADP) is deferred to v0.3.0.

## Requirements

- semantic-kernel >= 1.3 (Python SDK)
- Note: `KernelProcess` is experimental in SK Python (stable in C#)

## Quick Start

```bash
pip install -r requirements.txt
pip install -e ../../../sdk/python
pytest -v
```

## What Each Test Verifies

| Test | What it checks |
|---|---|
| `test_process_steps_created_for_all_nodes` | One step descriptor is created per manifest node; step IDs match node IDs |
| `test_composition_resolves_before_build` | `resolve_adp` composition runs before the SK build; all billing-variant nodes have corresponding steps |
| `test_tool_nodes_mapped_to_steps` | `tool` nodes produce step descriptors with `kind == "tool"` and the correct `tool_ref` |
| `test_model_ref_resolved_from_runtime` | `llm` nodes resolve `model_ref` through `runtime.models[]` to `provider` + `model` name |

## Mock Mode

All tests run in **mock mode** — no `semantic_kernel` installation is required for the basic test suite. When SK is not installed, `build_sk_from_adp` returns a `{"type": "mock_kernel"}` dict instead of a live `Kernel` instance; step descriptors are identical in both modes.

When SK is installed, each step descriptor additionally carries a `sk_construct` hint (`"KernelFunction"` for `llm` nodes, `"KernelPlugin"` for `tool` nodes).

## Architecture

```
build_sk_from_adp(manifest, backend_factory=None)
  → (kernel: Kernel | {"type": "mock_kernel"}, process_steps: list[dict])
```

Each step descriptor:

```python
# llm node
{"id": "chat", "kind": "llm", "model_ref": "gpt4", "provider": "openai", "model": "gpt-4o-mini"}

# tool node
{"id": "lookup", "kind": "tool", "tool_ref": "billing-api"}

# other node (input/output/router/...)
{"id": "input", "kind": "input"}
```

## Export

Export (SK → ADP) is deferred to **v0.3.0**. See the roadmap for details.

## Links

- [Framework Interop Guide](../../../spec/framework-interop.md) — full LangGraph/AutoGen/SK/CrewAI mapping
- [ADP Spec](../../../spec/adp-v0.1.0.md) — full protocol specification
