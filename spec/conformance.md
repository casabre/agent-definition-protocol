# ADP Conformance Program (v0.2.0)

This document defines how implementations demonstrate conformance to ADP v0.1.0 and ESP v0.2.0. RFC 2119 terms apply.

## What Is a Conformant Runtime?

A **conformant runtime** is any system that:

- Consumes an ADP manifest
- Interprets it according to ESP (if claiming ESP conformance), or according to its own semantics (if not)
- Executes using its own implementation

ADP does **not** maintain a registry of runtimes. Conformance means implementing the contract (schema, optional ESP semantics), not approval by ADP. Runtimes may support only subsets (e.g. ADP-Minimal, ESP-Basic). **ADP does not ship or endorse any runtime; it defines the contract runtimes may implement.**

## Conformance Classes

ADP defines two conformance classes:

- **ADP-Minimal**: MUST accept/emit manifests with required top-level fields and at least one runtime backend; MAY leave flow/evaluation empty objects.
- **ADP-Full**: MUST satisfy all required fields across runtime, flow, evaluation, tools, governance, and packaging (OCI profile). MUST validate against published schemas.

Manifests MAY declare their conformance class explicitly via the `conformance_class` field (`"minimal"` or `"full"`). If absent, implementations SHOULD infer class from content (empty `flow` or `evaluation` implies minimal). Implementations MUST reject a manifest that explicitly declares `"full"` but provides an empty `flow` or `evaluation`; the error message MUST state which section is empty.

## Required Behaviors

### Schema Validation

Implementations MUST validate ADP, runtime, flow, and evaluation documents against the JSON Schemas before packaging or deployment.

**Required schemas**:
- `schemas/adp.schema.json` - Main ADP schema
- `schemas/runtime.schema.json` - Runtime schema
- `schemas/flow.schema.json` - Flow schema
- `schemas/evaluation.schema.json` - Evaluation schema

**Validation requirements**:
- All required fields MUST be present
- Field types MUST match schema definitions
- Enum values MUST be valid
- References (e.g., `$ref`) MUST resolve

### Packaging

ADPKG-over-OCI implementations MUST:
- Include `adp/agent.yaml` in the package layer
- Follow media types defined in [adpkg-oci.md](adpkg-oci.md)
- Use OCI image layout structure (`oci-layout`, `index.json`, `blobs/sha256/`)
- Include config blob with media type `application/vnd.adp.config.v1+json`
- Include package layer with media type `application/vnd.adp.package.v1+tar`

### Interoperability

Implementations SHOULD:
- Honor `extensions.x_<vendor>` without failing validation
- Accept unknown extension fields gracefully
- Preserve extension fields when processing manifests

### Signing/SBOM (Recommended)

Implementations SHOULD:
- Attach SBOM (SPDX JSON) per the provenance profile
- Support Notary v2 signatures per [adpkg-oci.md](adpkg-oci.md)
- Provide verification tools for signatures and SBOM

## Test Fixtures

### Positive Fixtures

These fixtures MUST validate successfully:

- `fixtures/adp_full.yaml` - Full ADP manifest
- `examples/adp/acme-full-agent.yaml` - Complete agent example
- `examples/minimal/acme-minimal.yaml` - Minimal manifest
- `examples/runtime/acme-runtime-example.yaml` - Runtime example
- `examples/flow/acme-flow-example.yaml` - Flow example
- `examples/evaluation/acme-eval-suite.yaml` - Evaluation example
- `samples/python/langgraph/adp/agent.yaml` - LangGraph sample

### Negative Fixtures

These fixtures MUST fail validation with clear error messages:

- `fixtures/negative/invalid_adp_missing_runtime.yaml` - Missing required runtime
- `fixtures/negative/invalid_flow_missing_id.yaml` - Missing flow id
- `fixtures/negative/invalid_eval_bad_threshold.yaml` - Invalid threshold value

## Test Harness

### Reference Script

The reference validation script is `scripts/validate.sh`. It:
- Validates all positive fixtures against schemas
- Verifies negative fixtures fail validation
- Uses jsonschema for validation

### Implementation Requirements

Implementations SHOULD provide equivalent validation in their SDK/CLI and MAY add CI gates to block non-conformant manifests.

