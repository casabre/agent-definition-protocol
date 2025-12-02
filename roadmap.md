# ADP Roadmap

This document outlines planned features for future ADP releases.

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

- Stable API with backward compatibility guarantees
- Complete governance and provenance specifications
- Standardization body submission
- Production-ready tooling and ecosystem
