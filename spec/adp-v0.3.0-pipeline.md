# ADP Execution & Observation Harness (v0.3.0)

**Status: RFC/Preview — normative target v0.3.0.**

This document specifies the `pipeline`, `hooks`, `streaming`, and `runtime.adapter_hints` fields introduced in ADP v0.3.0. Together they form the **execution harness** and **observation harness** layers of ADP.

---

## Agent Harness Context

ADP wraps an agent across four layers:

| Layer | ADP fields | Role |
|---|---|---|
| **Execution harness** | `runtime`, `pipeline`, `streaming` | How the agent runs and processes I/O |
| **Observation harness** | `telemetry`, `hooks` | What can be seen during execution |
| Safety harness | `guardrails` | What is permitted to pass through |
| Testing harness | `x_testing` | How the agent is tested |

This document covers the execution and observation layers.

---

## Execution Order

When all sections are present, execution follows this order:

```
guardrails.input
  → pipeline.pre_process[]
    → flow graph (nodes + edges)
  → pipeline.post_process[]
→ guardrails.output
```

Hooks fire **out-of-band** at any point in this sequence based on their declared `event`. They observe but do not modify the data flow.

---

## Pipeline Stages

### Structure

```yaml
pipeline:
  pre_process:
    - id: "normalize-input"
      type: "function"
      function_ref: "acme.transforms:normalize_query"
      on_error: "fail"

  post_process:
    - id: "format-output"
      type: "script"
      runtime: "python"
      inline: |
        def process(data):
            return {"result": data.get("content", "").strip(), "version": "1.0"}
      on_error: "warn"
```

Pre-process stages run after `guardrails.input`, before the first flow node. Post-process stages run after the last flow node, before `guardrails.output`. Each stage receives the output of the previous stage.

### Stage Types

| Type | Required fields | Contract |
|---|---|---|
| `function` | `function_ref` | `f(input: dict, context: dict) → dict` |
| `script` | `runtime` + `inline` or `script_ref` | Script entrypoint: `def process(data: dict) -> dict` |
| `json_schema` | `schema` or `schema_ref` | Validation only — does NOT transform |

**Function/script stages**: receive `{input: dict, context: dict}`, return an updated dict. A `None`/missing return is treated as no-op (pass through unchanged).

**`json_schema` stages**: validate the current payload against the schema. Fail on schema violation. Do not transform.

**Chaining**: each stage receives the output of the previous one. Pre-process stages operate on the invocation input; post-process stages operate on the agent output.

### Error Handling

```yaml
on_error: "fail"    # default — halt execution; propagate error to caller
on_error: "warn"    # log warning; continue with unchanged payload
on_error: "skip"    # skip silently; continue with unchanged payload
```

---

## Hooks

Hooks are **out-of-band observers** — fire-and-forget handlers that observe the execution lifecycle. They do not modify the data flow. Their return values are ignored.

### Structure

```yaml
hooks:
  - event: "on_node_end"
    node_filter: ["chat-llm", "synthesizer"]  # limit to these node IDs
    handler:
      type: "function"
      function_ref: "acme.observability:record_llm_call"
    on_error: "log"

  - event: "on_error"
    handler:
      type: "script"
      runtime: "python"
      inline: |
        def handle(payload):
            print(f"Error in {payload.get('phase')}: {payload.get('error')}")
    on_error: "skip"
```

### Event Types

| Event | When it fires | Payload |
|---|---|---|
| `on_invoke_start` | Before first flow node | `{input: dict}` |
| `on_invoke_end` | After last flow node | `{input: dict, output: dict, duration_ms: int}` |
| `on_node_start` | Before each node executes | `{node_id: str, kind: str, state: dict}` |
| `on_node_end` | After each node executes | `{node_id: str, kind: str, state: dict, output: dict, duration_ms: int}` |
| `on_stream_start` | When streaming begins | `{node_id: str}` |
| `on_stream_chunk` | Per chunk (mode=token) | `{node_id: str, chunk: str, index: int}` |
| `on_stream_end` | When streaming completes | `{node_id: str, total_chunks: int}` |
| `on_error` | On any execution error | `{error: str, node_id: str?, phase: str}` |

### `node_filter`

`node_filter` limits a hook to specific node IDs. Only applicable to `on_node_start` / `on_node_end`. If absent, the hook fires for all nodes.

**Semantic validation (check 12)**: entries in `node_filter` MUST reference known `flow.graph.nodes[].id`. Validators emit an error for unknown node IDs.

### Handler Types

| Type | Required fields |
|---|---|
| `function` | `function_ref` (module:function) |
| `script` | `runtime` + `inline` or `script_ref` |

### Hook Error Handling

```yaml
on_error: "log"    # default — log and continue
on_error: "fail"   # propagate error; halt execution
on_error: "skip"   # suppress silently
```

---

## Streaming Policy

