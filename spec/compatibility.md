# Compatibility Policy (SemVer)

This document records stability expectations for ADP v0.1.0. RFC 2119 terms apply.

## SemVer
- Patch versions MUST be backward-compatible.
- Minor versions MAY add optional fields but MUST NOT break existing required fields.
- Major versions MAY introduce breaking changes.

## Field stability (v0.1.0)
- **Stable**: `adp_version`, `id`, `runtime.execution`, `flow.id`, `flow.graph`, `evaluation.suites`, `tools`, `memory`, `governance`, `deployment`, OCI packaging layout/media types.
- **Planned/Future**: composition (`extends/import/overrides`), advanced guardrails schema.
- **Extensions**: `extensions.x_<vendor>` and fields marked additionalProperties MAY change; consumers SHOULD ignore unknown extensions.

## Field compatibility table (selected)
- `runtime.backend` (stable): enum extensions require minor/major bump.
- `flow.graph.nodes.kind` (stable): new kinds require minor/major bump and schema update.
- `evaluation.evaluator.type` (stable): new types require minor/major bump.
- `interop.a2a` (informative/experimental): non-breaking changes only; treat as optional.
- `composition` (future): non-normative; ignore safely.

## Extension rules
- Vendors MAY add namespaced keys under `extensions.x_<vendor>`; they MUST NOT override core semantics.
- New core fields SHOULD be added as optional first; once marked required, a minor/major version bump is needed.