**Minimum test requirements**:
1. Schema validation of all positive fixtures
2. Negative fixture failure verification
3. Optional OCI package verification (signature + SBOM presence)

### CI Template

A CI template SHOULD include:

```yaml
# Example CI configuration
validate:
  - name: Validate schemas
    run: ./scripts/validate.sh
  
  - name: Test SDK validation
    run: |
      python -m pytest sdk/python/tests/test_validation.py
      npm test -- sdk/typescript/test/validation.test.ts
      cargo test --manifest-path sdk/rust/Cargo.toml validation
      go test ./sdk/go/adp/...
  
  - name: Test negative fixtures
    run: |
      for fixture in fixtures/negative/*.yaml; do
        ./scripts/validate.sh "$fixture" && exit 1 || true
      done
```

## SDK Conformance

### Reference Implementations

The following SDKs serve as reference implementations:

- **Python**: `sdk/python/adp_sdk/`
- **TypeScript**: `sdk/typescript/src/`
- **Rust**: `sdk/rust/src/`
- **Go**: `sdk/go/adp/`

### SDK Requirements

SDKs MUST provide:

1. **Validation**: `validate(adp: ADP) -> List[Error]`
   - Validate against JSON schemas
   - Return clear error messages

2. **Packaging**: `createPackage(source: Path, output: Path) -> ADPackage`
   - Create OCI-based ADPKG
   - Include required files (`adp/agent.yaml`)
   - Generate config blob with build metadata

3. **Unpacking**: `openPackage(path: Path) -> ADPackage`
   - Read OCI layout
   - Extract package layer
   - Load ADP manifest

4. **Inspection**: `inspect(package: ADPackage) -> Metadata`
   - Display package metadata
   - Show config and manifest digests
   - List package contents

### Cross-SDK Interoperability

SDKs MUST demonstrate interoperability:

- **Round-trip**: Create package with SDK A, read with SDK B
- **Shared fixtures**: All SDKs validate same fixtures
- **Consistent errors**: Similar error messages for same issues

### Test Coverage

SDKs SHOULD maintain:
- **Unit tests**: Core functionality (validation, packaging)
- **Integration tests**: End-to-end workflows
- **Fixture tests**: All positive and negative fixtures
- **Interoperability tests**: Cross-SDK round-trips

## ESP-Conformant Runners (ADP v0.2.0)

ESP-conformant runners are implementations that execute ADP agents according to the Execution Semantics Profile (ESP). See [ESP Specification](esp.md) for detailed semantics.

### ESP Conformance Levels

ESP defines two conformance levels:

- **ESP-Basic**: Implements core execution semantics:
  - Flow graph execution (node readiness, edge traversal)
  - State model (core fields: `inputs`, `context`, `memory`, `tool_responses`)
  - Basic error handling (permanent vs transient failures)

- **ESP-Full**: Implements all ESP semantics:
  - All ESP-Basic requirements
  - Tool binding semantics (`tool_ref` resolution)
  - Model and prompt resolution (`model_ref`, `system_prompt_ref`, `prompt_ref`)
  - Complete error and failure semantics (error propagation, multi-path handling)

Runners SHOULD document their conformance level.

### ESP Conformance Requirements

A runner is **ESP-conformant** (ESP-Full) if it:

1. **Correctly interprets flow graphs**:
   - Executes nodes according to node readiness rules
   - Traverses edges according to edge conditions
   - Preserves observable ordering constraints
   - Terminates runs according to termination conditions

2. **Implements state model semantics**:
   - Maintains state structure with core fields (`inputs`, `context`, `memory`, `tool_responses`)
   - Passes state between nodes correctly
   - Updates state according to node type semantics
   - Preserves state immutability constraints (`inputs` immutable)

3. **Implements tool binding semantics**:
   - Resolves `tool_ref` to tool definitions
   - Invokes tools according to tool type (MCP, HTTP, SQL)
   - Updates state with tool responses
   - Handles tool authentication correctly

4. **Implements model and prompt resolution semantics**:
   - Resolves `model_ref` via `runtime.models[]` or model registry
   - Resolves `system_prompt_ref` and `prompt_ref` via dot-notation
   - Fails gracefully on resolution failures
   - Supports provider abstraction

