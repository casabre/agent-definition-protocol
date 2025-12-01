# ADPKG Packaging Overview

ADPKG is a package for shipping agents. For ADP v0.1.0, OCI image layout is canonical (see `spec/adpkg-oci.md`). Legacy OPC/ZIP is deprecated.

OCI layout highlights:
- `oci-layout` and `index.json` following OCI image layout.
- `blobs/sha256/<digest>` containing manifest, config, and package layer.
- Package layer tar includes `/adp/agent.yaml` (required), `/acs/container.yaml` (optional), `/eval/`, `/tools/`, `/src/`, `/metadata/`.
- Config blob captures minimal metadata (agent id, adp_version).

Media types (proposed):
- Config: `application/vnd.adp.config.v1+json`
- Package layer: `application/vnd.adp.package.v1+tar`
- Manifest: `application/vnd.oci.image.manifest.v1+json`
