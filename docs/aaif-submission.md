# AAIF Sandbox Submission Checklist

This document tracks readiness for AAIF (AI Agent Interoperability Foundation) sandbox submission.

## Required Items

| Item | Status | Location |
|------|--------|----------|
| Specification document | Done | `spec/adp-v0.1.0.md` |
| Schema validators | Done | `schemas/` — 6 JSON Schema files |
| Conformance test suite | Done | `scripts/validate.sh`, `scripts/verify-node-semantics.py`, `scripts/esp-runner-harness.py` |
| Governance / provenance prose | Done | `spec/governance-provenance.md` |
| Roadmap | Done | `roadmap.md` |
| Maintainer contact | Done | `MAINTAINERS` |
| License | Verify | Check `LICENSE` file exists and is correct |
| Changelog | Done | `CHANGELOG.md` |
| GitHub Pages (schema $id resolution) | Done | `https://casabre.github.io/agent-definition-protocol/` |

## Submission Details

- **Spec location**: `spec/adp-v0.1.0.md` (primary) and `spec/esp.md` (execution semantics)
- **Schema base URL**: `https://casabre.github.io/agent-definition-protocol/schemas/`
- **Conformance tests**: `PYTHON_BIN=python3 bash scripts/validate.sh` (includes runner harness `--dry-run`) and `python3 scripts/verify-node-semantics.py`
- **Reference implementations**: Python, TypeScript, Rust, Go (under `sdk/`)
- **Maintainer**: Carsten Sauerbrey (@casabre) — see `MAINTAINERS`

## Before Submitting

- [ ] Run `PYTHON_BIN=python3 bash scripts/validate.sh` — all schemas must pass (includes runner harness `--dry-run`)
- [ ] Run `python3 scripts/verify-node-semantics.py` — all 7 node kinds must validate
- [ ] Run SDK test suites: `cd sdk/python && python3 -m pytest`, `cd sdk/typescript && npm test`, `cd sdk/rust && cargo test`, `cd sdk/go && go test ./...`
- [ ] Verify `https://casabre.github.io/agent-definition-protocol/` returns HTTP 200
- [ ] Confirm `LICENSE` file is present and correct
- [ ] Tag `v0.1.1` in git: `git tag -a v0.1.1 -m "ADP v0.1.1 — framework interop guide, runtime-flow binding, semantic validation, runner harness"`
- [ ] Review AAIF sandbox process requirements at submission time (process may have changed)

## v0.1.1 Additions (Done)

| Addition | Status | Location |
|----------|--------|----------|
| Backend compatibility matrix + condition expression spec | Done | `spec/runtime-flow-binding.md` |
| Framework interop guide (LangGraph/AutoGen/SK/CrewAI) | Done | `spec/framework-interop.md` |
| Cross-schema semantic validation in all 4 SDKs | Done | `sdk/*/validation.*` |
| Runner conformance harness (7 ADR scenarios) | Done | `scripts/esp-runner-harness.py`, `scripts/esp-conformance-fixtures.yaml` |
| LangGraph round-trip pytest suite | Done | `examples/runners/langgraph/` |
| `conformance_class` field in ADP schema | Done | `schemas/adp.schema.json` |
| `runtime_ref` field in flow schema | Done | `schemas/flow.schema.json` |
| Deployment-time override gap documented | Done | `spec/runtime-flow-binding.md §5` |

## Known Gaps (Acceptable for Sandbox)

- Notary v2 signing: specified as SHOULD; reference tooling planned for v0.2.0
- SPDX SBOM generation: specified as SHOULD; reference tooling planned for v0.2.0
- A2A agent card integration: stubs present; full alignment deferred to v0.2.0
- `subflow` node kind: schema-defined but semantics deferred to v0.2.0
