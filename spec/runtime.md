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