5. **Implements error and failure semantics**:
   - Distinguishes permanent vs transient failures
   - Handles permanent failures correctly (stop run or mark branch failed)
   - Handles transient failures with optional retries
   - Propagates errors correctly in multi-path flows

### ESP Conformance Scenarios

The following scenarios demonstrate ESP conformance:

#### Scenario 1: Simple Linear Flow

**Given**:
```yaml
flow:
  graph:
    nodes:
      - id: "input"
        kind: "input"
      - id: "process"
        kind: "llm"
        model_ref: "primary"
      - id: "output"
        kind: "output"
    edges:
      - { from: "input", to: "process" }
      - { from: "process", to: "output" }
    start_nodes: ["input"]
    end_nodes: ["output"]
```

**ESP-conformant behavior**:
1. Run begins at `input` node
2. `input` executes, initializes state: `{ inputs: {...}, context: {...}, tool_responses: {} }`
3. Edge from `input` to `process` is traversed
4. `process` becomes ready and executes:
   - Resolves `model_ref: "primary"` to model configuration
   - Invokes LLM with model and context
   - Updates `context` with LLM response
5. Edge from `process` to `output` is traversed
6. `output` executes, reads from `context`, returns result
7. Run terminates successfully

#### Scenario 2: Tool Invocation

**Given**:
```yaml
tools:
  http_apis:
    - id: "api"
      base_url: "https://api.example.com"
flow:
  graph:
    nodes:
      - id: "call-api"
        kind: "tool"
        tool_ref: "api"
```

**ESP-conformant behavior**:
1. `call-api` node executes
2. Runner resolves `tool_ref: "api"` to `tools.http_apis[0]`
3. Runner extracts parameters from `context` or `node.params`
4. Runner invokes HTTP API with `base_url` and parameters
5. Runner updates `tool_responses["api"]` with response
6. Runner MAY update `context` with response data

#### Scenario 3: Conditional Edge

**Given**:
```yaml
flow:
  graph:
    edges:
      - from: "node-a"
        to: "node-b"
        condition: "$.context.status == 'ready'"
```

**ESP-conformant behavior**:
1. After `node-a` executes, runner evaluates edge condition
2. Runner evaluates `$.context.status == 'ready'` against current state
3. If condition is `true`, edge is traversed and `node-b` becomes ready
4. If condition is `false`, edge is not traversed and `node-b` does not execute

#### Scenario 4: Multi-Path Execution

**Given**:
```yaml
flow:
  graph:
    nodes:
      - id: "split"
        kind: "router"
      - id: "path-a"
        kind: "llm"
      - id: "path-b"
        kind: "tool"
    edges:
      - { from: "split", to: "path-a" }
      - { from: "split", to: "path-b" }
```

**ESP-conformant behavior**:
1. After `split` executes, both `path-a` and `path-b` become ready
2. Runner MAY execute `path-a` and `path-b` in parallel (subject to observable ordering)
3. If `path-a` fails permanently, `path-b` MAY continue execution
4. Run terminates when all paths complete or fail

### ESP Conformance Testing

ESP-conformant runners SHOULD:

1. **Pass conformance scenarios**: Execute scenarios above and verify correct behavior
2. **Validate state structure**: Ensure state conforms to ESP state model
3. **Verify reference resolution**: Test model_ref, prompt_ref, tool_ref resolution
4. **Test error handling**: Verify permanent/transient failure handling
5. **Document behavior**: Document any deviations or extensions

### Partial ESP Conformance

Runners MAY implement **partial ESP conformance**:

- **Flow execution only**: Implements flow graph execution but not tool binding
- **State model only**: Implements state model but uses custom execution semantics
- **Reference resolution only**: Implements reference resolution but custom flow execution

Runners MUST document their partial conformance and which ESP features they support.

### ESP vs ADP Conformance

- **ADP conformance**: Validates manifests, packages correctly (SDK-level)
- **ESP conformance**: Executes agents correctly (Runner-level)

A runner MAY be ADP-conformant (validates/packages) without being ESP-conformant (executes). ESP conformance is **optional** but recommended for deterministic execution.

## Conformance Reporting

### Report Format

Implementations SHOULD publish conformance results:

