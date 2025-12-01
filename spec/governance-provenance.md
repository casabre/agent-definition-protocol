# Governance, Provenance, and Interop (Roadmap)

These topics are required for standards alignment and enterprise adoption. Current fields are minimal; this document outlines planned enhancements and interim expectations. RFC 2119 terms apply.

## Interim requirements (ADP-Full)
- Packages SHOULD include an SBOM (SPDX JSON) as an OCI referrer or additional layer.
- Packages SHOULD be signed using Notary v2 (OCI registry signatures) with provenance tying to source repo/ref.
- Config SHOULD record builder identity, source repo/ref, and build timestamp.

## Governance (planned)
- Formalize `guardrails` schema (policy references, enforcement modes).
- Define telemetry requirements (OTEL resource attributes, endpoints).
- Data scope semantics for `allowed_data_scopes` / `forbidden_data_scopes`.
- Secret handling SHOULD avoid embedding secrets in manifests; recommend env/provisioned vault references.
- Data residency/PII: ADP-Full implementations SHOULD enforce data scope policies and log access decisions.
- Security models: Tool endpoints SHOULD declare auth schemes (e.g., bearer/env-ref) and consumers MUST avoid embedding static secrets.
- Privacy: Manifests SHOULD document PII handling posture; processors SHOULD log access to restricted scopes.

## Provenance
- Embed build metadata in ADPKG config (source repo/ref, builder, timestamp) and expose via OCI annotations.
- Provide SBOM layer (e.g., SPDX) in OCI package or referrer.
- Support signing via OCI Notary v2; document verification steps and trust policy examples.

## Interop
- Map `interop.a2a` to A2A AgentCard fields for round-trip fidelity.
- Clarify external tool schemas (MCP/API/SQL) for stronger validation.

## Status
- Not finalized; to be specified in a future ADP release alongside composition.
