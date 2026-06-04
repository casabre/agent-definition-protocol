# ADP v0.3.0 Workspace Specification

**Agent Definition Protocol — Workspace & Storage v0.3.0**

> **Version**: v0.3.0  
> **Status**: Draft  
> **Editor**: ADP Working Group  
> **Last Updated**: 2026-05-25  

---

## Abstract

This document adds the **Workspace & Storage** harness primitive to ADP. The LangChain article rates filesystem access as the #1 agent harness primitive: durable storage, workspace for reading/writing data, cross-session persistence, and git-based versioning. ADP previously had no way to declare this.

---

## Table of Contents

1. [Top-Level `workspace` Section](#1-top-level-workspace-section)
2. [Field Reference](#2-field-reference)
3. [Interaction with Memory](#3-interaction-with-memorycontext_assembly)
4. [Interaction with Sandbox](#4-interaction-with-toolsandbox)
5. [Semantic Validation Checks](#5-semantic-validation-checks)

---

## 1. Top-Level `workspace` Section

```yaml
workspace:
  # root: literal path OR env var reference
  root: "/workspaces/my-agent"   # literal path
  # OR:
  # root_env_var: "AGENT_WORKSPACE_ROOT"  # runner reads os.environ[root_env_var] at startup

  git:
    enabled: true
    auto_commit: false              # if true, runner commits after each successful invocation
    branch_per_session: false       # if true, runner creates a git branch per session_id

  permissions:
    read:  ["**"]                   # glob patterns for readable paths (default: all)
    write: ["output/**", "tmp/**"]  # glob patterns for writable paths (default: none)
    exec:  ["scripts/**"]           # glob patterns for executable paths (default: none)

  # Remote storage mounts: cloud buckets/stores mounted into workspace as directories
  mounts:
    - id: "training-data"
      provider: "s3"               # s3 | gcs | azure_blob | cloudflare_r2
      bucket: "my-agent-data"
      prefix: "training/"          # only mount objects under this prefix
      target: "data/training"      # mount point relative to workspace.root
      read_only: true
      credentials_env_var: "AWS_CREDENTIALS"

    - id: "model-outputs"
      provider: "gcs"
      bucket: "agent-outputs"
      target: "data/outputs"
      read_only: false
      credentials_env_var: "GCS_CREDENTIALS"

  cleanup:
    on: "agent_stop"                # agent_stop | session_end | never
    exclude: ["output/**"]          # paths to preserve on cleanup
```

---

## 2. Field Reference

| Field | Type | Required | Description |
|---|---|---|---|
| `root` | string | yes (if no `root_env_var`) | Workspace root as a literal path |
| `root_env_var` | string | yes (if no `root`) | Env var name (e.g. `"AGENT_WORKSPACE_ROOT"`); consistent with `tools.http_apis[].auth.env_var` pattern |
| `git.enabled` | boolean | no | Default false. Runner MUST have git available if true |
| `git.auto_commit` | boolean | no | Default false. Commits agent outputs after each run |
| `git.branch_per_session` | boolean | no | Default false. Creates `session/<id>` branch per run |
| `permissions.read[]` | glob array | no | Default `["**"]`. Restricts which paths the agent can read |
| `permissions.write[]` | glob array | no | Default `[]`. Empty = read-only workspace |
| `permissions.exec[]` | glob array | no | Default `[]`. Empty = no execution permission |
| `cleanup.on` | enum | no | When to remove workspace files |
| `cleanup.exclude[]` | glob array | no | Paths to preserve on cleanup |
| `mounts[]` | array | no | Remote storage buckets mounted into workspace |
| `mounts[].credentials_env_var` | string | no | Env var name (not value) holding mount credentials; consistent with `auth.env_var` pattern |

---

## 3. Interaction with `memory.context_assembly.static_injection`

When `static_injection[].source: "file"`, the `path` is resolved relative to `workspace.root`. If no `workspace` is declared, file-source static injection MUST be rejected by the runner (Check 24 enforces this).

---

## 4. Interaction with `tools.sandbox`

The workspace `root` is mounted as the sandbox working directory when a `sandbox` tool is present (see [`adp-v0.3.0-sandbox.md`](adp-v0.3.0-sandbox.md)). The same `permissions` apply inside the sandbox.

---

## 5. Semantic Validation Checks

- **Check 25**: `workspace.permissions.write[]` paths MUST NOT escape `workspace.root` (no `..` traversal)
- **Check 25b**: Exactly one of `workspace.root` or `workspace.root_env_var` MUST be present (mutually exclusive, one required)
- **Check 26**: `workspace.git.auto_commit: true` requires `workspace.git.enabled: true`
- **Check 31**: `workspace.mounts[].id` values must be unique; `workspace.mounts[].target` paths MUST NOT escape `workspace.root` (no `..` traversal)

---

**Expert skills applied**: `role-senior-software-engineer`, `role-senior-agentic-ai-developer`

*This document is part of the ADP v0.3.0 specification. See [adp-v0.3.0.md](adp-v0.3.0.md) for the master specification.*
