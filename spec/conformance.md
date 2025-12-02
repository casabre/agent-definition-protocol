# ADP Conformance Program (v0.1.0)

This document defines how implementations demonstrate conformance to ADP v0.1.0. RFC 2119 terms apply.

## Conformance Classes

ADP defines two conformance classes:

- **ADP-Minimal**: MUST accept/emit manifests with required top-level fields and at least one runtime backend; MAY leave flow/evaluation empty objects.
- **ADP-Full**: MUST satisfy all required fields across runtime, flow, evaluation, tools, governance, and packaging (OCI profile). MUST validate against published schemas.

## Required Behaviors

### Schema Validation

Implementations MUST validate ADP, runtime, flow, and evaluation documents against the JSON Schemas before packaging or deployment.

**Required schemas**:
- `schemas/adp.schema.json` - Main ADP schema
- `schemas/runtime.schema.json` - Runtime schema
- `schemas/flow.schema.json` - Flow schema
- `schemas/evaluation.schema.json` - Evaluation schema

**Validation requirements**:
- All required fields MUST be present
- Field types MUST match schema definitions
- Enum values MUST be valid
- References (e.g., `$ref`) MUST resolve

### Packaging

ADPKG-over-OCI implementations MUST:
- Include `adp/agent.yaml` in the package layer
- Follow media types defined in [adpkg-oci.md](adpkg-oci.md)
- Use OCI image layout structure (`oci-layout`, `index.json`, `blobs/sha256/`)
- Include config blob with media type `application/vnd.adp.config.v1+json`
- Include package layer with media type `application/vnd.adp.package.v1+tar`

### Interoperability

Implementations SHOULD:
- Honor `extensions.x_<vendor>` without failing validation
- Accept unknown extension fields gracefully
- Preserve extension fields when processing manifests

### Signing/SBOM (Recommended)

Implementations SHOULD:
- Attach SBOM (SPDX JSON) per the provenance profile
- Support Notary v2 signatures per [adpkg-oci.md](adpkg-oci.md)
- Provide verification tools for signatures and SBOM

## Test Fixtures

### Positive Fixtures

These fixtures MUST validate successfully:

- `fixtures/adp_full.yaml` - Full ADP manifest
- `examples/adp/acme-full-agent.yaml` - Complete agent example
- `examples/minimal/acme-minimal.yaml` - Minimal manifest
- `examples/runtime/acme-runtime-example.yaml` - Runtime example
- `examples/flow/acme-flow-example.yaml` - Flow example
- `examples/evaluation/acme-eval-suite.yaml` - Evaluation example
- `samples/python/langgraph/adp/agent.yaml` - LangGraph sample

### Negative Fixtures

These fixtures MUST fail validation with clear error messages:

- `fixtures/negative/invalid_adp_missing_runtime.yaml` - Missing required runtime
- `fixtures/negative/invalid_flow_missing_id.yaml` - Missing flow id
- `fixtures/negative/invalid_eval_bad_threshold.yaml` - Invalid threshold value

## Test Harness

### Reference Script

The reference validation script is `scripts/validate.sh`. It:
- Validates all positive fixtures against schemas
- Verifies negative fixtures fail validation
- Uses jsonschema for validation

### Implementation Requirements

Implementations SHOULD provide equivalent validation in their SDK/CLI and MAY add CI gates to block non-conformant manifests.

**Minimum test requirements**:
1. Schema validation of all positive fixtures
2. Negative fixture failure verification
3. Optional OCI package verification (signature + SBOM presence)

### CI Template

A CI template SHOULD include:

```yaml
# Example CI configuration
validate:
  - name: Validate schemas
    run: ./scripts/validate.sh
  
  - name: Test SDK validation
    run: |
      python -m pytest sdk/python/tests/test_validation.py
      npm test -- sdk/typescript/test/validation.test.ts
      cargo test --manifest-path sdk/rust/Cargo.toml validation
      go test ./sdk/go/adp/...
  
  - name: Test negative fixtures
    run: |
      for fixture in fixtures/negative/*.yaml; do
        ./scripts/validate.sh "$fixture" && exit 1 || true
      done
```

## SDK Conformance

### Reference Implementations

The following SDKs serve as reference implementations:

- **Python**: `sdk/python/adp_sdk/`
- **TypeScript**: `sdk/typescript/src/`
- **Rust**: `sdk/rust/src/`
- **Go**: `sdk/go/adp/`

### SDK Requirements

SDKs MUST provide:

1. **Validation**: `validate(adp: ADP) -> List[Error]`
   - Validate against JSON schemas
   - Return clear error messages

2. **Packaging**: `createPackage(source: Path, output: Path) -> ADPackage`
   - Create OCI-based ADPKG
   - Include required files (`adp/agent.yaml`)
   - Generate config blob with build metadata

3. **Unpacking**: `openPackage(path: Path) -> ADPackage`
   - Read OCI layout
   - Extract package layer
   - Load ADP manifest

4. **Inspection**: `inspect(package: ADPackage) -> Metadata`
   - Display package metadata
   - Show config and manifest digests
   - List package contents

### Cross-SDK Interoperability

SDKs MUST demonstrate interoperability:

- **Round-trip**: Create package with SDK A, read with SDK B
- **Shared fixtures**: All SDKs validate same fixtures
- **Consistent errors**: Similar error messages for same issues

### Test Coverage

SDKs SHOULD maintain:
- **Unit tests**: Core functionality (validation, packaging)
- **Integration tests**: End-to-end workflows
- **Fixture tests**: All positive and negative fixtures
- **Interoperability tests**: Cross-SDK round-trips

## Conformance Reporting

### Report Format

Implementations SHOULD publish conformance results:

```json
{
  "implementation": "adp-python-sdk",
  "version": "0.1.0",
  "adp_version": "0.1.0",
  "conformance_class": "ADP-Full",
  "test_date": "2024-01-15",
  "test_results": {
    "positive_fixtures": 7,
    "positive_passed": 7,
    "negative_fixtures": 3,
    "negative_passed": 3,
    "schema_validation": "passed",
    "oci_packaging": "passed",
    "signature_support": "partial",
    "sbom_support": "partial"
  },
  "test_environment": {
    "python_version": "3.11",
    "os": "linux"
  }
}
```

### Reporting Requirements

- **Date**: Test execution date
- **Tool version**: Implementation version
- **Schema version**: ADP schema version tested
- **Results**: Pass/fail for each test category
- **Environment**: Runtime environment details

## Pass Criteria

### ADP-Minimal

To claim ADP-Minimal conformance:
- ✅ All positive fixtures validate successfully
- ✅ All negative fixtures fail validation
- ✅ Minimal manifest requirements met

### ADP-Full

To claim ADP-Full conformance:
- ✅ All ADP-Minimal requirements met
- ✅ All required fields validated
- ✅ OCI packaging implemented
- ✅ Config blob includes build metadata
- ✅ Package layer structure correct
- ✅ (Recommended) Signature and SBOM support

## Certification

**Note**: Formal certification is not yet available. This document defines self-reported conformance. Future versions may include:
- Official conformance test suite
- Certification badges
- Registry of conformant implementations

## References

- [ADP Specification](adp-v0.1.0.md)
- [ADPKG over OCI](adpkg-oci.md)
- [Compatibility Policy](compatibility.md)
- [Test Fixtures](../../fixtures/)

## Status

Normative for ADP v0.1.0 conformance claims.
