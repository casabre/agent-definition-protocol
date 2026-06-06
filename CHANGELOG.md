# Changelog

All notable changes to this project will be documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)

## [0.4.0] - 2026-06-06

### Added

**100% test coverage across all SDKs**:
- Python SDK: 353 tests, 100% line coverage enforced by `--cov-fail-under=100`; fixed autogen and crewai import-error branches; added adapter tests (PydanticAI, GoogleADK, registry) and full v0.3.0 semantic-check coverage in `test_validation_v03.py`
- Go SDK: 100% statement coverage; new `adapters_test.go` covering all 8 framework adapters (LangGraph, AutoGen, CrewAI, LlamaIndex, GoogleADK, OpenAIAgents, PydanticAI, SemanticKernel) and `AdapterRegistry`; ~80 new `ValidateADPSemantics` branch tests
- TypeScript SDK: 100% statements / branches / functions / lines (unchanged — already green)
- Rust SDK: 98.7% line coverage / 100% function coverage; 295 tests spanning `composition.rs`, `validation.rs`, `evaluation.rs`, `adpkg.rs`, `adp.rs`; remaining gaps are LLVM IR closing-brace artifacts from nested `if let` patterns (same category as `?` operator branch coverage)

**Id-keyed local field merge** — Local fields in a manifest with `extends:` now use id-keyed merge semantics instead of RFC 7396 array-replace:
- Objects deep-merge recursively (local wins on scalar collisions); absent keys added; `null` removes a key
- Lists where **all** local items carry `id` merge by id (matched entries patched in-place; unmatched base entries kept; unknown ids appended); any no-id item triggers full list replacement
- Applies at step 2 of the resolution pipeline (after `extends`, before `import` and `overrides:`); `overrides:` retains the final word
- Valid for all `adp_version` values; no manifest changes required
- Spec: `spec/adp-v0.3.0-composition.md`; implemented in Python, Go, TypeScript, and Rust SDKs

**SDK framework integrations** — `adp_sdk.integrations.*` optional subpackage:
- `adp_sdk.integrations.langgraph` — ADP↔LangGraph round-trip; exports `build_langgraph_from_adp`, `adp_from_langgraph`, `make_condition_fn`, `resolve_backend`, `ADPState`, `COMPAT_MATRIX`
- `adp_sdk.integrations.autogen` — ADP→AutoGen (pyautogen >= 0.4 / `autogen_agentchat`); rewritten for new API (`AssistantAgent`, `RoundRobinGroupChat`, `SelectorGroupChat`)
- `adp_sdk.integrations.crewai` — ADP→CrewAI Flows API (crewai >= 1.0); uses string-based `@listen` decorators compatible with crewai 1.x
- `adp_sdk.integrations.semantic_kernel` — ADP→SK KernelProcess; works in mock mode without SK installed
- `sdk/python/pyproject.toml`: optional extras `langgraph`, `autogen`, `crewai`, `semantic-kernel`, `all-integrations`
- `sdk/python/tests/integrations/`: 16 integration tests (4 per framework); skip cleanly via `pytest.importorskip` when framework is absent
- `examples/runners/*/build_adp_graph.py`: converted to thin backward-compatible shims (`from adp_sdk.integrations.<module> import *`)
- TypeScript `sdk/typescript/src/integrations/langgraph.ts` and `semantic_kernel.ts`: ESM-compatible integration modules using `createRequire`; structural tests in `test/run.ts`
- `sdk/python/README.md`: documents optional extras and integration import patterns

**ADP Testing Section (preview — v0.3.0 RFC)**:
- `schemas/testing.schema.json`: standalone schema for `x_testing` block — AUT adapter, LLM-as-judge definitions, test corpus, run record destination
- `schemas/adp.schema.json`: added `x_testing` extension key (open schema; validated separately against `testing.schema.json`)
- `schemas/evaluation.schema.json`: added optional `judge_ref` field to evaluator — links an `llm_judge` metric to a reproducibility-pinned judge in `x_testing.judges[]`
- `spec/adp-v0.3.0-testing.md`: RFC/preview spec for the `testing` section — AUT adapters, judge reproducibility, corpus sourcing, OCI/git digest pinning, `judge_ref` linkage, open decisions
- `examples/testing/billing-test.yaml`: complete `x_testing` usage example (HTTP AUT, OCI corpus with digest, git rubric with commit, `judge_ref` linkage)
- `scripts/validate.sh`: added `testing` schema + standalone `x_testing` block validation for `billing-test.yaml`

