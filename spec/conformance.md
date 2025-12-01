# ADP Conformance Program (v0.1.0)

This document defines how implementations demonstrate conformance to ADP v0.1.0. RFC 2119 terms apply.

## Classes
- **Minimal**: MUST accept/emit manifests with required top-level fields and at least one runtime backend; MAY leave flow/evaluation empty objects.
- **Full**: MUST satisfy all required fields across runtime, flow, evaluation, tools, governance, and packaging (OCI profile). MUST validate against published schemas.

## Required behaviors
- **Schema validation**: Implementations MUST validate ADP, runtime, flow, and evaluation documents against the JSON Schemas before packaging or deployment.
- **Packaging**: ADPKG-over-OCI MUST include `adp/agent.yaml` and follow media types in `spec/adpkg-oci.md`.
- **Interop**: SHOULD honor `extensions.x_<vendor>` without failing validation.
- **Signing/SBOM (recommendation)**: SHOULD attach SBOM and Notary v2 signatures per the provenance profile.

## Fixtures
- Positive fixtures: `fixtures/adp_full.yaml`, `examples/*`, `samples/python/langgraph/*`.
- Negative fixtures: `fixtures/negative/invalid_adp_missing_runtime.yaml`, `fixtures/negative/invalid_flow_missing_id.yaml`, `fixtures/negative/invalid_eval_bad_threshold.yaml`.

## Harness
- Reference script: `scripts/validate.sh` (uses jsonschema).
- Implementations SHOULD provide equivalent validation in their SDK/CLI and MAY add CI gates to block non-conformant manifests.
- A CI template SHOULD include: (a) schema validation of all manifests, (b) negative fixture failures, (c) optional OCI package verification (signature + SBOM presence) per `spec/adpkg-oci.md`.

## Pass criteria
- All positive fixtures MUST validate against schemas.
- All negative fixtures MUST fail validation with a clear error.
- If signature/SBOM verification is enabled, ADPKG artifacts MUST present a valid Notary v2 signature and SPDX referrer.

## Reporting
- Implementations SHOULD publish conformance results (date, tool version, schema version).

Status: Normative for ADP v0.1.0 conformance claims.
