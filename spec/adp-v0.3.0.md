# ADP v0.3.0 Specification

**Agent Definition Protocol v0.3.0 — Harness-Covering Features**

> **Version**: v0.3.0  
> **Status**: Draft  
> **Editor**: ADP Working Group  
> **Last Updated**: 2026-05-25  

---

## Abstract

ADP v0.3.0 introduces **8 new harness layers** to the Agent Definition Protocol, transforming it from a basic agent description format into a comprehensive **agent harness specification**. These additions address the largest gaps identified in real-world agent deployment: memory, loops, tooling policy, guardrails, workspace, sandbox, artifacts, and observability.

This document is the **master specification** for v0.3.0. Each harness layer has its own detailed specification document:

| Harness Layer | Spec Document | Purpose |
|---|---|---|
| **Memory & Context** | [`adp-v0.3.0-memory.md`](adp-v0.3.0-memory.md) | Fifth harness layer: episodic/semantic stores, working memory, context assembly |
| **Orchestration Loops** | [`adp-v0.3.0-loops.md`](adp-v0.3.0-loops.md) | Explicit bounded iteration with `loop` node kind |
| **Tooling Policy** | [`adp-v0.3.0-tools-policy.md`](adp-v0.3.0-tools-policy.md) | Retry, timeout, cache, rate-limit, progressive loading |
| **Guardrails (Extended)** | [`adp-v0.3.0-guardrails.md`](adp-v0.3.0-guardrails.md) | HITL interrupts, cost enforcement, agent trust levels |
| **Workspace & Storage** | [`adp-v0.3.0-workspace.md`](adp-v0.3.0-workspace.md) | Filesystem binding, git integration, remote mounts |
| **Execution Sandbox** | [`adp-v0.3.0-sandbox.md`](adp-v0.3.0-sandbox.md) | Code execution environment with provider abstraction |
| **Artifacts** | [`adp-v0.3.0-artifacts.md`](adp-v0.3.0-artifacts.md) | Versioned named outputs |
| **Observability** | [`adp-v0.3.0-observability.md`](adp-v0.3.0-observability.md) | Declarative OTel tracing + cost reporting |
| **Framework Adapters** | [`adp-v0.3.0-adapters.md`](adp-v0.3.0-adapters.md) | SDK-level import/export modules for 8 frameworks |

---

## Table of Contents

