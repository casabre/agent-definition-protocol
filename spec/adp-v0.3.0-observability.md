# ADP v0.3.0 Observability Specification

**Agent Definition Protocol — Declarative Tracing v0.3.0**

> **Version**: v0.3.0  
> **Status**: Draft  
> **Editor**: ADP Working Group  
> **Last Updated**: 2026-05-25  

---

## Abstract

This document formalizes a declarative **Observability** layer: manifests declare which tracing backend to use and what events to emit. All 6 researched frameworks (AutoGen, Semantic Kernel, CrewAI, LlamaIndex, Google ADK, OpenAI Agents SDK) expose OpenTelemetry-compatible tracing. ADP v0.2.0 had no way to declare this — runners had to be configured out-of-band.

---

## Table of Contents

1. [Top-Level `observability` Section](#1-top-level-observability-section)
2. [OTel Semantic Conventions](#2-otel-semantic-conventions)
3. [Relationship to `guardrails.cost`](#3-relationship-to-guardrailscost)
4. [Semantic Validation Checks](#4-semantic-validation-checks)

---

## 1. Top-Level `observability` Section

```yaml
observability:
  tracing:
    backend: "otlp"                # otlp | langfuse | langsmith | arize | phoenix | stdout | none
    endpoint_env_var: "OTEL_EXPORTER_OTLP_ENDPOINT"  # env var holding the collector endpoint
    api_key_env_var: "OTEL_API_KEY"                  # env var holding auth key (absent = unauthenticated)
    trace_events:
      - "model_request"            # LLM call start/end + token counts
      - "tool_call"                # tool invocation start/end + result
      - "flow_node"                # each AFG node start/end
      - "loop_iteration"           # each loop body execution
      - "interrupt"                # HITL pause + resume
      - "cost_check"               # guardrails.cost threshold evaluation
      - "artifact_write"           # artifact version write
    sampling_rate: 1.0             # 0.0-1.0; 1.0 = trace every run; 0.1 = sample 10%
    service_name: "my-agent"       # OTel service.name attribute; defaults to manifest id

  cost_reporting:
    enabled: true
    track_by: "invocation"         # invocation | session | user | day
    emit_metric: "gen_ai.cost.usd" # OTel metric name for cost
    model_refs: ["primary"]        # absent = all models
```

---

## 2. OTel Semantic Conventions

Runners implementing `ADP-Harness-Observability` MUST follow the [OpenTelemetry Generative AI semantic conventions](https://opentelemetry.io/docs/specs/semconv/gen-ai/) for span/metric names:

- `gen_ai.client.operation.name` for LLM calls
- `gen_ai.usage.input_tokens` / `gen_ai.usage.output_tokens` for token usage
- `gen_ai.tool.name` / `gen_ai.tool.call.id` for tool calls

ADP extends these with agent-specific attributes:
- `adp.agent.id` — manifest `id`
- `adp.node.id` / `adp.node.kind` — flow node
- `adp.loop.id` / `adp.loop.iteration` — loop iteration
- `adp.interrupt.id` — HITL interrupt event

---

## 3. Relationship to `guardrails.cost`

| | `observability.cost_reporting` | `guardrails.cost` |
|---|---|---|
| **Type** | Passive metric emission | Active enforcement |
| **Effect** | Emits OTel metric; no side effect | Blocks / warns / interrupts / downgrades |
| **Threshold** | None (reports all spend) | `threshold_usd` triggers action |
| **Can coexist** | Yes | Yes |

Both can coexist on the same manifest. `observability.cost_reporting` emits metrics for monitoring, while `guardrails.cost` enforces limits.

---

## 4. Semantic Validation Checks

- **Check 35**: `observability.tracing.trace_events[]` entries must be from the valid enum: `model_request | tool_call | flow_node | loop_iteration | interrupt | cost_check | artifact_write`
- **Check 35b**: `observability.cost_reporting.model_refs[]` (when present) must reference known `runtime.models[].id` values

---

**Expert skills applied**: `role-senior-software-engineer`, `role-senior-agentic-ai-developer`

*This document is part of the ADP v0.3.0 specification. See [adp-v0.3.0.md](adp-v0.3.0.md) for the master specification.*
