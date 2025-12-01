# Agent Definition Protocol (ADP) v0.1

## Purpose

ADP defines a portable manifest for AI agents. It captures runtime, capabilities, tools, memory, evaluation, governance, and deployment details so an agent can be run and promoted across environments without losing context.

## Top-level structure

```yaml
adp_version: "0.1"
id: "string"
name: "string"
description: "string"
owner: "string"
tags: ["string", ...]
runtime: { ... }
skills: [ ... ]
interop: { ... }
tools: { ... }
memory: { ... }
evaluation: { ... }
governance: { ... }
deployment: { ... }
```

- `adp_version` (required): Protocol version string.
- `id` (required): Stable identifier for the agent (URI-like recommended).
- `name`, `description`, `owner`: Human-friendly metadata.
- `tags`: Free-form strings for discovery.

## Runtime

Describes how the agent executes.

Fields:
- `framework`: Agent framework or orchestration runtime (e.g., langgraph, semantic-kernel).
- `entrypoint`: Import path or command to start the agent (e.g., `pkg.module:app`).
- `language`: Implementation language.
- `models[]`: Model bindings used by the agent.
  - Each model: `provider` (e.g., openai, anthropic), `name`, optional `purpose` (generation, embedding), optional `endpoint`.
- `image` (optional): OCI image reference for the runnable agent.
  - `registry`, `repository`, `tag`, `digest`, and optional `annotations` (map). Use `digest` when available for immutability.

## Skills

Capabilities exposed by the agent.

Each item in `skills[]`:
- `id` (required): Stable capability id.
- `name`, `description` (required): What the skill does.
- `tags[]`: Optional classification.
- `examples[]`: Sample utterances or invocations.
- `input_modes[]`, `output_modes[]`: e.g., `text`, `json`, `audio`.
- `handler` (optional): Reference to code function/route implementing the skill.

## Interop

Defines how the agent maps to other agent specifications.

- `a2a`: Bridge to the A2A AgentCard concept. Either:
  - `inline`: Embed an AgentCard-like object (schema deferred to future version), or
  - `ref`: File path or URL to an AgentCard document.
The intent is to provide future compatibility without prescribing the entire A2A schema here.

## Tools

External capabilities the agent can call.

Structure:
- `mcp_servers[]`: MCP server entries with `id`, `description`, `transport` (e.g., `stdio`, `sse`), and `endpoint`.
- `http_apis[]`: HTTP APIs with `id`, `description`, `base_url`, optional `auth` description.
- `sql_functions[]`: SQL routines with `id`, `description`, `connection` (DSN or host/db), and optional `schema` reference.

## Memory

Defines vector-store backing for retrieval or context injection.

Fields:
- `provider` (e.g., pinecone, weaviate, pgvector).
- `endpoint`: URL or connection string.
- `index`: Primary index/collection name.
- `namespace`: Logical namespace or tenant.

## Evaluation

Evaluation configuration and promotion policy.

Fields:
- `suites[]`: Each suite has `id`, `runner` (e.g., `pytest`, `evals`, `custom`), `config_path` (path within repo/package), `metrics` (list of metric names), and optional `thresholds` (map metric->minimum value).
- `kpis[]` (optional): High-level KPIs to monitor, each with `id`, `name`, `description`, `url` (details/dashboard), optional `target`, `unit`.
- `promotion_policy`: Rules for moving to higher environments.
  - `require_passing_suites[]`: List of suite ids that must pass before promotion/deploy.

## Governance

Controls for identity, data, telemetry, and guardrails.

Fields:
- `identity`: Service principal or workload identity representing the agent.
- `allowed_data_scopes[]`: Data domains/labels the agent may access.
- `forbidden_data_scopes[]`: Explicitly disallowed scopes.
- `telemetry_endpoint`: Endpoint for traces/logs/metrics.
- `guardrail_policy_set`: Reference to a policy bundle or document.

## Deployment

Declarative deployment targets.

Fields:
- `environments[]`: Each item has `name` (e.g., `dev`, `prod`) and `endpoint` (URL/base path where the agent is reachable after deploy).

## Versioning and evolution

- This document defines v0.1 of ADP.
- Backward-incompatible changes will increment the minor/major version and be accompanied by schema updates.
