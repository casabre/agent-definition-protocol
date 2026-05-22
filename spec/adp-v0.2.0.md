# Agent Definition Protocol (ADP) v0.2.0

Normative for ADP v0.2.0. RFC 2119 terms apply.

**Status**: Draft  
**Version**: 0.2.0  
**Date**: 2026  
**Supersedes**: adp-v0.1.0.md (additive — all v0.1.x manifests remain valid)

---

## Summary of Changes from v0.1.x

| Area | Change |
|------|--------|
| **Composition** | New: `extends`, `import`, `overrides` fields |
| **Guardrails** | Formalized schema; was forward-declared stub in v0.1.x |
| **Telemetry** | New top-level `telemetry` section; `governance.telemetry_endpoint` deprecated |
| **Tool auth** | New `auth` object on `tools.mcp_servers[]`, `tools.http_apis[]`, `tools.sql_functions[]` |
| **Compliance** | New `governance.compliance[]` array |
| **`tool_ref`** | Now normative; was marked `(v0.2.0+)` in v0.1.x |
| **`promotion_policy.blocking`** | Now normative; deferred in v0.1.x |
| **Subflow D8** | Basic semantics specified; was deferred stub in v0.1.x |

All v0.1.x manifests with `adp_version: "0.1.0"` are accepted by v0.2.0-conformant runners. New fields are optional unless noted.

---

## Part I — Composition

Composition allows manifests to build on each other: a base manifest defines shared runtime, guardrails, and evaluation; product teams extend it with only their flow and tools. Environment-specific patches replace individual values without copying the full manifest.

### `extends` (optional, string)

A URI pointing to a base ADP manifest. Resolved before any local fields are applied.

**Supported URI schemes**: relative paths (`./base.yaml`), `file://`, `https://`. Registry URIs (`registry://`) are reserved for v0.3.0.

**Merge rule**: RFC 7396 JSON Merge Patch — objects deep-merge (local wins on conflict), arrays replace entirely, `null` removes the key from the merged result.

> ⚠ **Warning — RFC 7396 array replacement**: `extends` uses RFC 7396, which means any array key present in the child manifest **replaces** the base array entirely. For example, if the child manifest includes `evaluation: {}` (an empty object), it replaces the base's entire `evaluation` section. To inherit base evaluation, **omit the `evaluation` key entirely** in the child manifest. Use `import` for additive evaluation suite composition.

**Cycle detection**: Runners MUST detect circular `extends` chains. A chain depth greater than 10 MUST be rejected with a clear error. Runners MUST fail on unresolvable URIs before execution starts; they MUST NOT silently ignore resolution failures.

**Example**:
```yaml
adp_version: "0.2.0"
id: "agent.acme.billing"
extends: "./platform-base.yaml"
flow:
  id: "acme.billing.flow"
  graph:
    nodes:
      - { id: "input",  kind: "input" }
      - { id: "output", kind: "output" }
    edges: []
    start_nodes: ["input"]
    end_nodes:   ["output"]
# evaluation key omitted — inherits base evaluation via RFC 7396
```

### `import` (optional, array)

Each entry references a **module** — a partial ADP YAML file containing any subset of ADP fields. Modules do not require `adp_version`; `id` SHOULD be present for error messages.

**Entry schema**:
```yaml
import:
  - id: "safety-evals"
    from: "./modules/safety-evals.yaml"
    sections:          # optional; if absent, all sections are imported
      - "evaluation"
```

Fields:
- `id` (string, required): Local alias used in error messages.
- `from` (string, required): URI of the module (same schemes as `extends`).
- `sections` (array of strings, optional): Top-level ADP keys to import. If absent, all sections are imported.

**Additive merge**: Imported array fields (e.g. `evaluation.suites[]`, `guardrails.input[]`) are **appended** to existing arrays, not replaced. Imported object fields deep-merge. Later imports in the list win on object field conflicts. Local manifest fields win over all imports.

**Application order**: imports are applied after `extends`, before local fields.

**Example** — import a shared safety suite:
```yaml
adp_version: "0.2.0"
id: "agent.acme.hr"
extends: "./platform-base.yaml"
import:
  - id: "safety-evals"
    from: "https://evals.platform.example/safety-v2.yaml"
    sections: ["evaluation"]
```

### `overrides` (optional, array)

Overrides are JSON Pointer (RFC 6901) patches applied after all other composition steps — after `extends`, imports, and local fields. They are designed for environment-specific patches (e.g. swapping a model ID for prod).

**Entry schema**:
```yaml
overrides:
  - path: "/runtime/models/0/model"
    value: "gpt-4o"
    op: "set"          # set (default) | delete | append
```

Fields:
- `path` (string, required): RFC 6901 JSON Pointer into the post-merge manifest.
- `value` (any, required for `set` and `append`): Value to set or append.
- `op` (string, optional, default `"set"`): Operation — `set`, `delete`, or `append`.

