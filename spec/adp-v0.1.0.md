# Agent Definition Protocol (ADP) v0.1.0

**Status**: Draft / Proposal  
**Version**: 0.1.0  
**Date**: 2024

## Purpose

Agent Definition Protocol (ADP) defines a portable manifest for AI agents. It captures runtime, capabilities, tools, memory, evaluation, governance, and deployment details so an agent can be run and promoted across environments without losing context.

**Define an agent once, run it anywhere, and keep evaluation and governance attached using your existing tools.**

## Overview

ADP treats agents like first-class software artifacts. Instead of rewriting glue code for each environment, you declare:
- **Runtime**: How the agent executes (multi-backend: Docker/WASM/Python/TypeScript/binary/custom)
- **Flow**: How work routes through the agent (AFG v0.1 graph)
- **Tools**: External capabilities (MCP servers, HTTP APIs, SQL functions)
- **Memory**: Vector store bindings
- **Evaluation**: Quality checks, KPIs, and promotion policies
- **Governance**: Data scopes, telemetry, guardrails
- **Deployment**: Environment targets

All of this is packaged using OCI (Open Container Initiative) artifacts with provenance, signing, and SBOM support.

## Conformance Classes

ADP defines two conformance classes:

- **ADP-Minimal**: Smallest valid manifest. Requires `adp_version`, `id`, `runtime.execution[]` (at least one backend), and `flow`/`evaluation` (may be empty objects). See [minimal.md](minimal.md) for details.

- **ADP-Full**: Complete manifest with all required fields across runtime, flow, evaluation, tools, governance, and packaging. Must validate against published schemas. See [conformance.md](conformance.md) for requirements.

## Top-Level Structure

```yaml
adp_version: "0.1.0"  # Required: Protocol version
id: "agent.acme.analytics"  # Required: Stable identifier (URI-like recommended)
name: "Acme Analytics Agent"  # Optional: Human-friendly name
description: "Multi-backend agent answering analytics questions"  # Optional
owner: "acme-labs"  # Optional
tags: ["analytics", "acme"]  # Optional: Free-form strings

runtime: { ... }  # Required: See runtime.md
flow: { ... }  # Required: See flow.md
evaluation: { ... }  # Required: See evaluation.md

skills: [ ... ]  # Optional: Capabilities exposed by the agent
interop: { ... }  # Optional: Interoperability mappings (A2A, etc.)
tools: { ... }  # Optional: External capabilities (MCP, HTTP, SQL)
memory: { ... }  # Optional: Vector store bindings
governance: { ... }  # Optional: Data scopes, telemetry, guardrails
deployment: { ... }  # Optional: Environment targets
```

### Required Fields

- `adp_version` (string): MUST be "0.1.0" for this version
- `id` (string): Stable identifier for the agent (URI-like recommended, e.g., "agent.acme.analytics")
- `runtime` (object): Execution configuration. See [runtime.md](runtime.md)
- `flow` (object): Flow graph definition. See [flow.md](flow.md)
- `evaluation` (object): Evaluation configuration. See [evaluation.md](evaluation.md)

### Optional Fields

- `name`, `description`, `owner`, `tags`: Human-friendly metadata
- `skills[]`: Capabilities exposed by the agent
- `interop`: Interoperability mappings (A2A AgentCard, etc.)
- `tools`: External capabilities (MCP servers, HTTP APIs, SQL functions)
- `memory`: Vector store bindings
- `governance`: Data scopes, telemetry endpoints, guardrail policies
- `deployment`: Environment targets (dev, staging, prod)

## Component Specifications

ADP is composed of several component specifications:

### Runtime

The runtime specification defines how agents execute across multiple backends. See [runtime.md](runtime.md) for details.

**Key concepts**:
- Multi-backend execution model (`runtime.execution[]` array)
- Supported backends: `docker`, `wasm`, `python`, `typescript`, `binary`, `custom`
- Source modes: repository references or inline source
- Common controls: environment variables, resource hints, logging, healthchecks

**Example**:
```yaml
runtime:
  execution:
    - backend: "python"
      id: "python-backend"
      entrypoint: "acme_agents.main:app"
      environment:
        python_version: "3.11"
```

### Flow

The flow specification (AFG v0.1) defines how work routes through the agent as a directed graph. See [flow.md](flow.md) for details.

**Key concepts**:
- Directed graph with nodes and edges
- Node types: `input`, `output`, `llm`, `tool`, `router`, `retriever`, `evaluator`, `subflow`
- UI metadata for visualization
- Vendor extensions

**Example**:
```yaml
flow:
  id: "acme.analytics.flow"
  graph:
    nodes:
      - id: "input"
        kind: "input"
      - id: "planner"
        kind: "llm"
    edges:
      - { from: "input", to: "planner" }
    start_nodes: ["input"]
    end_nodes: ["planner"]
```

### Evaluation

The evaluation specification defines quality checks, KPIs, and promotion policies. See [evaluation.md](evaluation.md) for details.