```json
{
  "implementation": "adp-python-sdk",
  "version": "0.1.0",
  "adp_version": "0.1.0",
  "conformance_class": "ADP-Full",
  "test_date": "2024-01-15",
  "test_results": {
    "positive_fixtures": 7,
    "positive_passed": 7,
    "negative_fixtures": 3,
    "negative_passed": 3,
    "schema_validation": "passed",
    "oci_packaging": "passed",
    "signature_support": "partial",
    "sbom_support": "partial"
  },
  "test_environment": {
    "python_version": "3.11",
    "os": "linux"
  }
}
```

### Reporting Requirements

- **Date**: Test execution date
- **Tool version**: Implementation version
- **Schema version**: ADP schema version tested
- **Results**: Pass/fail for each test category
- **Environment**: Runtime environment details

## Pass Criteria

### ADP-Minimal

To claim ADP-Minimal conformance:
- ✅ All positive fixtures validate successfully
- ✅ All negative fixtures fail validation
- ✅ Minimal manifest requirements met

### ADP-Full

To claim ADP-Full conformance:
- ✅ All ADP-Minimal requirements met
- ✅ All required fields validated
- ✅ OCI packaging implemented
- ✅ Config blob includes build metadata
- ✅ Package layer structure correct
- ✅ (Recommended) Signature and SBOM support

## v0.2.0 Conformance Requirements

### Guardrail Enforcement

A v0.2.0-conformant runner MUST:
- Evaluate `guardrails.input[]` rails before the first flow node executes.
- Evaluate `guardrails.output[]` rails before returning from the last flow node.
- Return an error without executing further nodes when a `mode: "block"` rail fires on input.
- Return an error without returning output when a `mode: "block"` rail fires on output.

### `promotion_policy.blocking`

A v0.2.0-conformant runner that executes evaluation suites MUST:
- Stop and return an error when `promotion_policy.blocking: true` and any suite metric threshold is not met.
- Not proceed to the next environment on a blocking evaluation failure.

### Subflow (D8)

A v0.2.0-conformant runner MUST:
- Accept `subflow` nodes with `flow_ref` (inline) or `adp_ref` (external manifest reference).
- Execute the subflow graph with the parent state as inputs.
- Write the subflow result to `state.context[<subflow_node_id>]` as an object.
- NOT expose subflow internal state to the parent.
- Propagate subflow errors to the parent as permanent failures.

### Composition Conformance Class

A runner that claims **composition-conformant** status MUST implement the full `resolve_adp()` resolution order:
1. `extends` — RFC 7396 deep-merge
2. `import` — additive array merge
3. Local fields — local wins
4. `overrides` — RFC 6901 JSON Pointer patches
5. Full validation (schema + semantics) on the merged result

A composition-conformant runner MUST also:
- Detect circular `extends` chains (depth > 10 → error).
- Fail on unresolvable URIs before execution starts.
- Apply the pre-composition semantic validation guard (emit WARNING rather than hard error when validating pre-resolution manifests).

## Certification

**Note**: Formal certification is not yet available. This document defines self-reported conformance. Future versions may include:
- Official conformance test suite
- Certification badges
- Registry of conformant implementations

## References

- [ADP v0.1.0 Specification](adp-v0.1.0.md)
- [ADP v0.2.0 Specification](adp-v0.2.0.md)
- [ADPKG over OCI](adpkg-oci.md)
- [Compatibility Policy](compatibility.md)
- [Test Fixtures](../../fixtures/)

## Status

Normative for ADP v0.1.0, v0.2.0, and ESP v0.2.0 conformance claims.

---

## ADP v0.3.0 Conformance

### Agent Harness Layers

ADP v0.3.0 provides a portable **agent harness** — framework-neutral scaffolding that wraps an agent across four layers. Conformance levels apply per layer:

| Harness layer | ADP fields | Description |
|---|---|---|
| **Execution harness** | `runtime`, `pipeline`, `streaming` | How the agent runs and processes I/O |
| **Observation harness** | `telemetry`, `hooks` | What can be seen happening during execution |
| **Safety harness** | `guardrails` | What is permitted to pass through |
| **Testing harness** | `x_testing` (`aut`, `evaluators[]`, `checkers[]`) | How the agent is tested in isolation |