**ADP v0.3.0 — Agent Harness** (schemas, SDKs, spec, examples):

*Framing*:
- ADP now described as "the agent harness" — portable scaffolding for execution, testing, and observation
- `README.md`: replaced "OpenAPI for AI agents" with agent harness framing; added four harness layers table (execution / observation / safety / testing)

*Schemas*:
- `schemas/runtime.schema.json`: 8 new optional model fields — `top_p`, `seed`, `timeout_ms`, `use_streaming_api`, `stop_sequences` (max 4), `frequency_penalty`, `presence_penalty`, `structured_output` (format/schema/schema_ref); `adapter_hints` on runtime root
- `schemas/adp.schema.json`: `adp_version` enum adds `"0.3.0"`; new top-level fields: `subagents[]`, `pipeline` (pre/post_process stages), `hooks[]`, `streaming`; `skills[].visibility` (public/authenticated); typed `interop.a2a` with `agent_card` and `authenticated_extension`; new definitions: `pipeline_stage`, `hook`, `hook_handler`, `a2a_agent_card`, `a2a_authenticated_extension`, `a2a_authentication`, `a2a_skill`
- `schemas/testing.schema.json` (**BREAKING**: AUT adapter.type removed http/grpc/stdio/docker/oci): AUT redesigned as runtime override (`execution_ref`, `endpoint`, `timeout_seconds`, `env`); `evaluators[]` unified type replaces `judges[]` — all 4 types (llm_judge/script/container/deterministic) with if/then guards; `judges[]` kept as deprecated alias; `checkers[]` (json_schema/regex/script); `parameters` block; title updated to "ADP Testing v0.3.0"
- `schemas/evaluation.schema.json`: added `script`/`container` evaluator types; `runtime`, `inline`, `script_ref`, `image`, `image_digest`, `weight`, `threshold`, `evaluator_ref` fields; `judge_ref` marked deprecated; if/then guards for script+container types

*SDKs — all 4 (Python / TypeScript / Rust / Go)*:
- New model struct fields: `top_p`, `seed`, `timeout_ms`, `use_streaming_api`, `stop_sequences`, `frequency_penalty`, `presence_penalty`, `structured_output`
- `adp_version` accepts `"0.3.0"` in schema + struct validation
- New top-level ADP fields: `subagents[]`, `hooks`, `pipeline`, `streaming`, `x_testing`
- Semantic validation checks 12–14:
  - Check 12: `hooks[].node_filter` entries must reference known `flow.graph.nodes[].id`
  - Check 13: `subflow` node `adp_ref` (non-URI/path) must resolve to `subagents[].id`
  - Check 14: `evaluation.suites[].metrics[].evaluator_ref` must resolve to `x_testing.evaluators[].id` or `x_testing.judges[].id`
- Deprecation warning: `x_testing.judges[]` without `evaluators[]` emits `WARNING: x_testing.judges[] is deprecated`
- `EvaluationResult` type (passed, score, reason, metadata, evaluator_id, evaluator_type)
- `load_evaluator()` / `LoadEvaluator()` unified loader — `script` (bash inline + local file) and `deterministic` implemented; `llm_judge`/`container` deferred in Rust/Go with helpful error message
- Python: `referencing.Registry` replaces deprecated `RefResolver`; correctly resolves `#/definitions/*` against root document

*Spec docs*:
- `spec/adp-v0.3.0-testing.md`: **full rewrite** — v0.3.0 testing harness framing; AUT override-based design + migration table from v0.2.0; checkers (json_schema/regex/script); unified evaluators[] (all 4 types); container wire protocol (normative); git-pinned script_ref; evaluation pipeline ordering; parameters block
- `spec/adp-v0.3.0-pipeline.md`: **new file** — execution + observation harness RFC; pipeline stage execution order + contracts; hook event model + payloads; streaming policy + framework translation table; adapter_hints specification
- `spec/runtime.md`: v0.3.0 addendum — 8 new model parameters with provider mapping table; `use_streaming_api` vs `streaming.enabled` precedence rule; adapter_hints reference
- `spec/conformance.md`: v0.3.0 conformance section — four harness layers; optional features table (MAY/SHOULD/MUST); `streaming.enabled: false` hard gate; AUT breaking change migration guide
- `spec/interop-mapping.md`: expanded — A2A derivation table; authenticated extension card (merge semantics, skill visibility routing, runner serving responsibilities); ADP snake_case → A2A camelCase mapping