**Key concepts**:
- Evaluator types: `deterministic`, `llm_judge`, `telemetry`
- Suites: Collections of metrics with scoring
- KPIs: High-level metrics with targets
- Promotion policies: Rules for moving to higher environments

**Example**:
```yaml
evaluation:
  suites:
    - id: "groundedness"
      metrics:
        - id: "groundedness.score"
          type: "llm_judge"
          threshold: 4.0
  promotion_policy:
    require_passing_suites: ["groundedness"]
```

### Tools

Tools define external capabilities the agent can call:

- **MCP servers**: Model Context Protocol servers (`tools.mcp_servers[]`)
- **HTTP APIs**: REST/HTTP endpoints (`tools.http_apis[]`)
- **SQL functions**: Database routines (`tools.sql_functions[]`)

### Memory

Memory defines vector store bindings for retrieval or context injection:

```yaml
memory:
  provider: "pinecone"  # or weaviate, pgvector, etc.
  endpoint: "https://..."
  index: "agent-index"
  namespace: "acme"
```

### Governance

Governance controls identity, data access, telemetry, and guardrails:

```yaml
governance:
  identity: "service-principal://..."
  allowed_data_scopes: ["public", "analytics"]
  forbidden_data_scopes: ["pii"]
  telemetry_endpoint: "https://..."
  guardrail_policy_set: "policies/acme-guardrails.yaml"
```

### Deployment

Deployment defines environment targets:

```yaml
deployment:
  environments:
    - name: "dev"
      endpoint: "https://dev.acme.ai/agent"
    - name: "prod"
      endpoint: "https://api.acme.ai/agent"
```

## Packaging (ADPKG)

ADP agents are packaged using OCI (Open Container Initiative) artifacts. See [adpkg-oci.md](adpkg-oci.md) for details.

**Key concepts**:
- OCI image layout structure
- Media types: `application/vnd.adp.config.v1+json`, `application/vnd.adp.package.v1+tar`
- Provenance: Notary v2 signatures, SBOM (SPDX JSON)
- Build metadata: Source repo, ref, builder identity, timestamp

## Interoperability

ADP is designed to work with existing protocols:

- **MCP**: Model Context Protocol for tool integration
- **OCI**: Open Container Initiative for packaging
- **A2A**: Agent-to-Agent protocol mapping (see [integrations/a2a-mapping.md](integrations/a2a-mapping.md))
- **OpenTelemetry**: Telemetry integration (see [interop-mapping.md](interop-mapping.md))

See [interop-mapping.md](interop-mapping.md) for detailed mappings.

## Versioning and Compatibility

ADP follows SemVer:
- **Patch versions** (0.1.0 → 0.1.1): MUST remain backward-compatible
- **Minor versions** (0.1.0 → 0.2.0): MAY add optional fields but MUST NOT break existing required fields
- **Major versions** (0.1.0 → 1.0.0): MAY introduce breaking changes

See [compatibility.md](compatibility.md) for detailed compatibility policy and field stability guarantees.

## Validation

Validate ADP manifests against JSON Schemas:

```bash
./scripts/validate.sh
```

Schemas are located in [`schemas/`](../schemas/):
- `adp.schema.json` - Main ADP schema
- `runtime.schema.json` - Runtime schema
- `flow.schema.json` - Flow schema
- `evaluation.schema.json` - Evaluation schema

SDKs (Python, TypeScript, Rust, Go) provide programmatic validation.

## Examples

- **Minimal**: [`examples/minimal/acme-minimal.yaml`](../examples/minimal/acme-minimal.yaml)
- **Full**: [`examples/adp/acme-full-agent.yaml`](../examples/adp/acme-full-agent.yaml)
- **Runtime**: [`examples/runtime/acme-runtime-example.yaml`](../examples/runtime/acme-runtime-example.yaml)
- **Flow**: [`examples/flow/acme-flow-example.yaml`](../examples/flow/acme-flow-example.yaml)
- **Evaluation**: [`examples/evaluation/acme-eval-suite.yaml`](../examples/evaluation/acme-eval-suite.yaml)

## Conformance

See [conformance.md](conformance.md) for:
- Conformance classes (Minimal vs Full)
- Required behaviors
- Test fixtures
- Reporting requirements

## Future Features

The following features are planned for future releases:
- **Composition**: `extends`, `import`, `overrides` for manifest composition
- **Advanced governance**: Formal guardrail policy schema
- **Enhanced provenance**: Richer build metadata and signing options

See [`roadmap.md`](../roadmap.md) for details.

## References

- [Runtime Specification](runtime.md)
- [Flow Specification](flow.md)
- [Evaluation Specification](evaluation.md)
- [Minimal Specification](minimal.md)
- [ADPKG over OCI](adpkg-oci.md)
- [Conformance Program](conformance.md)
- [Compatibility Policy](compatibility.md)
- [Interoperability Mapping](interop-mapping.md)
- [Execution Semantics Profile (ESP)](esp.md) (v0.2.0+)

