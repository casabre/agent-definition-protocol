# ADP Roadmap

This document outlines current versioning, a high-level perspective from submission to 1.0, and planned features for future releases.

---

## Current state (versioning and features)

| Area                                    | Version / status                                                                               |
| --------------------------------------- | ---------------------------------------------------------------------------------------------- |
| **ADP manifest**                        | v0.1.0 spec; schema supports `adp_version` 0.1.0 and 0.2.0                                     |
| **ESP (Execution Semantics Profile)**   | v0.2.0; framework-neutral execution contract for runners                                       |
| **Runtime / flow / evaluation schemas** | Published; v0.2.0 titles where extended (e.g. models, node refs)                               |
| **ADPKG (OCI packaging)**               | Spec in `spec/adpkg-oci.md`; provenance/signing guidance                                       |
| **Conformance**                         | `spec/conformance.md`: ADP-Minimal, ADP-Full, ESP-conformant runners; fixtures and validate.sh |
| **SDKs**                                | Python, TypeScript, Rust, Go: validate, pack, unpack, inspect                                  |
| **Governance / provenance**             | Normative prose and roadmap in `spec/governance-provenance.md`; full tooling TBD               |

Composition (extends/import/overrides) is **not** yet specified; see section below.

---

## Perspective: submission → review → 1.0

High-level path from foundation submission to a stable 1.0:

1. **Submission ([Agentic AI Foundation](https://aaif.io))**  
   Submit ADP + ESP as a draft specification (e.g. sandbox or similar stage) to the [Agentic AI Foundation](https://aaif.io) for ecosystem visibility and neutral hosting.

2. **Review draft**  
   Address feedback from the foundation and the community; refine narrative, scope, and optional semantics (ESP); align with other agent standards (MCP, A2A, OTel) where relevant.

3. **Work toward 1.0**  
   - Stabilize manifest and packaging (backward-compatibility expectations).  
   - Complete governance and provenance specifications and reference tooling.  
   - Publish conformance tests and, if applicable, a conformance program.  
   - Target a v1.0.0 release with clear stability and compatibility guarantees.

This perspective is indicative; actual stages and names depend on the foundation’s process.

---

## v0.2.0 (Planned)

### Composition

Composition (extends/import/overrides) is planned for a future ADP release. Do not rely on these fields for interoperability until a finalized spec and schema are published.

## Goals (proposed)
- **extends**: inherit from a base ADP manifest (versioned identifier). Deep-merge semantics; local fields override inherited fields.
- **import**: pull modules (flow, prompts, guardrails, evaluation, tools) by reference. Module content is merged into the current manifest.
- **overrides**: patch-like updates using dotted/JSON-pointer style paths. Last writer wins.

## Proposed merge order
1. Load base ADP (if `extends.adp` present).
2. Apply module imports (`import.*`).
3. Apply local document fields.
4. Apply overrides (patch paths).

Conflicts resolve by last writer; invalid paths SHOULD fail validation.

## Extensions (non-normative)
- Implementers MAY add vendor-specific data under `extensions.x_<vendor>` objects in runtime/flow/evaluation to avoid collisions.
- Extension fields SHOULD be documented and SHOULD NOT redefine core semantics.

## Validation (future)
- Referenced modules/ADPs must resolve and validate against schemas.
- Overrides must target existing or whitelisted paths.
- Merged result must satisfy ADP schema.

## Ignore-safe behavior
- Current validators SHOULD ignore composition fields (extends/import/overrides) without failing, while emitting warnings, until a normative schema is published.

## Status
- Placeholder only; no schema or tooling support is provided yet.

### Enhanced Governance

Formal governance features are planned for v0.2.0:

- **Guardrails Schema**: Formalize policy references and enforcement modes
- **Data Scopes**: Formalize data domain taxonomy and access logging
- **Telemetry Requirements**: Define required OTel resource attributes
- **Security Models**: Tool authentication schemes and secret handling
- **Privacy**: PII handling posture and compliance requirements

See `spec/governance-provenance.md` for current status and roadmap details.

## v1.0.0 (Future)

- Stable manifest and packaging with backward-compatibility guarantees
- Complete governance and provenance specifications and reference tooling
- Conformance program and published test suite
- Production-ready tooling and ecosystem; alignment with foundation (e.g. AAIF) and related standards
