# ADR: ESP Node Semantics (D1–D7)

**Status**: Accepted  
**Date**: 2026-05-15  
**Authority**: This document is the authoritative record for per-node state behavior. If spec prose and this ADR disagree, fix the spec — do not amend this ADR without recording the reason below.

---

## Context

`spec/esp.md` defines a state model with four core fields (`inputs`, `context`, `memory`, `tool_responses`) and a traversal model, but left per-node write semantics unspecified (stub at line 382). Before writing spec prose, the key design questions were resolved here.

---

## Decisions

### D1 — `input` node: where does payload land?

**Decision**: Invocation payload is written to `state.inputs` verbatim. No sub-key is introduced.

**Rationale**: `state.inputs` is defined as the immutable invocation input. Wrapping it in a sub-key (e.g., `state.inputs["user"]`) adds indirection with no benefit — all downstream nodes already reference `state.inputs` directly. Keeping it flat matches the state model definition in esp.md §State Model.

---

### D2 — `llm` node: which context key does the response write to?

**Decision**: The LLM response is written to `state.context[node.id]`.

**Rationale**: Using `node.id` as the key makes the write target deterministic and predictable from the manifest — downstream nodes can reference `$.context.<node_id>` in edge conditions without requiring runtime documentation. It also prevents collisions between multiple LLM nodes in the same flow.

---

### D3 — `tool` node: how are results merged into state?

**Decision**: Results are appended to `state.tool_responses[node.id]`. The array is never replaced on re-execution; each invocation adds a new entry.

**Rationale**: Append-only semantics preserve the full invocation history within a run, which is important for debugging, evaluation, and audit trails. `node.id` (not `tool_ref`) is used as the key so that two tool nodes referencing the same tool can be distinguished by their position in the flow. This is consistent with the `tool_responses` structure already defined in esp.md §State Model.

---

### D4 — `router` node: what are the defined strategy values?

**Decision**: `strategy` is an enum with three values: `sequence`, `conditional`, `parallel`.

- `sequence`: activate edges one at a time in declaration order; proceed to next only when previous path completes.
- `conditional`: evaluate edge conditions; activate all edges whose conditions evaluate to `true`.
- `parallel`: activate all outgoing edges simultaneously (subject to observable ordering).

**Rationale**: These three cover the primary routing patterns without over-specifying. `conditional` is the most common case (matches the existing `edge.condition` mechanism). `sequence` and `parallel` are natural complements. Leaving `strategy` as a free string would make schema validation useless and create interoperability risk. Extensibility is preserved via `extensions` field.

---

### D5 — `retriever` node: how is the memory binding resolved?

**Decision**: A `memory_ref` field on the node names the memory provider to query. Results are written to `state.context[node.id]`.

**Rationale**: `memory_ref` provides an explicit, schema-validatable binding (analogous to `tool_ref` on tool nodes). Writing results to `state.context[node.id]` follows the same pattern as D2 — deterministic, predictable, and collision-free. The runner resolves `memory_ref` against `adp.memory` first, then a runner-provided registry.

---

### D6 — `evaluator` node: relationship to evaluation suites?

**Decision**: A `suite_ref` field on the node references an evaluation suite by ID (matching `evaluation.suites[].id`). The runner writes `{"passed": <bool>, "score": <number>}` to `state.context[node.id]`. A `blocking` field (bool, default `false`) on the node halts the flow on failure when set to `true`.

**Rationale**: `suite_ref` makes the binding explicit and schema-validatable. Writing a structured result to `state.context[node.id]` allows downstream nodes (e.g., a router) to inspect evaluation outcome via edge conditions. `blocking` defaults to `false` so that non-blocking evaluation is the safe default — callers opt in to halting behavior explicitly.

---

### D7 — `output` node: what is the terminal write?

**Decision**: An `output_ref` field on the node names the `context` key to return as the run result. If `output_ref` is absent, the runner falls back to the last key written to `state.context` during the run.

**Rationale**: An explicit `output_ref` removes ambiguity about which context key is the final answer, which is especially important when multiple LLM or tool nodes write to context. The fallback to last-written preserves backward compatibility with flows that do not specify `output_ref`.

---

## Consequences

- `schemas/flow.schema.json` must add: `memory_ref` (string), `suite_ref` (string), `output_ref` (string), `blocking` (boolean) to the node definition.
- `schemas/flow.schema.json` must change `strategy` from untyped string to `enum: ["sequence", "conditional", "parallel"]`.
- `spec/esp.md §Flow Node Semantics` must be written using these decisions as the authority.
- `spec/flow.md` should add a reference to this ADR and to the per-node semantics section in esp.md.