### v0.3.0 Optional Features

All v0.3.0 features are optional for the **minimal** conformance level. A manifest that does not use any v0.3.0-exclusive fields remains valid against the v0.3.0 schema.

| Feature | Field(s) | Level | Normative rule |
|---|---|---|---|
| Execution pipeline | `pipeline.pre_process[]`, `pipeline.post_process[]` | MAY | Runners MAY ignore pipeline stages |
| Lifecycle hooks | `hooks[]` | MAY | Runners MAY ignore hook declarations |
| Streaming policy (enabled) | `streaming.enabled` | **MUST** if declared | If `streaming.enabled: false` is declared, runner MUST NOT stream to caller |
| Streaming mode | `streaming.mode` | SHOULD | Runners SHOULD map mode to framework stream API |
| Subagents catalog | `subagents[]` | MAY | Runners MAY resolve inline or via registry |
| Framework adapter hints | `runtime.adapter_hints` | MAY | Runners MUST ignore keys for unsupported frameworks |
| A2A agent card (inline) | `interop.a2a.agent_card` | MAY | — |
| A2A authenticated extension | `interop.a2a.authenticated_extension` | MAY | Runners without auth MUST serve only the public card |
| Model extended params | `runtime.models[].top_p`, `seed`, `timeout_ms`, etc. | MAY | Runners SHOULD pass supported params to the provider |
| Semantic check 12 | `hooks[].node_filter` validity | SHOULD | Emit error for unknown node IDs in node_filter |
| Semantic check 13 | `subflow.adp_ref` validity | SHOULD | Emit error when adp_ref is a plain ID not in subagents[] |
| Semantic check 14 | `evaluator_ref` validity | SHOULD | Emit error when evaluator_ref is not in x_testing.evaluators[] |
| Unified evaluators | `x_testing.evaluators[]` | MAY | `judges[]` is a deprecated alias; runners SHOULD prefer `evaluators[]` |
| Checkers | `x_testing.checkers[]` | MAY | Fast-fail pre-filter before evaluators |

### Streaming MUST Rule

`streaming.enabled: false` is the single hardest conformance requirement in v0.3.0. A runner that streams to a caller when the manifest declares `streaming.enabled: false` is **non-conformant** regardless of framework capabilities. This applies even if `runtime.models[].use_streaming_api: true` is set — that field controls the internal LLM API call only, not the agent's external streaming contract.

### AUT Redesign (Breaking Change in v0.3.0 Testing Schema)

The `testing.schema.json` AUT adapter type enum was reduced. `adapter.type: "http"`, `"grpc"`, `"stdio"`, `"docker"`, and `"oci"` are no longer valid in the AUT block. Test invocation for those backends is derived from `runtime.execution`. Only `"function"` and `"sdk"` remain as test-specific adapter types.

Manifests that used the old AUT adapter types MUST migrate:
- Replace `aut.adapter.type: "http"` with `aut.endpoint` (override field)
- Remove `aut.id` (AUT is a singleton; id was removed)
- Move `timeout_seconds` from `adapter.timeout_seconds` to `aut.timeout_seconds`

---

## ADP v0.3.0 Harness Profiles

ADP v0.3.0 introduces **optional harness conformance profiles** that build on the core conformance classes. These profiles define which v0.3.0 features a conformant implementation MUST support.