**Behavior**:
- `op: set` — MUST fail with a clear error if `path` does not exist in the merged manifest.
- `op: append` — MUST fail if `path` does not resolve to an array.
- `op: delete` — If `path` does not exist, the operation is silently ignored (idempotent).

**Example** — production model and environment override:
```yaml
adp_version: "0.2.0"
id: "agent.acme.billing"
extends: "./billing-variant.yaml"
overrides:
  - { path: "/runtime/models/0/model",             value: "gpt-4o" }
  - { path: "/runtime/execution/0/env/APP_ENV",    value: "prod" }
```

### Resolution Order

Runners MUST apply composition in this order:

```
1. Fetch and merge base (extends)   — RFC 7396 deep-merge; base first, local overlay wins
2. Fetch and merge imports in order — additive arrays, deep-merge objects; later import wins; local wins over all
3. Apply local document fields      — local wins over extends and imports
4. Apply overrides                  — RFC 6901 JSON Pointer patches
5. Validate merged manifest         — call schema validation AND semantic validation on the fully resolved result
```

Semantic validation (`validate_adp_semantics`) MUST only run on fully resolved manifests. Runners and SDK functions that encounter a manifest with `extends` or `import` fields SHOULD emit the following warning before proceeding:

> `WARNING: manifest has unresolved composition fields (extends/import); semantic validation may be incomplete — call resolve_adp() first`

---

## Part II — Guardrails (Formalized)

The top-level `guardrails` field was forward-declared in v0.1.x with `additionalProperties: true`. ADP v0.2.0 formalizes its schema and adds runner enforcement requirements.

### Schema

```yaml
guardrails:
  input:
    - id: "pii-filter"
      provider: "guardrails-ai"          # guardrails-ai | nemo | azure-content-safety | llamaguard | custom
      policy_ref: "./policies/pii.rail"  # path or URI to policy definition
      mode: "block"                      # block | flag | redact | log
  output:
    - id: "content-safety"
      provider: "azure-content-safety"
      categories: ["hate", "violence"]
      threshold: "medium"                # low | medium | high
      mode: "block"
  on_violation: "block"                  # block | log; default: block
```

**Rail fields**:
- `id` (string, required): Unique identifier for the rail within this guardrails block.
- `provider` (string, required): Guardrail provider name.
- `policy_ref` (string, required): Non-empty URI or path to the policy definition. Runners MUST fail if `policy_ref` is empty.
- `mode` (string, optional): What to do when the rail fires. Default is `block`.
- `categories` (array of strings, optional): For content-safety providers — categories to check.
- `threshold` (string, optional): Sensitivity threshold for content-safety providers.

### Runner Enforcement

Runners MUST evaluate `input` rails before the first flow node executes. Runners MUST evaluate `output` rails before returning from the last flow node. If a rail with `mode: "block"` fires, the runner MUST return an error and MUST NOT execute further nodes (for input rails) or return the output (for output rails). `mode: "log"` rails MUST record the violation but MUST NOT interrupt execution.

---

## Part III — Telemetry

ADP v0.2.0 adds a dedicated top-level `telemetry` section based on OpenTelemetry Gen AI semantic conventions.

### `governance.telemetry_endpoint` Deprecation

`governance.telemetry_endpoint` is **deprecated in v0.2.0** and will be **removed in v0.3.0**. Runners SHOULD prefer `telemetry.endpoint` when both fields are present. Manifests using `governance.telemetry_endpoint` remain valid until v0.3.0.

### Schema

```yaml
telemetry:
  endpoint: "https://otel.example.com/v1"
  protocol: "http/protobuf"             # grpc | http/protobuf | http/json
  service_name: "acme-analytics"
  sampling_rate: 1.0                    # 0.0–1.0; default 1.0
  required_attributes:
    - "gen_ai.system"
    - "gen_ai.request.model"
    - "gen_ai.usage.input_tokens"
    - "gen_ai.usage.output_tokens"
```

**Fields**:
- `endpoint` (string, optional): OTel collector endpoint.
- `protocol` (string, optional): Wire protocol. One of `grpc`, `http/protobuf`, `http/json`.
- `service_name` (string, optional): OTel `service.name` resource attribute.
- `sampling_rate` (number, optional): Head-based sampling rate between `0.0` and `1.0`. Default `1.0`.
- `required_attributes` (array of strings, optional): OTel attribute names that MUST be present on every span produced by this agent.

**`required_attributes` validation**: Each entry MUST match `gen_ai.*` (OTel Gen AI semantic conventions) or be prefixed with `x_<vendor>.` for vendor-specific extensions. Any other value is a semantic validation error.

---

## Part IV — Tool Authentication

Each tool type (`mcp_servers[]`, `http_apis[]`, `sql_functions[]`) MAY declare an `auth` object. Authentication declarations are for documentation and CI verification; **runners MUST NOT read secrets from the manifest**. Secrets are resolved at execution time from environment variables named by `env_var`.

### Schema

```yaml
auth:
  scheme: "bearer"         # bearer | api_key | oauth2 | mtls | none
  env_var: "CRM_TOKEN"     # environment variable name (required when scheme != "none")
  scopes: ["read:contacts"] # oauth2 only
```