*Fixtures*:
- `fixtures/adp_full.yaml`: all v0.3.0 fields (model params, subagents, pipeline, hooks, streaming, adapter_hints, A2A agent_card + authenticated_extension)
- `fixtures/testing_checkers.yaml`: positive fixture — all evaluator types + all checker types
- `fixtures/negative/invalid_testing_aut_http_type.yaml`: negative — AUT `adapter.type: "http"` rejected
- `fixtures/negative/invalid_model_structured_output.yaml`: negative — invalid `structured_output.schema`
- `fixtures/semantic/sem_neg_hook_node_filter.yaml`: negative semantic — check 12
- `fixtures/semantic/sem_neg_subagent_ref.yaml`: negative semantic — check 13
- `fixtures/semantic/sem_neg_evaluator_ref.yaml`: negative semantic — check 14

*Examples*:
- `examples/adp/acme-full-agent.yaml`: updated to `adp_version: "0.3.0"`; added models[] with all new params; added `subagents[]`, `pipeline`, `hooks`, `streaming`, `runtime.adapter_hints`
- `examples/acme-analytics/adp/agent.yaml`: updated to `adp_version: "0.3.0"`; added `skills[].visibility`; replaced `interop.a2a.ref` with inline `agent_card` + `authenticated_extension`

### Changed (Breaking — testing.schema.json only)

- `testing.schema.json`: `aut.adapter.type` no longer accepts `"http"`, `"grpc"`, `"stdio"`, `"docker"`, `"oci"`. These types derived from `runtime.execution`. Migrate: use `aut.endpoint` for endpoint override; remove `aut.id`. See `spec/adp-v0.3.0-testing.md` migration table.

### Deprecated

- `x_testing.judges[]`: use `x_testing.evaluators[]` with `type: "llm_judge"`. SDK validators emit a warning. `judges[]` will be removed in v1.0.
- `evaluation.evaluators[].judge_ref`: use `evaluator_ref` which accepts any evaluator type. `judge_ref` accepted but ignored when `evaluator_ref` is also present.

## [0.2.0] - 2026-05-17

### Added

**Composition** — manifest inheritance and environment overrides:
- `extends` field: RFC 7396 JSON Merge Patch inheritance from a base manifest (objects deep-merge, arrays replace, `null` removes)
- `import` field: additive module import — arrays append, objects deep-merge; imported arrays coexist with local arrays
- `overrides` field: RFC 6901 JSON Pointer patches applied last; `set`/`append` fail on missing path; `delete` is no-op on missing
- `resolve_adp()` / `resolveAdp()` / `ResolveADP()` in all 4 SDKs — full composition resolution with cycle detection (depth > 10 → error), URI validation, and dual schema + semantic validation on the merged result
- `CompositionError` / `CompositionError` exception class in Python and TypeScript SDKs
- `schemas/adp-module.schema.json` — partial ADP schema for use with `import`
- `fixtures/composition/` — 5 YAML fixtures: base, child, module, cycle-a, cycle-b
- `examples/composition/` — 3 real-world examples: base-agent, billing-variant, prod.overlay
- Pre-composition validation guard: `validate_adp_semantics()` emits WARNING (not hard error) when manifest contains unresolved `extends`/`import` fields

**Governance formalization**:
- `guardrails` schema formalized: `input[]` and `output[]` rails with `provider`, `policy_ref`, `mode`; `on_violation` field
- New top-level `telemetry` section: OTel endpoint, protocol, service_name, sampling_rate, `required_attributes` (gen_ai.*)
- `governance.compliance[]` array: GDPR, HIPAA, SOC2, EU-AI-Act, ISO-27001, FedRAMP + custom `x_<vendor>.*`
- Tool `auth` objects on `mcp_servers[]`, `http_apis[]`, `sql_functions[]`: `scheme`, `env_var`, `scopes`