| Profile | MUST implement | Description |
|---|---|---|
| `ADP-Harness-Memory` | `memory.stores[]`, `memory.working` (incl. compaction threshold), `memory.context_assembly` (incl. static_injection), checks 18–21, 21b, 21c, 24 | Memory & context layer with external stores and in-process working memory |
| `ADP-Harness-Loops` | `loop` node `max_iterations` + `max_tokens` enforcement; `on_max_exceeded`; `restart_context`; `loop_policy` fallback; checks 15, 15b, 16 | Orchestration loops with bounded iteration |
| `ADP-Harness-ToolPolicy` | `policy.retry`, `policy.timeout_ms`, `load_strategy` (incl. on_demand mechanics); checks 17, 29 | Tooling policy with retry, timeout, cache, rate-limit |
| `ADP-Harness-Safety` | `guardrails.interrupts[]` with HITL resume protocol + `execution_mode` (for block/log modes); `agent_trust` enforcement table; `guardrails.cost` (incl. downgrade); checks 22, 22b, 23, 30 | Extended guardrails with HITL, cost enforcement, agent trust |
| `ADP-Harness-Workspace` | `workspace` section; filesystem + git binding; remote mounts; checks 25, 25b, 26, 31 | Workspace & storage harness primitive |
| `ADP-Harness-Sandbox` | `tools.sandbox[]` with provider abstraction + snapshotting; code execution + browser binding; credential isolation rule; sandbox\<->workspace permission precedence; checks 27–28, 32 | Execution sandbox harness primitive |
| `ADP-Harness-Artifacts` | `artifacts.stores[]`; versioning semantics; session/user/agent scoping; checks 33–34 | Versioned named outputs |
| `ADP-Harness-Observability` | `observability.tracing` with OTel backend; `observability.cost_reporting`; trace events enum; checks 35, 35b | Declarative tracing configuration |
| `ADP-Harness-Adapters` | `runtime.adapter_hints` for all 8 frameworks; checks 36, 37 | SDK-level framework adapter modules |

---

## New Semantic Validation Checks (v0.3.0)

ADP v0.3.0 adds 19 new semantic validation checks (15–35b), bringing the total to 35 checks.

### Loop Checks

| Check | Description | Severity | Section |
|---|---|---|---|
| **Check 15** | `loop.body_nodes[]` must reference known node IDs in `flow.graph.nodes[]` | ERROR | [adp-v0.3.0-loops.md](adp-v0.3.0-loops.md) |
| **Check 15b** | `loop.body_nodes[]` must contain at least 2 nodes connected by at least one edge in `flow.graph.edges[]` | ERROR | [adp-v0.3.0-loops.md](adp-v0.3.0-loops.md) |
| **Check 16** | Loop node MUST NOT reference itself (directly or transitively) in `body_nodes` | ERROR | [adp-v0.3.0-loops.md](adp-v0.3.0-loops.md) |

### Tool Policy Checks

| Check | Description | Severity | Section |
|---|---|---|---|
| **Check 17** | `policy.cache.key_fields[]` entries MUST use dot-path notation (no `$` prefix, no bracket notation) | ERROR | [adp-v0.3.0-tools-policy.md](adp-v0.3.0-tools-policy.md) |
| **Check 29** | Any tool with `load_strategy: "on_demand"` MUST have a non-empty `description` field | ERROR | [adp-v0.3.0-tools-policy.md](adp-v0.3.0-tools-policy.md) |

### Memory Checks

| Check | Description | Severity | Section |
|---|---|---|---|
| **Check 18** | `memory.stores[]` IDs must be unique (post-composition) | ERROR | [adp-v0.3.0-memory.md](adp-v0.3.0-memory.md) |
| **Check 19** | `memory.operations[].store_ref` must reference a known `stores[].id` | ERROR | [adp-v0.3.0-memory.md](adp-v0.3.0-memory.md) |
| **Check 20** | `memory.context_assembly.order[].store_ref` must reference a known `stores[].id` | ERROR | [adp-v0.3.0-memory.md](adp-v0.3.0-memory.md) |
| **Check 21** | `memory.working.summary_model_ref` (when present) must reference a known `runtime.models[].id` | ERROR | [adp-v0.3.0-memory.md](adp-v0.3.0-memory.md) |
| **Check 21b** | `memory.working.summary_model_ref` MUST be present when `memory.working.strategy = "summary"` | ERROR | [adp-v0.3.0-memory.md](adp-v0.3.0-memory.md) |
| **Check 21c** | `memory.working.compaction_threshold_tokens` (when present) MUST be \<= `memory.working.max_tokens` | ERROR | [adp-v0.3.0-memory.md](adp-v0.3.0-memory.md) |
| **Check 24** | `memory.context_assembly.static_injection[].path` (when `source: "file"`) must be a relative path without `..` traversal; must also reference a declared `workspace` | ERROR | [adp-v0.3.0-memory.md](adp-v0.3.0-memory.md) |

### Guardrails Checks