1. [Overview](#overview)
2. [Design Principles](#design-principles)
3. [Changes from v0.2.0](#changes-from-v020)
4. [Conformance](#conformance)
5. [Migration Guide](#migration-guide)
6. [Document Index](#document-index)

---

## Overview

### What is a Harness?

An **agent harness** wraps an LLM to make it autonomous across multiple capability areas. ADP v0.1.0 and v0.2.0 covered:

- **Execution**: How the agent runs (`runtime.execution`)
- **Observation**: How the agent sees the world (`flow.graph`)
- **Safety**: How the agent stays within bounds (`guardrails`)
- **Testing**: How the agent is evaluated (`evaluation`)

v0.3.0 adds **5 new harness areas** plus **3 extensions** to existing areas:

### New Harness Layers (v0.3.0)

| Layer | v0.2.0 Status | v0.3.0 Addition |
|---|---|---|
| **Memory & Context** | Stub (4-field object) | Full layer: stores, working memory, context assembly, retention |
| **Orchestration Loops** | Implicit (graph cycles) | Explicit `loop` node kind with bounded iteration |
| **Workspace & Storage** | Absent | Filesystem binding, git, remote mounts |
| **Execution Sandbox** | Absent | Code execution environment |
| **Artifacts** | Absent | Versioned named outputs |
| **Observability** | Absent | Declarative tracing config |
| **Guardrails** | v0.2.0 (input/output) | Extended: HITL, cost, agent trust |
| **Tooling** | v0.1.0 (basic tools) | Extended: policy (retry, timeout, cache, rate-limit), load_strategy |
| **Adapters** | Passive hints only | Active SDK-level import/export modules |

---

## Design Principles

### 1. Backward Compatibility

All new fields in v0.3.0 are **optional**. A v0.2.0 manifest is a valid v0.3.0 manifest. The `adp_version` field accepts `"0.1.0"`, `"0.2.0"`, and `"0.3.0"`.

**Memory backward compatibility rule**: A runner seeing root-level `provider` on `memory` MUST silently upcast it to the v0.3.0 structured format with a single store of type `"semantic"` and scope `"agent"`.

### 2. Portability First

ADP manifests should be **framework-agnostic**. Framework-specific behavior is declared via:

- `runtime.adapter_hints`: Passive hints for runners
- `spec/adp-v0.3.0-adapters.md`: Active SDK-level adapters for import/export

### 3. Runner Responsibility

ADP declares **what** the harness should do; the **runner** implements **how**. For example:

- ADP declares `memory.working.compaction_threshold_tokens`
- Runner implements the context compaction logic
- ADP declares `loop.termination.max_tokens`
- Runner tracks cumulative token usage and enforces the bound

### 4. Composition-Friendly

All new array fields follow **additive merge** semantics on composition:

- `import`: appends arrays
- `extends` + local fields: objects deep-merge; id-carrying lists merge by `id` (local entry wins on collision); other lists replace
- Duplicate IDs: local/later manifest wins

---

## Changes from v0.2.0

### Schema Changes

#### New Top-Level Sections

1. **`memory`**: OneOf accepting legacy v0.1.x format OR new structured v0.3.0 format
2. **`workspace`**: Filesystem harness primitive
3. **`artifacts`**: Versioned named output storage
4. **`observability`**: Declarative tracing configuration

#### Extended Sections

1. **`tools`**: Added `sandbox[]` type, `load_strategy` enum, `policy` object to all tool types
2. **`guardrails`**: Added `interrupts[]`, `cost`, `agent_trust` sub-objects
3. **`flow`**: Added `loop` to node kind enum, `body_nodes` and `termination` to node properties, `loop_policy` at flow root
4. **`runtime.adapter_hints`**: Added keys for all 8 framework adapters

#### New Schema Files

- `schemas/memory.schema.json`
- `schemas/workspace.schema.json`
- `schemas/sandbox.schema.json`
- `schemas/artifacts.schema.json`
- `schemas/observability.schema.json`

#### Composition Enhancement

**Id-keyed local field merge** — Local fields in a manifest that uses `extends:` now apply id-keyed merge semantics instead of RFC 7396 array-replace. Objects deep-merge recursively; lists where all local items carry `id` merge by matching id (matched entries patched in-place; unmatched base entries kept; unknown ids appended); lists with any no-id item replace the base list; `null` removes a key. Valid for all `adp_version` values.

See [`adp-v0.3.0-composition.md`](adp-v0.3.0-composition.md) for the full specification.

---

## Conformance

### Conformance Classes (Unchanged from v0.2.0)

- **ADP-Minimal**: Empty flow and evaluation allowed; suitable for documentation-only manifests
- **ADP-Full**: Non-empty flow and evaluation required

### New Optional Harness Profiles (v0.3.0)

| Profile | Description | MUST Implement |
|---|---|---|
| `ADP-Harness-Memory` | Memory & context layer | `memory.stores[]`, `memory.working`, `memory.context_assembly`, checks 18-21, 21b, 21c, 24 |
| `ADP-Harness-Loops` | Orchestration loops | `loop` node, `max_iterations` + `max_tokens` enforcement, checks 15, 15b, 16 |
| `ADP-Harness-ToolPolicy` | Tooling policy | `policy.retry`, `policy.timeout_ms`, `load_strategy`, checks 17, 29 |
| `ADP-Harness-Safety` | Extended guardrails | `guardrails.interrupts[]` with HITL resume, `agent_trust` enforcement, `guardrails.cost`, checks 22, 22b, 23, 30 |
| `ADP-Harness-Workspace` | Workspace & storage | `workspace` section, filesystem + git binding, remote mounts, checks 25, 25b, 26, 31 |
| `ADP-Harness-Sandbox` | Execution sandbox | `tools.sandbox[]` with provider abstraction, snapshotting, credential isolation, checks 27-28, 32 |
| `ADP-Harness-Artifacts` | Artifacts | `artifacts.stores[]`, versioning semantics, checks 33-34 |

### New ESP Requirements (v0.3.0)

- **ESP-Full**: If `loop.termination.max_iterations` or `loop.termination.max_tokens` is declared, runner MUST enforce it
- **ESP-Full**: If `on_max_exceeded` is absent when any numeric bound is declared, runner MUST default to `"fail"`

---

## Migration Guide

### From v0.2.0 to v0.3.0

1. **No breaking changes**: Your v0.2.0 manifest is already valid v0.3.0
2. **Update `adp_version`**: Change to `"0.3.0"` to signal support for new features
3. **Adopt new features incrementally**: Add new harness layers as needed

### Memory Migration

**v0.2.0 format:**
```yaml
memory:
  provider: "pinecone"
  endpoint: "https://..."
  index: "agent-knowledge"
  namespace: "default"
```

**v0.3.0 equivalent:**
```yaml
memory:
  stores:
    - id: "knowledge"
      type: "semantic"
      provider: "pinecone"
      endpoint: "https://..."
      index: "agent-knowledge"
      scope: "agent"
```

Runners MUST perform this upcast automatically.

### Adding a Loop

**v0.2.0 (implicit cycle):**
```yaml
flow:
  graph:
    nodes:
      - id: "plan"
        kind: "llm"
      - id: "execute"
        kind: "tool"
      - id: "check"
        kind: "llm"
    edges:
      - from: "plan"
        to: "execute"
      - from: "execute"
        to: "check"
      - from: "check"
        to: "plan"
    start_nodes: ["plan"]
    end_nodes: ["check"]
```

**v0.3.0 (explicit loop):**
```yaml
flow:
  graph:
    nodes:
      - id: "plan"
        kind: "llm"
      - id: "execute"
        kind: "tool"
      - id: "check"
        kind: "llm"
      - id: "main-loop"
        kind: "loop"
        body_nodes: ["plan", "execute", "check"]
        termination:
          condition: "context.check.done == true"
          max_iterations: 10
    edges:
      - from: "plan"
        to: "execute"
      - from: "execute"
        to: "check"
      - from: "check"
        to: "plan"
    start_nodes: ["main-loop"]
    end_nodes: ["main-loop"]
  loop_policy:
    default_max_iterations: 10
```

---

## Document Index

### Core Specification

| Document | Description |
|---|---|
| [`adp-v0.3.0.md`](adp-v0.3.0.md) | **This document** — Master v0.3.0 spec |
| [`adp-v0.2.0.md`](adp-v0.2.0.md) | v0.2.0 specification (immutable) |
| [`adp-v0.1.0.md`](adp-v0.1.0.md) | v0.1.0 specification (amended with v0.3.0 note) |

### Harness Layer Specifications (v0.3.0)

| Document | Harness Layer | Key Features |
|---|---|---|
| [`adp-v0.3.0-memory.md`](adp-v0.3.0-memory.md) | Memory & Context | episodic/semantic stores, working memory, context assembly, static injection, operations, retention |
| [`adp-v0.3.0-loops.md`](adp-v0.3.0-loops.md) | Orchestration Loops | `loop` node kind, `body_nodes`, `termination`, `restart_context`, `loop_policy` |
| [`adp-v0.3.0-tools-policy.md`](adp-v0.3.0-tools-policy.md) | Tooling Policy | `load_strategy`, `policy.retry`, `policy.timeout_ms`, `policy.rate_limit`, `policy.cache` |
| [`adp-v0.3.0-guardrails.md`](adp-v0.3.0-guardrails.md) | Guardrails (Extended) | `interrupts[]` (HITL), `cost`, `agent_trust` with normative enforcement table |
| [`adp-v0.3.0-workspace.md`](adp-v0.3.0-workspace.md) | Workspace & Storage | `root`/`root_env_var`, git config, permissions, mounts, cleanup |
| [`adp-v0.3.0-sandbox.md`](adp-v0.3.0-sandbox.md) | Execution Sandbox | `tools.sandbox[]`, provider abstraction, snapshotting, credential isolation |
| [`adp-v0.3.0-artifacts.md`](adp-v0.3.0-artifacts.md) | Artifacts | `artifacts.stores[]`, versioning, scoping (session/user/agent) |
| [`adp-v0.3.0-observability.md`](adp-v0.3.0-observability.md) | Observability | `tracing` (OTel backend), `cost_reporting`, trace events enum |
| [`adp-v0.3.0-adapters.md`](adp-v0.3.0-adapters.md) | Framework Adapters | `AdapterBase` ABC, `AdapterRegistry`, 8 framework adapters, fidelity contract |

### Related Documents

| Document | Description |
|---|---|
| [`conformance.md`](conformance.md) | Conformance classes, checks, harness profiles (amended for v0.3.0) |
| [`esp.md`](esp.md) | Execution Semantics & Protocol (amended: D9 loop node semantics, tool policy order, session state, artifact write phase) |
| [`runtime.md`](runtime.md) | Runtime specification |
| [`flow.md`](flow.md) | Flow specification |
| [`evaluation.md`](evaluation.md) | Evaluation specification |

---

## Version History

| Version | Date | Changes |
|---|---|---|
| v0.3.0 | 2026-05-25 | Initial v0.3.0 release: 8 new harness layers, 20+ new validation checks, 8 framework adapters |

---

## Appendix: Quick Reference

### All v0.3.0 Node Kinds

```
input, output, llm, tool, router, retriever, evaluator, subflow, loop
```

### All v0.3.0 Tool Types

```
mcp_servers, http_apis, sql_functions, sandbox
```

### All v0.3.0 Memory Store Types

```
episodic, semantic  (NOT "working" — use memory.working sub-block)
```

### All v0.3.0 Loop Termination Modes

```
use_last, fail, escalate  (default: fail)
```

### All v0.3.0 Load Strategies

```
eager, lazy, on_demand
```

### All v0.3.0 Sandbox Runtimes

```
python, node, bash, browser, custom
```

### All v0.3.0 Sandbox Providers

```
docker, e2b, modal, daytona, vercel, cloudflare, runloop, blaxel, custom
```

### All v0.3.0 Artifact Scopes

```
session, user, agent
```

### All v0.3.0 Agent Trust Levels

```
sandboxed, supervised, autonomous
```

---

**Expert skills applied**: `role-senior-software-engineer`, `role-senior-agentic-ai-developer`

*This document is part of the ADP v0.3.0 specification. For questions, comments, or contributions, see [CONTRIBUTING.md](../CONTRIBUTING.md).*