**Semantic validation checks 7–11** in all 4 SDKs:
- Check 7: guardrail `policy_ref` non-empty
- Check 8: telemetry `required_attributes` must match `gen_ai.*` or `x_<vendor>.*`
- Check 9: tool `auth.env_var` required when `scheme != "none"`
- Check 10: compliance `standard` must be known or `x_`-prefixed
- Check 11: node `tool_ref` must reference an existing tool ID in `tools.*`

**Deferred items landing**:
- `tool_ref` on flow nodes: normative in v0.2.0; `resolve_callable()` in LangGraph builder handles tool lookup
- `promotion_policy.blocking: true`: normative; runner MUST stop and return error if threshold not met
- Subflow D8: basic semantics — inputs = parent state; output → `state.context[node.id]`; isolation from parent

**Framework runner examples**:
- `examples/runners/langgraph/`: composition round-trip test added; `resolve_callable()` handles `tool_ref`
- `examples/runners/autogen/`: new — ADP → AutoGen (pyautogen >= 0.2, import-only, 4 tests)
- `examples/runners/crewai/`: new — ADP → CrewAI Flows API (crewai >= 0.63, import-only, 4 tests)
- `examples/runners/semantic-kernel/`: new — ADP → SK KernelProcess (sk >= 1.3, import-only, 4 tests)
- `examples/runners/README.md`: 4-runner comparison table

**Conformance harness**:
- Scenario 8 added (subflow / D8) to `scripts/esp-conformance-fixtures.yaml`
- `scripts/esp-runner-harness.py`: D8 invariant check; `REQUIRED_NODE_KINDS` updated to include `subflow`
- `scripts/validate.sh`: composition smoke test step added

**Spec updates**:
- `spec/adp-v0.2.0.md` — new normative specification document
- `spec/esp.md` §`subflow` node: D8 semantics specified (was deferred stub)
- `spec/conformance.md`: v0.2.0 conformance requirements (guardrails, blocking, D8, composition class)
- `spec/framework-interop.md`: composition pipeline section + all 4 framework runner guides

### Deprecated
- `governance.telemetry_endpoint`: deprecated in v0.2.0; use top-level `telemetry.endpoint`. Will be removed in v0.3.0.

### Fixed
- `adp_version` enum in `schemas/adp.schema.json` updated to accept `"0.1.0"` and `"0.2.0"`
- `acme-analytics` example updated to v0.2.0: deprecated `governance.telemetry_endpoint` replaced with `telemetry` section; `http_apis.auth` updated to structured object

## [0.1.2] - 2026-05-16

### Fixed
- `spec/esp.md` §Edge Condition Evaluation: replaced JSONPath examples with the normative bare-dot `<key> <op> <value>` format defined in `spec/runtime-flow-binding.md §Condition Expression Format` (format conflict with `runtime-flow-binding.md` resolved)
- `spec/esp.md` §`output` node: clarified `output_ref` as a dot-path from state root (e.g. `context.chat-llm.content` → `state["context"]["chat-llm"]["content"]`), consistent with condition expression key paths
- `spec/esp.md` §`router` node example: updated condition strings from JSONPath to bare-dot format
- `scripts/esp-conformance-fixtures.yaml` router-conditional scenario: router was incorrectly shown writing `state.context["decide"]`; redesigned to read from `inputs.decision` (conditions: `inputs.decision == approved`/`rejected`); `after_node: decide` state now correctly shows `context: {}` — router writes no state (D4 invariant)
- `scripts/esp-runner-harness.py` D4 check: added invariant that `state.context` must NOT contain the router node ID after the router fires

### Added
- `fixtures/semantic/sem_neg_runtime_ref.yaml` — negative fixture for `runtime_ref` referencing a nonexistent `runtime.execution` entry
- `runtime_ref` negative test in all 4 SDK test suites (Python, TypeScript, Rust, Go)

## [0.1.1] - 2026-05-16

