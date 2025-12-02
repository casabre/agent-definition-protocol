# Agent Container Spec (ACS) v0.1.0

## Purpose

ACS describes how to build and run a container for an ADP-defined agent. It is YAML-based and intended to be portable across container builders.

## Top-level structure

```yaml
acs_version: "0.1.0"
base_image: "python:3.12-slim"

build:
  working_dir: "/app"
  copy:
    - src: "./src"
      dest: "/app/src"
  dependencies:
    python:
      - "langgraph==0.1.0"
    system:
      - "curl"

runtime:
  command: ["python", "-m", "agent_runtime"]
  args:
    - "--config"
    - "/app/agent.yaml"
  env:
    - "PYTHONUNBUFFERED=1"
  ports:
    - 8000
  healthcheck:
    path: "/health"
    interval_seconds: 30
    timeout_seconds: 3

telemetry_bindings:
  otel:
    enabled: true
    endpoint_env: "OTEL_EXPORTER_ENDPOINT"
    service_name: "my-agent"

eval_bindings:
  on_startup:
    run_suites:
      - "regression_basic"
  on_deploy:
    require_passing_suites:
      - "regression_basic"
      - "groundedness_q1"
```

## Fields

- `acs_version` (required): Spec version string.
- `base_image` (required): Container base image tag.

### build

- `working_dir`: Path inside the image to run commands.
- `copy[]`: List of copy steps with `src` (host path) and `dest` (image path).
- `dependencies`: Libraries to install.
  - `python[]`: Python packages (pip style).
  - `system[]`: System packages or apt names.

### runtime

- `command[]`: Entrypoint executable and arguments.
- `args[]`: Additional arguments appended to the command.
- `env[]`: List of `KEY=VALUE` strings.
- `ports[]`: Exposed container ports (numbers).
- `healthcheck`: Basic HTTP healthcheck definition.
  - `path`: Relative path used for probes.
  - `interval_seconds`, `timeout_seconds`: Timing configuration.

### telemetry_bindings

- `otel`: OpenTelemetry wiring.
  - `enabled` (bool): Whether OTEL is active.
  - `endpoint_env`: Name of env var containing the OTEL exporter endpoint.
  - `service_name`: Service identity for telemetry payloads.

### eval_bindings

Hooks that tie container lifecycle to evaluation.
- `on_startup.run_suites[]`: Suite ids to run when the container starts.
- `on_deploy.require_passing_suites[]`: Suites that must be passing before deploy.

## Versioning

This document defines v0.1.0 of ACS. Future versions may add richer build and runtime hooks and will provide migration notes.

## Schema

A JSON Schema for ACS is provided at `schemas/acs.schema.json`.