**Fields**:
- `scheme` (string, required): Authentication scheme.
- `env_var` (string, conditional): Name of the environment variable that holds the secret. MUST be non-empty when `scheme` is not `"none"`.
- `scopes` (array of strings, optional): OAuth2 scopes. Valid only when `scheme` is `"oauth2"`.

**Validation**: `env_var` MUST be present and non-empty when `scheme != "none"`. This is a semantic validation error.

**Example**:
```yaml
tools:
  http_apis:
    - id: "billing-api"
      base_url: "https://billing.acme.example"
      auth:
        scheme: "bearer"
        env_var: "BILLING_API_TOKEN"
```

---

## Part V — Compliance Posture

`governance.compliance` is a new array for declaring the compliance standards an agent must satisfy. These declarations are normative inputs to CI gates and audit tooling; they do not constitute legal compliance on their own.

### Schema

```yaml
governance:
  compliance:
    - standard: "gdpr"
      data_residency: ["eu-west-1", "eu-central-1"]
    - standard: "hipaa"
      phi_handling: "de-identify"   # de-identify | audit-log | block
    - standard: "soc2"
      audit_logging: true
    - standard: "eu-ai-act"
      risk_category: "limited"      # minimal | limited | high | unacceptable
```

**Fields**:
- `standard` (string, required): Compliance standard name. Known values: `gdpr`, `hipaa`, `soc2`, `eu-ai-act`, `iso-27001`, `fedramp`. Custom standards MUST use the `x_<vendor>.<name>` prefix convention.
- Additional standard-specific fields are permitted and ignored by runners that do not support them (`additionalProperties: true`).

**Validation**: `standard` values that are not known and do not use the `x_<vendor>.` prefix are semantic validation errors.

---

## Part VI — Deferred Items Landing in v0.2.0

### `tool_ref` on Flow Nodes (Normative)

`tool_ref` on flow nodes (`kind: "tool"`) is now normative. It was marked `(v0.2.0+)` in v0.1.x schemas.

`tool_ref` references a tool by `id` from any tool type in `tools.*` (`mcp_servers`, `http_apis`, `sql_functions`). Runners MUST resolve `tool_ref` before graph construction — if the referenced ID does not exist in `tools.*`, the runner MUST fail with:

> `"node '<node_id>' tool_ref '<tool_ref_value>' not found in tools"`

This is also a semantic validation error checked by `validate_adp_semantics`.

### `promotion_policy.blocking` (Normative)

`evaluation.promotion_policy.blocking: true` is now normative. When set to `true`, a runner executing an evaluation suite MUST stop and return an error if any evaluator's threshold is not met. The runner MUST NOT proceed to the next environment.

```yaml
evaluation:
  promotion_policy:
    require_passing_suites: ["safety"]
    blocking: true
```

### Subflow D8 — Basic Semantics

A `subflow` node invokes a nested ADP graph. Two reference modes:
- `flow_ref` (string): References an inline flow section within the same manifest.
- `adp_ref` (string): References an external ADP manifest by `id`.

**State contract (D8)**:
- **Inputs**: The subflow receives the parent's full state at the point of invocation.
- **Output**: The result of the subflow is written to `state.context[<subflow_node_id>]` as an object.
- **Isolation**: The subflow's internal state (intermediate `context`, `tool_responses`) is not visible to the parent.
- **Error propagation**: If the subflow fails, the parent runner MUST propagate the error and MUST NOT continue execution past the subflow node.

Full subflow input/output field mapping (selective state injection) is deferred to v0.3.0.

**Example**:
```yaml
flow:
  graph:
    nodes:
      - { id: "entry",  kind: "input" }
      - { id: "nested", kind: "subflow", flow_ref: "inner-flow" }
      - { id: "exit",   kind: "output", output_ref: "context.nested" }
    edges:
      - { from: "entry",  to: "nested" }
      - { from: "nested", to: "exit"   }
    start_nodes: ["entry"]
    end_nodes:   ["exit"]
```

---

## Part VII — Updated `adp_version` Field

For v0.2.0 manifests, `adp_version` MUST be `"0.2.0"`. Manifests with `adp_version: "0.1.0"` remain valid under a v0.2.0-conformant runner (backward compatibility).

---

## References

- [Execution Semantics Profile (ESP)](esp.md) — D1–D8 node semantics
- [Runtime-Flow Binding](runtime-flow-binding.md) — backend compatibility matrix
- [Framework Interoperability Guide](framework-interop.md) — LangGraph, AutoGen, CrewAI, Semantic Kernel
- [Conformance Program](conformance.md) — v0.2.0 conformance class requirements
- [ADP v0.1.0 Specification](adp-v0.1.0.md) — previous version
- RFC 7396 — JSON Merge Patch
- RFC 6901 — JavaScript Object Notation (JSON) Pointer
- OpenTelemetry Gen AI Semantic Conventions — `gen_ai.*` attribute names
