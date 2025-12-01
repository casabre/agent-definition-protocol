# ADPKG on OCI (Proposed)

This document describes the canonical packaging for ADP v0.1.0 using the OCI image layout. RFC 2119 terms apply.

## Layout
- `oci-layout`: MUST be `{ "imageLayoutVersion": "1.0.0" }`.
- `index.json`: MUST reference exactly one ADPKG manifest.
- `blobs/sha256/<digest>`: MUST store config, manifest, and layer blobs addressed by digest.

## Manifest
- `mediaType`: MUST be `application/vnd.oci.image.manifest.v1+json`.
- `config`: descriptor
  - `mediaType`: MUST be `application/vnd.adp.config.v1+json`.
  - `digest`: MUST be the sha256 of config JSON.
  - `size`: MUST match payload size.
- `layers[]`: descriptors for content layers
  - ADP package layer media type: MUST be `application/vnd.adp.package.v1+tar`.
  - Layer tar MUST contain `/adp/agent.yaml` and SHOULD contain `/eval/`, `/tools/`, `/src/`, `/metadata/`, and optional `/acs/container.yaml`.
  - Layer SHOULD include `metadata/version.json` with agent id/version and build timestamp.
- `annotations`:
  - `org.opencontainers.image.title`: SHOULD be the agent id.
  - `io.adp.version`: SHOULD be the ADP manifest version string.

## Media Types (proposed)
- Config: `application/vnd.adp.config.v1+json`
- Package layer: `application/vnd.adp.package.v1+tar`
- Manifest: OCI default (`application/vnd.oci.image.manifest.v1+json`)

## Provenance and Signing (normative profile)
- **Signatures**: MUST use Notary v2 for manifest signatures; signatures MUST cover the ADP package layer and config.
- **SBOM**: MUST include an SPDX JSON SBOM as an OCI referrer or additional layer with media type `application/vnd.adp.sbom.v1+json`.
- **Build metadata**: Config MUST include builder id, source repo URL, ref, and build timestamp; these SHOULD also appear as OCI annotations.
- **Trust policy**: Registries/consumers SHOULD define a trust policy to require signatures from approved identities and a valid SBOM; an example trust policy SHOULD be published with the implementation.
- **Verification**: Implementations SHOULD expose a `verify` command to fetch signatures and SBOM referrers and fail closed when missing.

## Status
- This is the canonical packaging for ADP v0.1.0.
- Legacy OPC/ZIP is deprecated.
