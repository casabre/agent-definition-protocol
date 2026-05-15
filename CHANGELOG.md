# Changelog

All notable changes to this project will be documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)

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
