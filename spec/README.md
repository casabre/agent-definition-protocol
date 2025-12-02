# ADP Specification Directory

This directory contains the normative specifications for the Agent Definition Protocol (ADP) v0.1.0.

## Structure

### Core Specification

  - **[adp-v0.1.0.md](adp-v0.1.0.md)** - Main ADP specification document (canonical entry point)
  - Overview of ADP v0.1.0
  - Top-level structure
  - Conformance classes
  - References to component specs

### Component Specifications

- **[runtime.md](runtime.md)** - Runtime specification
  - Multi-backend execution model (docker/wasm/python/typescript/binary/custom)
  - Environment, resources, logging, healthchecks
  - Source modes (repo/inline)

- **[flow.md](flow.md)** - Flow specification (AFG v0.1)
  - Directed graph structure
  - Node types and edges
  - UI metadata

- **[evaluation.md](evaluation.md)** - Evaluation specification
  - Evaluator types (deterministic, LLM-judge, telemetry)
  - Suites, metrics, KPIs
  - Promotion policies

- **[minimal.md](minimal.md)** - ADP-Minimal conformance class
  - Minimal required fields
  - Quick-start requirements

- **[acs-v0.1.md](acs-v0.1.md)** - Agent Container Spec (ACS)
  - Container build and runtime configuration
  - Telemetry and evaluation bindings

### Packaging and Distribution

- **[adpkg-oci.md](adpkg-oci.md)** - ADPKG over OCI specification
  - OCI image layout structure
  - Media types
  - Provenance and signing (Notary v2, SBOM)

### Interoperability and Compatibility

- **[interop-mapping.md](interop-mapping.md)** - Interoperability mappings
  - ADP ↔ A2A AgentCard mapping
  - OpenTelemetry integration
  - Required minimums for ADP-Full

- **[integrations/a2a-mapping.md](integrations/a2a-mapping.md)** - Detailed A2A mapping
  - Field-by-field mapping table
  - Round-trip examples

- **[compatibility.md](compatibility.md)** - Compatibility policy
  - SemVer policy
  - Field stability guarantees
  - Extension rules

### Quality and Governance

- **[conformance.md](conformance.md)** - Conformance program
  - Conformance classes (Minimal vs Full)
  - Required behaviors
  - Test fixtures and harness

- **[governance-provenance.md](governance-provenance.md)** - Governance and provenance
  - Signing (Notary v2)
  - SBOM requirements
  - Build metadata

- **[metrics-kpi-catalog.md](metrics-kpi-catalog.md)** - Metrics catalog
  - Standardized metric identifiers
  - Units and scales
  - LLM-judge reproducibility

## Versioning

All specifications in this directory are for **ADP v0.1.0**. The version string "0.1.0" is used consistently throughout.

## Status

**Current Status**: Draft / Proposal

See [STATUS.md](../STATUS.md) for current maturity level and roadmap.

## How to Navigate

1. **New to ADP?** Start with [adp-v0.1.0.md](adp-v0.1.0.md) for the overview
2. **Implementing a runtime?** See [runtime.md](runtime.md)
3. **Building flows?** See [flow.md](flow.md)
4. **Adding evaluation?** See [evaluation.md](evaluation.md)
5. **Packaging agents?** See [adpkg-oci.md](adpkg-oci.md)
6. **Ensuring conformance?** See [conformance.md](conformance.md)

## Schemas

JSON Schemas are located in the [`schemas/`](../schemas/) directory:
- `adp.schema.json` - Main ADP schema
- `runtime.schema.json` - Runtime schema
- `flow.schema.json` - Flow schema
- `evaluation.schema.json` - Evaluation schema
- `acs.schema.json` - ACS schema
- `adpkg-metadata-v0.1.schema.json` - ADPKG metadata schema

## Examples

See [`examples/`](../examples/) directory for:
- Minimal agent: `examples/minimal/acme-minimal.yaml`
- Full agent: `examples/adp/acme-full-agent.yaml`
- Runtime examples: `examples/runtime/`
- Flow examples: `examples/flow/`
- Evaluation examples: `examples/evaluation/`

## Validation

Validate ADP manifests against schemas using:
```bash
./scripts/validate.sh
```

Or use the SDKs (Python, TypeScript, Rust, Go) for programmatic validation.

