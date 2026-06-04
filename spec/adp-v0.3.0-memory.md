# ADP v0.3.0 Memory & Context Specification

**Agent Definition Protocol — Memory & Context Layer v0.3.0**

> **Version**: v0.3.0  
> **Status**: Draft  
> **Editor**: ADP Working Group  
> **Last Updated**: 2026-05-25  

---

## Abstract

This document formalizes the **Memory & Context** layer as ADP's fifth harness layer, joining Execution, Observation, Safety, and Testing. The memory gap was identified as the largest in ADP v0.2.0: the current `memory` block is a flat 4-field object describing only where to connect a vector store, not what memory does or how context is assembled.

---

## Table of Contents

1. [Memory Type Taxonomy](#1-memory-type-taxonomy)
2. [Schema](#2-schema)
3. [Backward Compatibility](#3-backward-compatibility-rule)
4. [Memory Store Composition Semantics](#4-memory-store-composition-semantics)
5. [ESP State Extension](#5-esp-state-extension)
6. [Framework Adapter Hints](#6-framework-adapter-hints)
7. [Semantic Validation Checks](#7-semantic-validation-checks)

---

## 1. Memory Type Taxonomy

Two store types (external-backed) plus one in-process block:

| Type | Scope | Lifecycle | Provider needed |
|---|---|---|---|
| `episodic` | Single session | Erased at TTL or session end | Yes (external) |
| `semantic` | Cross-session, shared | Persistent until TTL or explicit clear | Yes (external) |
| *(working)* | Single run, in-process | Erased at run end | No — declared via `memory.working` sub-block only |

**Design decision**: `working` is **NOT** a valid `stores[].type`. Working memory is always in-process, always declared via `memory.working`. This avoids ambiguity between "store with type working" and "in-process buffer." Runners needing an external backing for ephemeral session memory use `runtime.adapter_hints`.

---

## 2. Schema

The flat `memory:` block becomes a `oneOf` accepting both the legacy v0.1.x form (backward compat) and the new structured form. See `schemas/memory.schema.json` for the full JSON Schema.

### Structured v0.3.0 Format

```yaml
memory:
  stores:
    - id: "long-term"
      type: "semantic"           # episodic | semantic
      provider: "pinecone"
      endpoint: "https://..."
      index: "agent-knowledge"
      scope: "agent"             # agent | session | user
      ttl_seconds: 2592000       # 30 days; absent = no expiry
      pii: false

    - id: "session-history"
      type: "episodic"
      provider: "redis"
      endpoint: "redis://..."
      ttl_seconds: 3600
      scope: "session"
      pii: true

  working:
    strategy: "sliding_window"   # sliding_window | full | summary
    window_size: 20
    max_tokens: 4096
    summary_model_ref: "primary"    # REQUIRED when strategy=summary
    compaction_threshold_tokens: 6000  # trigger summarization when exceeded; absent = no threshold

  context_assembly:
    apply_to_node_kinds: ["llm"]  # default; add "retriever" or "evaluator" to extend
    order:
      - source: "working"
      - source: "store"
        store_ref: "long-term"
        top_k: 5
        relevance_threshold: 0.75
      - source: "store"
        store_ref: "session-history"
        top_k: 10
    max_total_tokens: 8192
    static_injection:
      - id: "agent-instructions"
        source: "file"
        path: "knowledge/AGENTS.md"
        position: "prepend"
        max_tokens: 2000
      - id: "domain-glossary"
        source: "inline"
        content: |
          # Domain glossary
          ...
        position: "append"

  operations:
    - on_event: "on_invoke_end"
      op: "write"
      store_ref: "session-history"
      fields: ["output"]
    - on_event: "on_invoke_end"
      op: "summarize"
      store_ref: "session-history"
      when: "context.working.message_count > 20"

  retention:
    pii_policy: "redact"          # redact | encrypt | block | log
    user_consent_required: true
    data_residency: ["eu-west-1"]
    auto_clear_on: "session_end"  # session_end | agent_stop | never
```

### Field Descriptions

| Field | Type | Required | Description |
|---|---|---|---|
| `stores[].id` | string | Yes | Unique identifier for this store |
| `stores[].type` | enum | Yes | `episodic` or `semantic` (NOT `working`) |
| `stores[].provider` | string | Yes | Provider name (e.g., "pinecone", "redis", "weaviate") |
| `stores[].endpoint` | string | No | Provider endpoint URL |
| `stores[].index` | string | No | Index/namespace name |
| `stores[].scope` | enum | No | `agent` (default), `session`, or `user` |
| `stores[].ttl_seconds` | integer | No | Time-to-live in seconds; absent = no expiry |
| `stores[].pii` | boolean | No | Whether store contains PII; default false |
| `working.strategy` | enum | No | `sliding_window` (default), `full`, or `summary` |
| `working.window_size` | integer | No | Number of messages to keep in sliding window |
| `working.max_tokens` | integer | No | Maximum tokens for working memory |
| `working.summary_model_ref` | string | Conditional | REQUIRED when `strategy=summary`; references `runtime.models[].id` |
| `working.compaction_threshold_tokens` | integer | No | Trigger compaction when working memory exceeds this; must be ≤ `max_tokens` |
| `context_assembly.apply_to_node_kinds` | array | No | Node kinds to apply context assembly to; default `["llm"]`; valid: `llm`, `retriever`, `evaluator` |
| `context_assembly.order[]` | array | No | Ordered list of context sources |
| `context_assembly.max_total_tokens` | integer | No | Maximum total tokens for assembled context |
| `context_assembly.static_injection[]` | array | No | Static knowledge to inject into context |
| `operations[]` | array | No | Declarative memory write operations |
| `retention.pii_policy` | enum | No | PII handling policy; default `"redact"` |
| `retention.user_consent_required` | boolean | No | Whether user consent is required; default false |
| `retention.data_residency` | array | No | Allowed data residency regions |
| `retention.auto_clear_on` | enum | No | When to auto-clear; default `"never"` |

---

## 3. Backward Compatibility Rule

A runner seeing root-level `provider` on `memory` MUST silently upcast it to `stores[0]` with:
- `type: "semantic"`
- `scope: "agent"`
- All original fields preserved

**Example:**
```yaml
# v0.1.x / v0.2.0 format
memory:
  provider: "pinecone"
  endpoint: "https://..."
  index: "knowledge"
  namespace: "default"

# Runner upcasts to:
memory:
  stores:
    - id: "legacy-store"
      type: "semantic"
      provider: "pinecone"
      endpoint: "https://..."
      index: "knowledge"
      scope: "agent"
      namespace: "default"
```

No existing manifests break. The `adp.schema.json` `oneOf` accepts both forms.

---

## 4. Memory Store Composition Semantics

When manifests are composed (`import`, `extends`):
- `import` **appends** stores arrays (same as other arrays in additive merge)
- If two stores share the same `id` after merge, the **outer/later manifest wins** (consistent with RFC 7396 merge semantics used for `extends`)
- `overrides` (RFC 6901 JSON Pointer) can patch any store field by ID
- **Check 18** (duplicate store IDs) is applied after composition resolution, not before

---

## 5. ESP State Extension

The session-scoped state gains a `memory` key — additive to the existing ESP state model (no existing checks broken):

```json
{
  "state": {
    "inputs": { ... },
    "context": { ... },
    "memory": {
      "long-term": { ... },
      "session-history": { ... }
    },
    "tool_responses": { ... },
    "session": {
      "id": "sess-abc123",
      "started_at": "2026-05-25T08:00:00Z",
      "turn_count": 3
    }
  }
}
```

The `session` key is new in v0.3.0 and scoped to runners implementing `scope="session"` stores.

---

## 6. Framework Adapter Hints

Framework-specific memory configuration is declared in `runtime.adapter_hints`, not in `memory`:

```yaml
runtime:
  adapter_hints:
    langgraph:
      checkpointer: "postgres"
      memory_store: "redis"
    crewai:
      memory: true
      embedder_config:
        provider: "openai"
        model: "text-embedding-3-small"
```

---

## 7. Semantic Validation Checks

- **Check 18**: `memory.stores[]` IDs must be unique (post-composition)
- **Check 19**: `memory.operations[].store_ref` must reference a known `stores[].id`
- **Check 20**: `memory.context_assembly.order[].store_ref` must reference a known `stores[].id`
- **Check 21**: `memory.working.summary_model_ref` (when present) must reference a known `runtime.models[].id`
- **Check 21b**: `memory.working.summary_model_ref` MUST be present when `memory.working.strategy = "summary"`
- **Check 21c**: `memory.working.compaction_threshold_tokens` (when present) MUST be ≤ `memory.working.max_tokens`
- **Check 24**: `memory.context_assembly.static_injection[].path` (when `source: "file"`) must be a relative path without `..` traversal; must also reference a declared `workspace`

---

## Appendix: Context Assembly Details

### Order Resolution

The `context_assembly.order[]` array defines the order in which context sources are injected. Each source contributes to the final context:

1. **`source: "working"`**: In-process working memory (sliding window, full, or summary)
2. **`source: "store"`**: External memory store with `store_ref`, `top_k`, and `relevance_threshold`

The runner concatenates all sources in order, then truncates to `max_total_tokens`.

### Static Injection Resolution

When `static_injection[].source: "file"`:
- `path` is resolved relative to `workspace.root`
- If no `workspace` is declared, the runner MUST reject file-source static injection (Check 24)
- The file content is read at runtime and injected at the specified `position` (`prepend` or `append`)
- Content is truncated to `max_tokens` if specified

---

**Expert skills applied**: `role-senior-software-engineer`, `role-senior-agentic-ai-developer`

*This document is part of the ADP v0.3.0 specification. See [adp-v0.3.0.md](adp-v0.3.0.md) for the master specification.*