The `streaming` section declares the agent's external streaming contract. ADP declares **intent**; SDK integrations translate to framework-specific APIs.

```yaml
streaming:
  enabled: true
  mode: "token"                  # "token" | "message" | "event" | "none"
  chunk_format: "server_sent_events"  # "text" | "json" | "server_sent_events"
  buffer_lines: 0                # 0 = unbuffered (true streaming)
  include_node_events: false     # include on_node_start/end in stream
```

### Fields

| Field | Default | Description |
|---|---|---|
| `enabled` | false | Whether this agent supports streaming invocation |
| `mode` | `"token"` | Streaming granularity |
| `chunk_format` | `"text"` | Wire format for stream chunks |
| `buffer_lines` | 0 | 0 = unbuffered; N > 0 = buffer N lines before emitting |
| `include_node_events` | false | Interleave `{type: "node_start"/"node_end"}` events with content |

### Streaming Modes

| `mode` | Description | LangGraph | AutoGen | CrewAI | SK |
|---|---|---|---|---|---|
| `token` | Individual tokens | `graph.stream(mode="updates")` | `on_messages()` stream | `Flow.kickoff()` yield | `kernel.invoke_stream()` |
| `message` | Complete node outputs | `graph.stream(mode="values")` | per-agent message events | `@listen` returns | `process_step.invoke()` |
| `event` | Structured lifecycle events | LangGraph event stream | AutoGen event bus | — | SK `ProcessEvent` |
| `none` | Disable streaming | synchronous | synchronous | synchronous | synchronous |

### `streaming.enabled: false` — Hard Gate

A runner MUST NOT stream to a caller when `streaming.enabled: false`, even if:
- The underlying framework supports streaming
- `runtime.models[].use_streaming_api: true` is declared

`use_streaming_api` is an **internal LLM API mode** — it controls whether the runner calls the LLM provider in streaming mode. `streaming.enabled` is the **external agent contract** — it controls what the agent exposes to callers. They are independent.

---

## Adapter Hints

`runtime.adapter_hints` is the structured escape hatch for framework-specific configuration that cannot be standardized in ADP.

```yaml
runtime:
  adapter_hints:
    langgraph:
      recursion_limit: 25
      stream_mode: "updates"
      checkpointer: "memory"
    autogen:
      max_turns: 10
      human_input_mode: "NEVER"
    crewai:
      process: "sequential"
      verbose: false
      memory: false
    semantic_kernel:
      execution_type: "sequential"
```

### Known Framework Keys

| Key | Fields | Description |
|---|---|---|
| `langgraph` | `recursion_limit` (int), `stream_mode` (values/updates/debug), `checkpointer` (memory/sqlite/postgres/none) | LangGraph `StateGraph.compile()` options |
| `autogen` | `max_turns` (int), `human_input_mode` (NEVER/TERMINATE/ALWAYS) | AutoGen team constructor options |
| `crewai` | `process` (sequential/hierarchical/parallel), `verbose` (bool), `memory` (bool) | CrewAI `Flow` constructor options |
| `semantic_kernel` | `execution_type` (sequential/stepwise) | SK kernel construction options |

Runners MUST ignore keys for unsupported frameworks. Unknown keys at root level pass through to adapters that understand them. `additionalProperties: true` at root level preserves forward compatibility.

### Framework Integration Pattern

Each SDK integration reads `runtime.adapter_hints.<framework>` and applies configuration:

```python
# Python SDK pattern
hints = manifest.get("runtime", {}).get("adapter_hints", {})
lg_hints = hints.get("langgraph", {})
graph.compile(
    recursion_limit=lg_hints.get("recursion_limit", 25),
    checkpointer=resolve_checkpointer(lg_hints.get("checkpointer", "none")),
)
```

---

## Full Example

```yaml
adp_version: "0.3.0"
id: "acme.pipeline-agent"

runtime:
  execution:
    - backend: "python"
      id: "main-backend"
      entrypoint: "acme.main:app"
  models:
    - id: "gpt4"
      provider: "openai"
      model: "gpt-4o"
      seed: 42
      use_streaming_api: true
  adapter_hints:
    langgraph:
      recursion_limit: 25
      stream_mode: "updates"
      checkpointer: "memory"

pipeline:
  pre_process:
    - id: "normalize-input"
      type: "function"
      function_ref: "acme.transforms:normalize_query"
      on_error: "fail"
  post_process:
    - id: "format-output"
      type: "script"
      runtime: "python"
      inline: |
        def process(data):
            return {"result": data.get("content", "").strip()}
      on_error: "warn"

hooks:
  - event: "on_node_end"
    node_filter: ["llm-node"]
    handler:
      type: "function"
      function_ref: "acme.observability:record_llm_call"
    on_error: "log"
  - event: "on_error"
    handler:
      type: "function"
      function_ref: "acme.alerting:notify"

streaming:
  enabled: true
  mode: "token"
  chunk_format: "server_sent_events"
  include_node_events: false
```
