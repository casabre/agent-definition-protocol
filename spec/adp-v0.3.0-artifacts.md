# ADP v0.3.0 Artifacts Specification

**Agent Definition Protocol — Versioned Named Outputs v0.3.0**

> **Version**: v0.3.0  
> **Status**: Draft  
> **Editor**: ADP Working Group  
> **Last Updated**: 2026-05-25  

---

## Abstract

This document formalizes an **Artifacts** layer — named, versioned binary/text outputs produced by an agent run. Distinct from memory stores (retrieval-oriented) and workspace (mutable scratch FS). Artifacts are **immutable once written**: each write produces a new version; older versions are preserved.

Inspired by **Google ADK** which treats artifacts as first-class session/user-scoped versioned outputs (auto-incremented integer versions, InMemoryArtifactService + GcsArtifactService).

---

## Table of Contents

1. [Top-Level `artifacts` Section](#1-top-level-artifacts-section)
2. [Artifact Path Structure](#2-artifact-path-structure)
3. [Flow Graph Binding](#3-flow-graph-binding)
4. [ESP State Extension](#4-esp-state-extension)
5. [Semantic Validation Checks](#5-semantic-validation-checks)

---

## 1. Top-Level `artifacts` Section

```yaml
artifacts:
  stores:
    - id: "run-outputs"
      scope: "session"             # session | user | agent
      provider: "gcs"              # gcs | s3 | azure_blob | inmemory | local
      bucket: "my-agent-artifacts"
      path_prefix: "runs/"         # storage path prefix; absent = no prefix
      ttl_seconds: 604800          # 7 days; absent = no expiry
      versioned: true              # if true, each write increments version counter
      credentials_env_var: "GCS_ARTIFACTS_CREDS"

    - id: "user-reports"
      scope: "user"
      provider: "s3"
      bucket: "agent-user-reports"
      ttl_seconds: 2592000         # 30 days
      versioned: true
      credentials_env_var: "S3_REPORTS_CREDS"
```

---

## 2. Artifact Path Structure (Normative)

Runners MUST store artifacts at a deterministic path:

```
{store.path_prefix}/{agent_id}/{scope_key}/{artifact_name}/{version}
```

Where `agent_id` is the manifest's top-level `id` field, and `scope_key` is:
- `scope: "session"` → `sessions/{session_id}`
- `scope: "user"` → `users/{user_id}`
- `scope: "agent"` → `shared`

Versions are **1-based integers**, auto-incremented by the runner per artifact name. Version 0 is never assigned. Runners return the version number in `state.artifacts[store_id][artifact_name].version`.

---

## 3. Flow Graph Binding

`llm`, `tool`, and `output` nodes can write artifacts by declaring `params.artifact`:

```yaml
nodes:
  - id: "generate-report"
    kind: "llm"
    model_ref: "primary"
    params:
      artifact:
        store_ref: "run-outputs"
        name: "analysis-report.md"
        content_field: "output"
```

### `content_field` Resolution per Node Kind

| Node kind | `content_field: "output"` resolves to |
|---|---|
| `llm` | `state.context[node_id].output` (model-generated text) |
| `tool` | `state.tool_responses[node_id].response` (tool call result) |
| `output` | The value of the node's `output_ref` context key |

### Failure Behavior

If the resolved `content_field` is absent in state at `on_invoke_end`, the runner MUST emit an error and skip the artifact write (no silent failure). The error is written to `state.context[node_id].artifact_error`.

Artifact writes happen in the ESP `on_invoke_end` phase, after the node's output is written to `state.context`.

---

## 4. ESP State Extension

```json
{
  "artifacts": {
    "run-outputs": {
      "analysis-report.md": {
        "version": 3,
        "written_at": "2026-05-25T09:00:00Z",
        "size_bytes": 4096,
        "store_ref": "run-outputs"
      }
    }
  }
}
```

---

## 5. Semantic Validation Checks

- **Check 33**: `artifacts.stores[].id` must be unique
- **Check 34**: `nodes[].params.artifact.store_ref` must reference a known `artifacts.stores[].id`

---

**Expert skills applied**: `role-senior-software-engineer`, `role-senior-agentic-ai-developer`

*This document is part of the ADP v0.3.0 specification. See [adp-v0.3.0.md](adp-v0.3.0.md) for the master specification.*
