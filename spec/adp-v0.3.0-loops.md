# ADP v0.3.0 Orchestration Loops Specification

**Agent Definition Protocol — Orchestration Loops v0.3.0**

> **Version**: v0.3.0  
> **Status**: Draft  
> **Editor**: ADP Working Group  
> **Last Updated**: 2026-05-25  

---

## Abstract

This document introduces a `loop` node kind for explicit bounded iteration in the AFG (Agent Flow Graph) flow graph. This addresses the gap in ADP v0.2.0 where iteration was only possible through implicit graph cycles without explicit bounds or termination conditions.

---

## Table of Contents

1. [Schema Additions](#1-schema-additions)
2. [Body Nodes Execution Order Semantics](#2-body-nodes-execution-order-semantics)
3. [Ralph Loop (Context-Restart) Pattern](#3-ralph-loop-context-restart-pattern)
4. [Nested Loop Rules](#4-nested-loop-rules)
5. [Multi-Bound Termination Semantics](#5-multi-bound-termination-semantics)
6. [Framework Mapping](#6-framework-mapping)
7. [ESP Section D9](#7-esp-section-d9)
8. [Semantic Validation Checks](#8-semantic-validation-checks)

---

## 1. Schema Additions

Two changes to `schemas/flow.schema.json`:

### 1a. Add `"loop"` to the `kind` enum

The node `kind` enum now includes `"loop"`:
```
"kind": {
  "type": "string",
  "enum": ["input", "output", "llm", "tool", "router", "retriever", "evaluator", "subflow", "loop"]
}
```

### 1b. Add `body_nodes` + `termination` as optional node-level fields

Loop nodes have two new properties:

```yaml
- id: "correction-loop"
  kind: "loop"
  body_nodes: ["generate", "verify"]     # node IDs forming the loop body
  termination:
    condition: "context.verify.passed == true"  # exit when true
    max_iterations: 5                           # hard upper bound
    max_tokens: 50000                           # exit when cumulative token usage exceeds
    on_max_exceeded: "use_last"                 # use_last | fail | escalate
    restart_context: true                      # Ralph Loop: restart context on compaction threshold
```

### 1c. Add `loop_policy` at the flow root level

```yaml
flow:
  id: "main-flow"
  graph:
    nodes: [...]
    edges: [...]
    start_nodes: [...]
    end_nodes: [...]
  loop_policy:
    default_max_iterations: 10         # applies to graph cycles without explicit loop node
    on_max_exceeded: "fail"
    total_run_max_iterations: 50       # absolute cap across ALL loops in a single run
```

---

## 2. Body Nodes Execution Order Semantics

**`body_nodes[]` is a membership declaration, not an execution sequence.**

The execution order within the loop body is determined by the existing flow graph edge topology connecting those nodes — specifically the edges already declared in `flow.graph.edges[]`. The runner resolves which edges are "in scope" for the loop body by restricting to edges where both `from` and `to` are in `body_nodes[]`.

If the `body_nodes[]` are not connected by any edges in the graph, the runner MUST emit a semantic validation error (Check 15b). A loop with disconnected body nodes has no defined execution order.

**Example:**
```yaml
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
    to: "plan"  # This edge creates the loop cycle
```

Here, the loop body execution order is: `plan` → `execute` → `check` → (back to `plan`).

---

## 3. Ralph Loop (Context-Restart) Pattern

The **Ralph Loop** (from LangChain article) addresses long-horizon tasks that span multiple context windows. When a loop iteration exhausts the working context, the agent restarts with the original prompt in a fresh context window, using persistent memory to maintain continuity.

In ADP, this is expressed via a `restart_context` flag on loop termination:

```yaml
- id: "long-horizon-loop"
  kind: "loop"
  body_nodes: ["plan", "execute", "checkpoint"]
  termination:
    condition: "context.checkpoint.done == true"
    max_iterations: 20
    on_max_exceeded: "fail"
    restart_context: true
```

### Runner Semantics for `restart_context: true`

- When `memory.working.compaction_threshold_tokens` is exceeded mid-iteration, the runner finishes the current iteration normally, then starts the next iteration with a fresh in-process context.
- If no `compaction_threshold_tokens` is set, `restart_context: true` means **restart on every iteration boundary** (unconditional restart variant — each iteration always starts with a clean context window).
- `memory.stores[]` (episodic/semantic) and `state.session` survive the restart.
- The original invocation `inputs` are re-injected at the start of each fresh context (ensuring goal coherence).
- This is purely a runner behavior hint; it does not affect the flow graph topology.

### Interaction with `max_tokens`

`restart_context` and `max_tokens` are **independent mechanisms**:
- `restart_context` triggers on compaction threshold mid-iteration (starts fresh context, does not exit the loop)
- `max_tokens` tracks cumulative token usage and triggers `on_max_exceeded` when the overall bound is reached (exits the loop)

A loop can have both: the context restarts on threshold, but the loop still terminates when total token usage hits `max_tokens`.

---

## 4. Nested Loop Rules

- **Nested `loop` nodes are permitted**: A loop can contain another loop in its `body_nodes`
- **No self-reference**: A loop MUST NOT reference itself in `body_nodes` (Check 16 — no self-reference, directly or transitively)
- **No mutual nesting**: If loop A contains loop B in `body_nodes`, then loop B MUST NOT contain loop A (enforced transitively by Check 16)
- **Inherited bounds**: Subflow nodes inside a loop inherit the outer loop's remaining `max_iterations` as an upper bound for their own composition resolution

---

## 5. Multi-Bound Termination Semantics

### `on_max_exceeded: "escalate"` Hard Dependency

If `escalate` is declared on a loop node but no `guardrails.interrupts[]` entry exists with a matching `trigger: "loop_max_exceeded"`, the runner MUST treat it as a **permanent failure**. No silent runaway loops.

### Multi-Bound Semantics

When both `max_iterations` and `max_tokens` are declared, the loop terminates as soon as **either** bound is exceeded first (logical OR). `on_max_exceeded` fires for both cases.

`max_tokens` tracks **cumulative token usage across all iterations** of that loop — not per-iteration. The `terminated_by` field in ESP D9 records which bound triggered the exit.

**Example:**
```yaml
- id: "refinement-loop"
  kind: "loop"
  body_nodes: ["draft-subagent", "review-subagent"]
  termination:
    condition: "context.review-subagent.approved == true"
    max_iterations: 3
    max_tokens: 100000
    on_max_exceeded: "escalate"
```

If this loop hits either 3 iterations OR 100,000 cumulative tokens, it terminates and escalates to HITL.

---

## 6. Framework Mapping

| ADP Field | LangGraph | AutoGen | CrewAI | Semantic Kernel |
|---|---|---|---|---|
| `max_iterations` | `recursion_limit` | `max_turns` | `max_iterations` | explicit counter |
| `max_tokens` | n/a | `TokenUsageTermination` | n/a | explicit counter |
| `termination.condition` | routing fn returning `END` | `termination_condition` lambda | `@task` return value | `KernelArguments` check |
| `on_max_exceeded: "escalate"` | `human_input_mode: ALWAYS` | `ALWAYS` input mode | human_input callback | `ProcessEvent` |

**ADP loop bounds win** over `adapter_hints` when both are present. A semantic validation warning is emitted for conflicts.

---

## 7. ESP Section D9

Loop node state writes to `state.context[loop_node.id]`:

```json
{
  "context": {
    "correction-loop": {
      "iterations": 3,
      "terminated_by": "condition",   // "condition" | "max_iterations" | "max_tokens"
      "last_output": { ... }
    }
  }
}
```

**`terminated_by` values:**
- `"condition"`: Loop exited because `termination.condition` evaluated to true
- `"max_iterations"`: Loop exited because `max_iterations` bound was reached
- `"max_tokens"`: Loop exited because `max_tokens` bound was reached

---

## 8. Semantic Validation Checks

- **Check 15**: `loop.body_nodes[]` must reference known node IDs in `flow.graph.nodes[]`
- **Check 15b**: `loop.body_nodes[]` must contain at least 2 nodes that are connected by at least one edge in `flow.graph.edges[]` (disconnected body = undefined execution order)
- **Check 16**: Loop node MUST NOT reference itself (directly or transitively) in `body_nodes`

---

## Appendix: Self-Verification Loop Pattern

The canonical self-verification loop pattern (maps to LangChain article):

```yaml
nodes:
  - id: "codegen"
    kind: "llm"
    model_ref: "primary"
  - id: "run-tests"
    kind: "tool"
    tool_ref: "python-exec"
    params:
      command: "pytest tests/ --tb=short 2>&1"
  - id: "verify-loop"
    kind: "loop"
    body_nodes: ["codegen", "run-tests"]
    termination:
      condition: "context.run-tests.exit_code == 0"
      max_iterations: 5
      on_max_exceeded: "fail"

edges:
  - from: "codegen"
    to: "run-tests"
  - from: "run-tests"
    to: "codegen"  # Loop back if tests fail
```

---

**Expert skills applied**: `role-senior-software-engineer`, `role-senior-agentic-ai-developer`

*This document is part of the ADP v0.3.0 specification. See [adp-v0.3.0.md](adp-v0.3.0.md) for the master specification.*