### Added
- `spec/runtime-flow-binding.md` — normative backend compatibility matrix, condition expression format (`<key> <op> <value>`), and graph construction algorithm for runner implementers
- `spec/framework-interop.md` — informative LangGraph/AutoGen/Semantic Kernel/CrewAI mapping guide with `make_condition_fn`, `resolve_backend`, and hello-world construction pseudocode
- `runtime_ref` field in `schemas/flow.schema.json` — binds a flow node to a specific `runtime.execution[]` backend; overrides default compatibility-matrix selection
- `conformance_class` field in `schemas/adp.schema.json` — explicit `"minimal"` or `"full"` declaration; absent = infer from content
- `validate_adp_semantics()` / `validateAdpSemantics()` / `ValidateADPSemantics()` in all four SDKs — cross-schema referential integrity (edge→node, start/end nodes, suite_ref, model_ref, runtime_ref)
- `fixtures/semantic/` — four negative YAML fixtures for semantic validation (dangling edge, duplicate node, bad suite_ref, bad model_ref)
- `scripts/esp-conformance-fixtures.yaml` — seven runner conformance scenarios per ADR D1–D7 (one per node kind)
- `scripts/esp-runner-harness.py` — `RunnerAdapter` ABC, `--dry-run` ADR compliance check, `--adapter` live execution mode
- `examples/runners/langgraph/` — ADP↔LangGraph round-trip pytest suite (`build_adp_graph.py`, `conftest.py`, `test_roundtrip.py`, `requirements.txt`, `README.md`)
- Parallel branch write semantics clarified in `spec/esp.md`

### Fixed
- `conformance_class: "full"` + empty `flow` or `evaluation` now returns a validation error in all four SDKs
- `runtime.models` field now preserved through Python SDK `model_dump()` (added `extra="allow"` to `RuntimeModel`)
- `conformance_class` and `runtime_ref` fields added to Rust `Adp` struct and Go `ADP` struct for full round-trip support

## [0.1.0] - 2026-05-15

### Added
- Agent Definition Protocol (ADP) v0.1.0 specification (`spec/adp-v0.1.0.md`)
- Flow graph schema with 8 node kinds: `input`, `output`, `llm`, `tool`, `router`, `retriever`, `evaluator`, `subflow`
- Execution Semantics Profile (ESP) — per-node state write semantics (D1–D7, see `spec/decisions/esp-node-semantics.md`)
- Agent Container Spec (ACS) v0.1.0 (`spec/acs.md`)
- ADPKG OCI Image Layout packaging spec (`spec/adpkg-oci.md`)
- JSON Schema validators for `adp`, `flow`, `runtime`, `evaluation`, `acs`, `adpkg-metadata` schemas
- `evaluation.promotion_policy` field for gating promotion on passing suites
- Reference SDKs in Python, TypeScript, Rust, and Go
  - Full JSON Schema validation in all four SDKs
  - OCI packaging (`pack`/`unpack`) in all four SDKs
  - `inspect()` and `verify()` operations in all four SDKs
  - Provenance fields in config blobs (`builder.id`, `source.repo`, `source.ref`, `build_timestamp`)
- Conformance test script (`scripts/validate.sh`) covering all six schemas
- Node semantics verification script (`scripts/verify-node-semantics.py`)
- ADR: ESP node semantics (`spec/decisions/esp-node-semantics.md`)

### Changed
- Schema `$id` URIs use canonical domain `https://casabre.github.io/agent-definition-protocol/schemas/`
- `adp_version` enum locked to `["0.1.0"]`; v0.2.0 entry will be added when v0.2.0 spec is published
- `strategy` field on router nodes changed from free string to enum: `sequence`, `conditional`, `parallel`
- Go SDK module path changed to `github.com/casabre/adp-sdk`
- Normative language in `spec/interop-mapping.md` changed to informative guidance

### Fixed
- `acs.schema.json` version constant corrected from `"0.1"` to `"0.1.0"`
- OCI package example digests replaced with valid zero-hash convention
- Go SDK replaced deprecated `ioutil.ReadFile` with `os.ReadFile`
- TypeScript SDK `$ref` validation now uses Ajv draft-2020-12 (`Ajv2020`)
- Rust SDK validation correctly rejects empty `id` and invalid `adp_version`
