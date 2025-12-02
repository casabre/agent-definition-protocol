# A2A AgentCard Mapping

This document provides detailed field-by-field mapping between ADP manifests and the A2A AgentCard format. The `interop.a2a` block in ADP enables round-trip compatibility with A2A AgentCard documents.

## Status

**Status**: Informative (non-normative)  
**A2A Status**: Draft specification  
**Last Updated**: 2024

This mapping is provided for interoperability purposes. ADP implementations SHOULD support `interop.a2a` blocks but are not required to validate A2A AgentCard schemas.

## Overview

ADP can embed or reference A2A AgentCards in two ways:

1. **Inline embedding**: `interop.a2a.agentcard` contains a full AgentCard object
2. **Reference**: `interop.a2a.ref` contains a file path or URL to an AgentCard document

```yaml
interop:
  a2a:
    # Option 1: Inline AgentCard
    agentcard:
      id: "agent.acme.analytics"
      name: "Acme Analytics Agent"
      # ... full AgentCard fields
    
    # Option 2: Reference to external AgentCard
    ref: "./a2a-agentcard.yaml"
    # or
    ref: "https://example.com/agentcards/acme-analytics.yaml"
```

## Field-by-Field Mapping

### Identity and Metadata

| ADP Field         | A2A AgentCard Field | Notes                              |
| ----------------- | ------------------- | ---------------------------------- |
| `adp.id`          | `id`                | Direct mapping; SHOULD match       |
| `adp.name`        | `name`              | Direct mapping                     |
| `adp.description` | `description`       | Direct mapping                     |
| `adp.owner`       | `owner`             | Direct mapping (if present in A2A) |
| `adp.tags[]`      | `tags[]`            | Direct mapping                     |

### Runtime

| ADP Field                        | A2A AgentCard Field    | Notes                                                  |
| -------------------------------- | ---------------------- | ------------------------------------------------------ |
| `runtime.execution[].backend`    | `execution.runtime`    | A2A may use single runtime; ADP supports multi-backend |
| `runtime.execution[].image`      | `execution.image`      | OCI image reference                                    |
| `runtime.execution[].entrypoint` | `execution.entrypoint` | Command or module entrypoint                           |
| `runtime.execution[].env`        | `execution.env`        | Environment variables                                  |

**Note**: ADP's multi-backend `execution[]` array maps to A2A's single `execution` object. When converting ADP → A2A, use the first execution entry or a composite representation.

### Tools

| ADP Field               | A2A AgentCard Field | Notes                  |
| ----------------------- | ------------------- | ---------------------- |
| `tools.mcp_servers[]`   | `tools.mcp[]`       | MCP server definitions |
| `tools.http_apis[]`     | `tools.http[]`      | HTTP API connectors    |
| `tools.sql_functions[]` | `tools.sql[]`       | SQL function bindings  |

### Flow

| ADP Field                  | A2A AgentCard Field     | Notes                               |
| -------------------------- | ----------------------- | ----------------------------------- |
| `flow.id`                  | `flow.id`               | Flow identifier                     |
| `flow.graph.nodes[]`       | `flow.graph.nodes[]`    | Node definitions                    |
| `flow.graph.edges[]`       | `flow.graph.edges[]`    | Edge definitions                    |
| `flow.graph.start_nodes[]` | `flow.graph.start[]`    | Start node IDs                      |
| `flow.graph.end_nodes[]`   | `flow.graph.end[]`      | End node IDs                        |
| `flow.graph.nodes[].ui`    | `flow.graph.nodes[].ui` | UI metadata (label, icon, position) |

### Evaluation

| ADP Field                       | A2A AgentCard Field             | Notes                        |
| ------------------------------- | ------------------------------- | ---------------------------- |
| `evaluation.suites[]`           | `evaluation.suites[]`           | Evaluation suite definitions |
| `evaluation.suites[].id`        | `evaluation.suites[].id`        | Suite identifier             |
| `evaluation.suites[].metrics[]` | `evaluation.suites[].metrics[]` | Metric definitions           |
| `evaluation.kpis[]`             | `evaluation.kpis[]`             | High-level KPIs              |

### Governance

