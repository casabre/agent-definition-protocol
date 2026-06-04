# ADP v0.3.0 Guardrails Specification

**Agent Definition Protocol — Guardrails v0.3.0 (Extends v0.2.0)**

> **Version**: v0.3.0  
> **Status**: Draft  
> **Editor**: ADP Working Group  
> **Last Updated**: 2026-05-25  
> **Note**: This document **extends** guardrails formalized in v0.2.0. It does NOT amend `spec/adp-v0.2.0.md` which remains immutable.

---

## Abstract

This document adds three additive features to the v0.2.0 guardrails: Human-in-the-loop interrupts, cost guardrails, and agent trust levels. All new fields are optional. Manifests using these fields SHOULD declare `adp_version: "0.3.0"`; validators emit a warning if v0.3.0 fields are present in a manifest with an older `adp_version`.

---

## Table of Contents

1. [Human-in-the-Loop Interrupts](#1-human-in-the-loop-interrupts)
2. [Cost Guardrails](#2-cost-guardrails)
3. [Agent Trust Level](#3-agent-trust-level)
4. [Semantic Validation Checks](#4-semantic-validation-checks)

---

## 1. Human-in-the-Loop Interrupts

```yaml
guardrails:
  interrupts:
    - id: "high-stakes-approval"
      trigger: "tool_call"             # tool_call | cost_threshold | loop_max_exceeded | custom
      tool_refs: ["delete-record"]     # which tool IDs trigger this (for tool_call trigger)
      mode: "pause_and_notify"         # pause_and_notify | block | log
      notification:
        channel: "webhook"
        endpoint_env_var: "APPROVAL_WEBHOOK_URL"
        timeout_seconds: 300
        on_timeout: "fail"             # fail | approve | deny
```

### HITL Resume Protocol (Normative Runner Requirement)

When the interrupt resolves, the runner MUST:

1. Read the resolution signal from `state.context["interrupt.<id>"]` — values: `approved`, `denied`, or `timeout_<on_timeout_value>`
2. **If `approved`**: resume the pending operation (re-execute the paused tool call or continue the loop)
3. **If `denied` or `timeout: fail`**: cancel the pending operation and return a permanent failure for the current flow node
4. **If `timeout: approve`**: proceed as approved
5. **If `timeout: deny`**: proceed as denied

The resolution signal is injected by the runner's notification channel integration; ADP does not define the transport — only the state key and semantics.

### Mode and Execution Mode

- `mode: "pause_and_notify"`: Always blocking by definition. The agent waits for human resolution. **`execution_mode` MUST NOT be set** (Check 22b).
- `mode: "block"` or `mode: "log"`: Can have `execution_mode`:
  - `"blocking"`: Check completes BEFORE agent invocation (prevents wasted tokens on invalid input)
  - `"parallel"`: Check runs concurrently with agent (best latency, but agent may start before rejection)

Maps to OpenAI Agents SDK guardrail `execution_mode` for input/output validation guardrails.

---

## 2. Cost Guardrails

```yaml
guardrails:
  cost:
    threshold_usd: 0.50
    on_threshold_exceeded: "block"     # block | warn | interrupt | downgrade
    interrupt_ref: "high-stakes-approval"  # required when on_threshold_exceeded="interrupt"
    downgrade_model_ref: "budget"      # required when on_threshold_exceeded="downgrade"
    track_by: "invocation"             # invocation | session | user | day
    model_refs: ["primary"]            # absent = all models
```

### Cost Tracking

Runner estimates cost from `gen_ai.usage.input_tokens` + `gen_ai.usage.output_tokens` × runner-owned price table. ADP declares the policy; runner owns pricing data.

### Downgrade Behavior

When `on_threshold_exceeded: "downgrade"`:
- Runner switches to the cheaper model (`downgrade_model_ref`) for **subsequent** LLM calls in the run
- This implements a "soft cap with degradation" pattern — more graceful than hard blocking
- The downgrade model MUST be present in `runtime.models[]` (Check 30)

### Relationship to Observability

| | `observability.cost_reporting` | `guardrails.cost` |
|---|---|---|
| **Type** | Passive metric emission | Active enforcement |
| **Effect** | Emits OTel metric; no side effect | Blocks / warns / interrupts / downgrades |
| **Threshold** | None (reports all spend) | `threshold_usd` triggers action |
| **Can coexist** | Yes | Yes |

---

## 3. Agent Trust Level

```yaml
guardrails:
  agent_trust:
    level: "supervised"                # sandboxed | supervised | autonomous
    side_effect_tool_refs: ["delete-record", "send-email"]  # tools classified as write/side-effect
```

### Normative Enforcement Table

| Operation | `sandboxed` | `supervised` | `autonomous` |
|---|---|---|---|
| Read-only tool calls | Allowed | Allowed | Allowed |
| Side-effect tool calls (`side_effect_tool_refs`) | MUST block | MUST trigger interrupt | Allowed (logged) |
| `memory.stores[]` writes | MUST block | Allowed | Allowed |
| `memory.stores[]` reads | Allowed | Allowed | Allowed |
| Streaming output | Allowed | Allowed | Allowed |
| Subagent delegation | MUST block | Allowed (inherits level) | Allowed |

If a runner declares `ADP-Harness-Safety` conformance, it MUST implement this table.

---

## 4. Semantic Validation Checks

- **Check 22**: `guardrails.interrupts[].tool_refs[]` must reference known tool IDs
- **Check 22b**: `guardrails.interrupts[].execution_mode` MUST NOT be set when `mode: "pause_and_notify"` (HITL interrupts are always blocking by definition)
- **Check 23**: `guardrails.cost.interrupt_ref` (when present) must reference a known `guardrails.interrupts[].id`
- **Check 30**: `guardrails.cost.downgrade_model_ref` MUST be present when `on_threshold_exceeded: "downgrade"`; it MUST reference a known `runtime.models[].id`

---

**Expert skills applied**: `role-senior-software-engineer`, `role-senior-agentic-ai-developer`

*This document is part of the ADP v0.3.0 specification. See [adp-v0.3.0.md](adp-v0.3.0.md) for the master specification.*
