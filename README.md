# Agent Definition Protocol (ADP)

Agent Definition Protocol (ADP) is a lightweight, open specification for describing AI agents, how they run, and how they are shipped. It treats agents like first-class software artifacts: declare runtime, tools, flows, evaluation, and governance once, then move them between laptops, CI, and production without rewriting glue.

**Define an agent once, run it anywhere, and keep evaluation and governance attached using your existing tools.**

Why care? ADP v0.1.0 keeps the critical context glued to the agent: multi-backend runtimes (Docker/WASM/Python/TS/binary/custom), flow graphs (AFG v0.1), evaluation suites/KPIs, telemetry, and guardrails. You get faster rollouts with portable packages and predictable deployments—without compromising accuracy, efficiency, security, or governance. OCI-based packaging, provenance/signing (Notary v2 + SBOM guidance), and cross-SDK validation are first-class; composition (extends/import/overrides) is a future feature.

## What is included

- **ADP v0.1.0**: YAML/JSON manifest for identity, multi-backend runtime, flow, tools, memory, evaluation (suites + KPIs), governance, and deployment. Composition (extends/import/overrides) is planned for a future release.
- **ADP-Minimal** and **ADP-Full**: Minimal required fields vs. full fidelity.
- **Runtime spec**: Multi-backend model (docker/wasm/python/typescript/binary/custom) with env/resources/logging/healthcheck.
- **Flow (AFG v0.1)**: Graph of nodes/edges with UI metadata and vendor extensions.
- **Evaluation v0.1.0**: Deterministic, LLM-judge, and telemetry evaluators with triggers and aggregation.
- **SDKs**: Python, TypeScript, Rust, Go SDKs for validate/pack/unpack/inspect (OCI-based ADPKG; see `spec/adpkg-oci.md`). Conformance fixtures under `fixtures/` are shared across SDK tests.
- **ACME examples**: Full agent plus focused runtime/flow/eval/minimal examples.
- **LangGraph sample**: See `samples/python/langgraph/` for a minimal LangGraph agent packaged with ADP v0.1.0.
- **Governance/Provenance (roadmap)**: See `spec/governance-provenance.md` for signing/SBOM/provenance plans.

## Minimal ADP example (v0.1.0)

```yaml
adp_version: "0.1.0"
id: "agent.acme.analytics"
name: "Acme Analytics Agent"
runtime:
  execution:
    - backend: "python"
      id: "python-backend"
      entrypoint: "acme_agents.main:app"
flow: {}
evaluation: {}
```

## Validation
- Use `./scripts/validate.sh` to validate examples against schemas.
- SDKs provide programmatic validation and OCI packaging helpers.

## Composition (future)
- See `roadmap.md` for the placeholder. Composition is not yet finalized for interoperability.

## SDKs (Python, TS, Rust, Go)
- Python: `pip install -e sdk/python` then:
  ```python
  from adp_sdk.adp_model import ADP
  from adp_sdk.adpkg import ADPackage
  adp = ADP.from_file("examples/adp/acme-full-agent.yaml")
  pkg = ADPackage.create_from_directory("examples", "acme-oci")
  ```
- TypeScript: `cd sdk/typescript && npm install && npm run build`, then `import { createPackage, openPackage } from "./dist";`
- Rust/Go: see `sdk/rust/src/lib.rs` and `sdk/go/adp` for load/validate/create/open helpers.

## Where to look
- Specs: `spec/` (runtime, flow, evaluation, minimal, adpkg-oci, governance-provenance)
- Schemas: `schemas/`
- Examples: `examples/` (minimal, runtime, flow, evaluation, full ADP)
- Samples: `samples/python/langgraph/` (LangGraph OCI packaging)
- Validation: `./scripts/validate.sh`
- Conformance: see `spec/conformance.md` and negative fixtures in `fixtures/negative`
- Compatibility: SemVer/stability in `spec/compatibility.md`

## Compatibility policy
- ADP follows SemVer. Patch versions MUST remain backward-compatible; minor versions MAY add optional fields; major versions MAY break compatibility.
- ADPKG-over-OCI media types are versioned (`application/vnd.adp.*.v1+...`); new versions MUST NOT break existing v1 consumers without a media type bump.

## Conformance fixtures
- Shared fixtures live in `fixtures/` and are exercised by SDK test suites to ensure consistent parsing/validation across Python/TS/Rust/Go.
- `./scripts/validate.sh` validates the ACME examples plus the LangGraph sample against the schemas for a quick cross-SDK sanity check.

## Provenance and signing
- Packaging is OCI-first (`spec/adpkg-oci.md`) with guidance for Notary v2 signatures and SBOM attachment; see `spec/governance-provenance.md` for roadmap details and recommended provenance signals.

## Getting started

- Read the specs in `spec/` and walkthroughs in `docs/`.
- Try the ACME examples under `examples/` (minimal, runtime, flow, evaluation, full ADP).
- Validate with `./scripts/validate.sh` (jsonschema) or the SDKs.
- SDKs live under `sdk/` (Python, TypeScript, Rust, Go) for programmatic ADP/ADPKG handling.

Future integrations: ADP is designed to plug into Agent-Dojo and AgentHub for conformance and distribution pipelines.

## Contributing

Contributions are welcome! See `CONTRIBUTING.md`, `GOVERNANCE.md`, and `CODE_OF_CONDUCT.md`.