| Check | Description | Severity | Section |
|---|---|---|---|
| **Check 22** | `guardrails.interrupts[].tool_refs[]` must reference known tool IDs | ERROR | [adp-v0.3.0-guardrails.md](adp-v0.3.0-guardrails.md) |
| **Check 22b** | `guardrails.interrupts[].execution_mode` MUST NOT be set when `mode: "pause_and_notify"` | ERROR | [adp-v0.3.0-guardrails.md](adp-v0.3.0-guardrails.md) |
| **Check 23** | `guardrails.cost.interrupt_ref` (when present) must reference a known `guardrails.interrupts[].id` | ERROR | [adp-v0.3.0-guardrails.md](adp-v0.3.0-guardrails.md) |
| **Check 30** | `guardrails.cost.downgrade_model_ref` MUST be present when `on_threshold_exceeded: "downgrade"`; it MUST reference a known `runtime.models[].id` | ERROR | [adp-v0.3.0-guardrails.md](adp-v0.3.0-guardrails.md) |

### Workspace Checks

| Check | Description | Severity | Section |
|---|---|---|---|
| **Check 25** | `workspace.permissions.write[]` paths MUST NOT escape `workspace.root` (no `..` traversal) | ERROR | [adp-v0.3.0-workspace.md](adp-v0.3.0-workspace.md) |
| **Check 25b** | Exactly one of `workspace.root` or `workspace.root_env_var` MUST be present | ERROR | [adp-v0.3.0-workspace.md](adp-v0.3.0-workspace.md) |
| **Check 26** | `workspace.git.auto_commit: true` requires `workspace.git.enabled: true` | ERROR | [adp-v0.3.0-workspace.md](adp-v0.3.0-workspace.md) |
| **Check 31** | `workspace.mounts[].id` values must be unique; `workspace.mounts[].target` paths MUST NOT escape `workspace.root` | ERROR | [adp-v0.3.0-workspace.md](adp-v0.3.0-workspace.md) |

### Sandbox Checks

| Check | Description | Severity | Section |
|---|---|---|---|
| **Check 27** | `tools.sandbox[].policy.timeout_ms` MUST be present (no unbounded sandbox execution) | ERROR | [adp-v0.3.0-sandbox.md](adp-v0.3.0-sandbox.md) |
| **Check 28** | `tools.sandbox[].mounts[].source: "workspace"` requires a `workspace` section to be declared | ERROR | [adp-v0.3.0-sandbox.md](adp-v0.3.0-sandbox.md) |
| **Check 32** | `tools.sandbox[].snapshot.enabled: true` with `provider: "custom"` emits a WARNING (custom providers may not support snapshots) | WARNING | [adp-v0.3.0-sandbox.md](adp-v0.3.0-sandbox.md) |

### Artifacts Checks

| Check | Description | Severity | Section |
|---|---|---|---|
| **Check 33** | `artifacts.stores[].id` must be unique | ERROR | [adp-v0.3.0-artifacts.md](adp-v0.3.0-artifacts.md) |
| **Check 34** | `nodes[].params.artifact.store_ref` must reference a known `artifacts.stores[].id` | ERROR | [adp-v0.3.0-artifacts.md](adp-v0.3.0-artifacts.md) |

### Observability Checks

| Check | Description | Severity | Section |
|---|---|---|---|
| **Check 35** | `observability.tracing.trace_events[]` entries must be from the valid enum | ERROR | [adp-v0.3.0-observability.md](adp-v0.3.0-observability.md) |
| **Check 35b** | `observability.cost_reporting.model_refs[]` (when present) must reference known `runtime.models[].id` values | ERROR | [adp-v0.3.0-observability.md](adp-v0.3.0-observability.md) |

### Adapters Checks

| Check | Description | Severity | Section |
|---|---|---|---|
| **Check 36** | `runtime.adapter_hints` keys MUST be from the known framework enum; unknown keys emit a WARNING | WARNING | [adp-v0.3.0-adapters.md](adp-v0.3.0-adapters.md) |
| **Check 37** | `import_from()` MUST place untranslatable fields into `manifest.extensions` | WARNING | [adp-v0.3.0-adapters.md](adp-v0.3.0-adapters.md) |

---

*Expert skills applied: `role-senior-software-engineer`, `role-senior-agentic-ai-developer`*
