# ADP Runtime Specification v0.1.0

Runtimes describe how agents execute across multiple backends. ADP v0.1.0 formalizes `runtime.execution` as an ordered list of backends that can mix Docker, WASM, Python, TypeScript, binaries, and custom/external endpoints.

## Conformance and terminology
- **Normative language**: MUST/SHOULD/MAY follow RFC 2119. Items marked “required” are normative MUSTs for ADP-Full. ADP-Minimal MAY omit optional fields but MUST include at least one execution entry.
- **Conformance classes**:
  - **ADP-Full**: MUST supply all required fields per backend type, healthcheck/logging/resource hints where applicable, and use valid OCI image references for docker.
  - **ADP-Minimal**: MUST include `runtime.execution` with at least one entry containing `backend` and `id`; other fields MAY be omitted.

## Execution model
- **Multi-backend**: `runtime.execution[]` allows composing multiple runtimes in one agent definition.
- **Source modes**: Backends can reference repos (url/path/ref) or inline source (Python/TS) for quick iteration.
- **Common controls**: Environment variables, resource hints, logging, and health checks apply uniformly.

## Supported backend types (required fields by type)
- `docker`: `image` (OCI ref), optional `entrypoint` (array), `ports` (map or list), `env`, `healthcheck`.
- `wasm`: `module` (path/URL), optional `exported_functions[]`, `wasi` (bool), `memory.max_mb`.
- `python`: `entrypoint` (module:function), `source` (repo/inline), `environment.python_version`, `dependencies[]`.
- `typescript`: `entrypoint` (built JS), `source` (repo/inline), `package_manager`, `build_cmd`, optional `dependencies`.
- `binary`: `path` (exe), optional `args[]`.
- `custom`: `type` (e.g., `external-http`), `endpoint` URL.

## Fields (per execution entry)
- `backend` (required): `docker` | `wasm` | `python` | `typescript` | `binary` | `custom`.
- `id` (required): Stable backend identifier.
- `entrypoint`: Command or module entrypoint (array or string as appropriate).
- `image`: OCI image reference (docker).
- `module`: WASM module path/URL; `exported_functions[]`; `wasi` (bool); `memory.max_mb`.
- `source`: `{ mode: repo|inline, repo, path, ref, inline }` depending on backend.
- `environment`: Language/runtime hints (e.g., `python_version`).
- `dependencies`: Language-specific dependency list.
- `package_manager`, `build_cmd`: For TS/Node.
- `path`, `args`: For binaries.
- `type`, `endpoint`: For custom runtimes.
- `env`: Map of environment variables.
- `resources`: `{ cpu, memory }` hints.
- `logging`: `{ level, destination }`.
- `healthcheck`: `{ path | command, interval_seconds, timeout_seconds }`.
- `ports`: Port mappings (strings like `8080:8080`) or list of ints (for docker).
- `extensions`: Vendor-specific extensions under `extensions.*`.

## Models (v0.2.0+)

`runtime.models[]` (optional): Array of model configurations for LLM nodes. Each model includes:
- `id` (required): Model reference identifier used by `flow.graph.nodes[].model_ref`
- `provider` (required): Provider name (e.g., "openai", "anthropic", "custom")
- `model` (required): Provider-specific model identifier
- `api_key_env` (optional): Environment variable name for API key
- `base_url` (optional): Provider-specific base URL
- `temperature` (optional): Default temperature parameter
- `max_tokens` (optional): Default max tokens parameter
- `extensions` (optional): Provider-specific extensions

Example:
```yaml
runtime:
  models:
    - id: "primary"
      provider: "openai"
      model: "gpt-4"
      api_key_env: "OPENAI_API_KEY"
```

See [ESP Specification](esp.md) for model resolution semantics.

## ACME runtime example
See `examples/runtime/acme-runtime-example.yaml` for a composite runtime with docker, wasm, python, typescript, binary, and custom backends.

---

## Model Parameters (v0.3.0)

Eight new optional fields on each `runtime.models[]` entry:

| Field | Type | Constraint | Provider mapping |
|---|---|---|---|
| `top_p` | number | 0.0–1.0 | OpenAI `top_p` · Anthropic `top_p` |
| `seed` | integer | — | OpenAI `seed` (exact) · Anthropic best-effort |
| `timeout_ms` | integer | ≥ 1 | Runner-enforced per-call timeout; not sent to provider |
| `use_streaming_api` | boolean | — | OpenAI `stream: true` · Anthropic `stream: true` (internal; see below) |
| `stop_sequences` | array[string] | max 4 items | OpenAI `stop` · Anthropic `stop_sequences` |
| `frequency_penalty` | number | −2.0–2.0 | OpenAI `frequency_penalty` · not supported by Anthropic |
| `presence_penalty` | number | −2.0–2.0 | OpenAI `presence_penalty` · not supported by Anthropic |
| `structured_output` | object | — | OpenAI `response_format` · Anthropic tool_use schema |

`structured_output` fields:
- `format`: `"json_object"` \| `"json_schema"` \| `"text"`
- `schema`: Inline JSON Schema object (used when `format: "json_schema"`)
- `schema_ref`: Path or URI to a JSON Schema file (alternative to `schema`)

### `use_streaming_api` vs `streaming.enabled` — precedence rule

These are two distinct concepts:

- `model.use_streaming_api: true` — calls the LLM provider API in streaming mode **internally**. The runner may buffer tokens before returning to the caller. This is a model-level implementation detail.
- `streaming.enabled: true` (top-level) — the agent **exposes** streaming to its callers. This is an agent-level interface contract.

They are independent: `use_streaming_api: true` + `streaming.enabled: false` is valid — the runner calls OpenAI in streaming mode internally but returns a buffered complete response to callers.

**Hard rule**: `streaming.enabled: false` MUST be respected. Runners MUST NOT stream to callers even if `use_streaming_api: true` is declared on the model.

## Adapter Hints (v0.3.0)

`runtime.adapter_hints` (optional object) is the framework-specific configuration escape hatch. Keys are framework adapter names; values are open objects validated by each adapter's own schema.

Known framework keys with typed sub-schemas:
- `langgraph`: `recursion_limit` (int), `stream_mode` (`"values"|"updates"|"debug"`), `checkpointer` (`"memory"|"sqlite"|"postgres"|"none"`)
- `autogen`: `max_turns` (int), `human_input_mode` (`"NEVER"|"TERMINATE"|"ALWAYS"`)
- `crewai`: `process` (`"sequential"|"hierarchical"|"parallel"`), `verbose` (bool), `memory` (bool)
- `semantic_kernel`: `execution_type` (`"sequential"|"stepwise"`)

Runners ignore keys for unsupported frameworks. Unknown keys pass through. See [`spec/adp-v0.3.0-pipeline.md`](adp-v0.3.0-pipeline.md) for the complete adapter hints specification.
