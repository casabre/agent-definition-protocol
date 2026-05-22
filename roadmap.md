# ADP Roadmap

This document outlines current versioning, a high-level perspective from submission to 1.0, and planned features for future releases.

---

## Current state (v0.2.0)

| Area | Version / status |
|------|-----------------|
| **ADP manifest** | v0.2.0 spec (`spec/adp-v0.2.0.md`); schema accepts `0.1.0` and `0.2.0` |
| **Composition** | `extends` (RFC 7396), `import` (additive), `overrides` (RFC 6901) — specified and implemented in all 4 SDKs |
| **Guardrails** | Formalized schema with provider/policy_ref/mode; runner enforcement semantics specified |
| **Telemetry** | Top-level `telemetry` section with OTel gen_ai.* required_attributes; `governance.telemetry_endpoint` deprecated |
| **Tool auth** | `auth` objects on all tool types; `env_var`-based secret resolution |
| **Compliance** | `governance.compliance[]` array for GDPR/HIPAA/SOC2/EU-AI-Act/ISO-27001/FedRAMP |
| **Subflow D8** | Basic semantics specified: inputs=parent state, output→`state.context[node.id]`, isolation |
| **ESP** | v0.2.0; D1–D8 complete; 8-scenario conformance harness |
| **Frameworks** | LangGraph (round-trip), AutoGen, CrewAI, Semantic Kernel runner examples |
| **SDKs** | Python, TypeScript, Rust, Go: validate, pack, unpack, resolve_adp, checks 7–11 |

---

## Perspective: submission → review → 1.0

1. **Submission ([Agentic AI Foundation](https://aaif.io))**  
   Submit ADP + ESP as a draft specification to the [Agentic AI Foundation](https://aaif.io) for ecosystem visibility and neutral hosting.

2. **Review draft**  
   Address feedback from the foundation and community; refine narrative, scope, and optional semantics; align with MCP, A2A, and OTel where relevant.

3. **Work toward 1.0**
   - Stabilize manifest and packaging with backward-compatibility guarantees.
   - Complete governance and provenance specifications and reference tooling.
   - Publish conformance tests and conformance program.
   - Target v1.0.0 with clear stability guarantees.

---

## v0.3.0 (Planned)

### Registry protocol

`registry://` URI scheme for `extends` and `import` — resolve manifests and modules from a central registry. Includes authentication, versioning, and caching semantics.

### Notary v2 / SPDX SBOM tooling

Reference tooling for Notary v2 signing and SPDX SBOM generation. Specified as SHOULD in v0.1.x; reference implementation deferred.

### Full subflow state mapping

Selective state injection for subflow nodes: `inputs_from` and `outputs_to` fields for controlling which parent state fields are passed to the subflow and which subflow outputs are promoted to parent context. Completes D8.

### `governance.telemetry_endpoint` removal

Deprecated in v0.2.0; removed in v0.3.0. Use top-level `telemetry.endpoint` instead.

### AutoGen v0.4+ (autogen-agentchat) mapping

The v0.2.0 AutoGen runner targets the v0.2 legacy API (ConversableAgent). v0.3.0 will add a mapping guide and runner example for `autogen-agentchat` (v0.4+).

### Framework export (framework → ADP)

AutoGen, CrewAI, and SK runners are import-only in v0.2.0. Export (framework → ADP) requires framework-specific introspection APIs. v0.3.0 will evaluate and implement where feasible.

### A2A agent card full integration

Stubs present in v0.1.x; full A2A agent card alignment deferred to v0.3.0.

---

## v1.0.0 (Future)

- Stable manifest and packaging with backward-compatibility guarantees
- Complete governance and provenance specifications and reference tooling
- Conformance program and published test suite
- Production-ready tooling and ecosystem; alignment with foundation (e.g. AAIF) and related standards
