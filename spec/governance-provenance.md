# Governance and Provenance

This document describes governance and provenance requirements for ADP v0.1.0. RFC 2119 terms apply.

## Status

**Provenance**: Normative for ADP v0.1.0 (see [adpkg-oci.md](adpkg-oci.md) for details)  
**Governance**: Roadmap (planned for future release)

## Provenance (Normative)

Provenance requirements are defined in the [ADPKG over OCI specification](adpkg-oci.md). This section summarizes key requirements.

### Build Metadata

ADPKG config blobs MUST include:
- `agent_id`: Agent identifier
- `adp_version`: ADP specification version
- `builder.id`: Builder tool identifier (e.g., "adp-cli")
- `source.repo`: Source repository URL
- `source.ref`: Source reference (tag, commit, branch)
- `build_timestamp`: ISO 8601 timestamp

These fields SHOULD also appear as OCI annotations on the manifest.

### Signing

- **Signatures**: MUST use Notary v2 for manifest signatures
- **Coverage**: Signatures MUST cover the ADP package layer and config
- **Verification**: Implementations SHOULD expose a `verify` command to fetch signatures and fail closed when missing

### SBOM (Software Bill of Materials)

- **Format**: MUST include an SPDX JSON SBOM
- **Location**: As an OCI referrer or additional layer
- **Media Type**: `application/vnd.adp.sbom.v1+json`
- **Content**: SHOULD include all dependencies, licenses, and component versions

### Trust Policy

Registries and consumers SHOULD define a trust policy to:
- Require signatures from approved identities
- Require valid SBOM presence
- Validate build metadata provenance

Example trust policy structure:
```json
{
  "version": "1.0",
  "trusted_identities": [
    "acme-labs@example.com"
  ],
  "required_artifacts": [
    "signature",
    "sbom"
  ],
  "required_metadata": [
    "source.repo",
    "source.ref",
    "build_timestamp"
  ]
}
```

### Verification Steps

See [adpkg-oci.md](adpkg-oci.md) for complete verification procedures. Key steps:

1. Verify OCI layout structure
2. Verify manifest and config digests
3. Verify package layer contains `/adp/agent.yaml`
4. Verify signatures (if trust policy requires)
5. Verify SBOM presence (if trust policy requires)

## Governance (Roadmap)

The following governance features are planned for future ADP releases:

### Guardrails Schema

- Formalize `guardrails` schema with policy references and enforcement modes
- Define policy bundle format and validation
- Support multiple enforcement modes (strict, warn, audit)

### Data Scopes

- Formalize semantics for `allowed_data_scopes[]` and `forbidden_data_scopes[]`
- Define data domain taxonomy
- Specify access logging requirements

### Telemetry Requirements

- Define required OpenTelemetry resource attributes
- Specify telemetry endpoint formats
- Define correlation between ADP manifests and telemetry

### Security Models

- Tool endpoint authentication schemes (bearer, env-ref, vault-ref)
- Secret handling: MUST avoid embedding secrets in manifests
- Recommend environment variables or provisioned vault references

### Privacy and Compliance

- PII handling posture documentation
- Data residency requirements
- Access logging for restricted scopes

### Current Status

For ADP v0.1.0, governance fields (`governance.identity`, `governance.allowed_data_scopes`, etc.) are defined in the [main ADP specification](adp-v0.1.0.md) but their semantics are minimal. Implementations SHOULD:

- Accept governance fields without validation
- Log governance-related decisions when possible
- Prepare for future schema formalization

## References

- [ADPKG over OCI](adpkg-oci.md) - Provenance requirements
- [ADP Specification](adp-v0.1.0.md) - Governance field definitions
- [Roadmap](../roadmap.md) - Future governance features