| ADP Field                            | A2A AgentCard Field                  | Notes                                  |
| ------------------------------------ | ------------------------------------ | -------------------------------------- |
| `governance.identity`                | `governance.identity`                | Service principal or workload identity |
| `governance.allowed_data_scopes[]`   | `governance.data_scopes.allowed[]`   | Allowed data domains                   |
| `governance.forbidden_data_scopes[]` | `governance.data_scopes.forbidden[]` | Forbidden data domains                 |
| `governance.telemetry_endpoint`      | `governance.telemetry.endpoint`      | Telemetry endpoint URL                 |
| `governance.guardrail_policy_set`    | `governance.policies[]`              | Policy bundle references               |

## Round-Trip Examples

### ADP → A2A Conversion

**ADP Manifest**:
```yaml
adp_version: "0.1.0"
id: "agent.acme.analytics"
name: "Acme Analytics Agent"
runtime:
  execution:
    - backend: "python"
      id: "python-backend"
      entrypoint: "acme_agents.main:app"
flow:
  id: "acme.flow"
  graph:
    nodes:
      - id: "input"
        kind: "input"
    edges: []
    start_nodes: ["input"]
    end_nodes: ["input"]
```

**Equivalent A2A AgentCard**:
```yaml
id: "agent.acme.analytics"
name: "Acme Analytics Agent"
execution:
  runtime: "python"
  entrypoint: "acme_agents.main:app"
flow:
  id: "acme.flow"
  graph:
    nodes:
      - id: "input"
        kind: "input"
    edges: []
    start: ["input"]
    end: ["input"]
```

### A2A → ADP Conversion

**A2A AgentCard**:
```yaml
id: "agent.example"
name: "Example Agent"
execution:
  runtime: "docker"
  image: "example/agent:1.0.0"
```

**Equivalent ADP Manifest**:
```yaml
adp_version: "0.1.0"
id: "agent.example"
name: "Example Agent"
runtime:
  execution:
    - backend: "docker"
      id: "docker-backend"
      image: "example/agent:1.0.0"
flow: {}
evaluation: {}
```

## Validation Guidance

### ADP → A2A

When converting ADP to A2A AgentCard:

1. **Required fields**: Ensure all A2A required fields are present
2. **Multi-backend**: If ADP has multiple `runtime.execution[]` entries, choose the primary backend or create a composite representation
3. **Empty objects**: ADP allows `flow: {}` and `evaluation: {}`; A2A may require explicit structures

### A2A → ADP

When converting A2A AgentCard to ADP:

1. **Required fields**: ADP requires `adp_version`, `id`, `runtime`, `flow`, `evaluation`
2. **Empty objects**: Use `flow: {}` and `evaluation: {}` if A2A doesn't define them
3. **Runtime mapping**: Map A2A's single `execution` to ADP's `runtime.execution[]` array

## Implementation Notes

### ADP Implementations

- **SHOULD** accept `interop.a2a.ref` and resolve referenced AgentCards
- **SHOULD** accept `interop.a2a.agentcard` inline AgentCard objects
- **MAY** validate AgentCard structure but are not required to
- **SHOULD** preserve `interop.a2a` blocks when processing ADP manifests

### A2A Implementations

- **MAY** read ADP manifests if they include `interop.a2a.agentcard`
- **SHOULD** extract AgentCard from `interop.a2a.agentcard` or resolve `interop.a2a.ref`
- **MAY** ignore ADP-specific fields outside the `interop.a2a` block

## Limitations

1. **Multi-backend**: ADP supports multiple runtime backends; A2A may only support one
2. **Composition**: ADP's future composition features (`extends`, `import`) may not map to A2A
3. **Packaging**: ADP's OCI-based ADPKG format is ADP-specific and doesn't map to A2A
4. **Versioning**: ADP and A2A version independently; compatibility is not guaranteed

## References

- [ADP Specification](../adp-v0.1.0.md)
- [ADP Interoperability Mapping](../interop-mapping.md)
- A2A AgentCard Specification (external, draft)

## Status

This mapping is **informative** and may evolve as both ADP and A2A specifications mature. Implementations SHOULD treat `interop.a2a` as optional and gracefully handle missing or invalid AgentCard data.
