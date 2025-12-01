# ADP Runtime Specification v0.2.0

Runtimes describe how agents execute across multiple backends. ADP v0.2.0 formalizes `runtime.execution` as an ordered list of backends that can mix Docker, WASM, Python, TypeScript, binaries, and custom/external endpoints.

## Execution model
- **Multi-backend**: `runtime.execution[]` allows composing multiple runtimes in one agent definition.
- **Source modes**: Backends can reference repos (url/path/ref) or inline source (Python/TS) for quick iteration.
- **Common controls**: Environment variables, resource hints, logging, and health checks apply uniformly.

## Supported backend types
- `docker`: OCI image reference with optional entrypoint/command/ports.
- `wasm`: WASM module path/URL with exported functions and optional WASI.
- `python`: Module entrypoint, repo/inline source, interpreter version, dependencies.
- `typescript`: Node/TS package with repo source, package manager, build command, entrypoint.
- `binary`: Executable path/URL with args.
- `custom`: External runtime described by `type` and `endpoint`.

## Fields (per execution entry)
- `backend` (required): Backend kind (`docker`, `wasm`, `python`, `typescript`, `binary`, `custom`).
- `id` (required): Stable backend identifier.
- `entrypoint`: Command or module entrypoint.
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
- `ports`: Port mappings (strings like `8080:8080`).
- `extensions`: Vendor-specific extensions under `extensions.*`.

## ACME runtime example
See `examples/runtime/acme-runtime-example.yaml` for a composite runtime with docker, wasm, python, typescript, binary, and custom backends.
