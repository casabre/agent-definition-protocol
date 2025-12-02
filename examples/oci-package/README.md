# OCI Package Example

This directory demonstrates a complete OCI-based ADPKG package structure for ADP v0.1.0.

## Structure

```
oci-package/
├── oci-layout                    # OCI layout version marker
├── index.json                    # OCI index pointing to manifest
├── blobs/
│   └── sha256/
│       ├── <config-digest>.json  # ADP config blob
│       ├── <manifest-digest>.json # OCI manifest
│       └── <layer-digest>.tar    # Package layer (tar archive)
└── README.md                     # This file
```

## Example Contents

### oci-layout
```json
{"imageLayoutVersion": "1.0.0"}
```

### index.json
Points to the manifest descriptor with media type `application/vnd.oci.image.manifest.v1+json`.

### Config Blob (blobs/sha256/<config-digest>.json)
Contains ADP-specific metadata:
```json
{
  "agent_id": "acme.analytics.agent",
  "adp_version": "0.1.0",
  "builder": {
    "id": "adp-cli",
    "version": "0.1.0"
  },
  "source": {
    "repo": "https://github.com/acme/agent-suite",
    "ref": "v1.1.0"
  },
  "build_timestamp": "2024-01-15T10:30:00Z"
}
```

### Package Layer (blobs/sha256/<layer-digest>.tar)
A tar archive containing:
```
/adp/agent.yaml          # Required: ADP manifest
/acs/container.yaml      # Optional: ACS container spec
/eval/                   # Optional: Evaluation suites
/tools/                  # Optional: Tool definitions
/src/                    # Optional: Source code
/metadata/version.json   # Recommended: Package metadata
/metadata/build-info.json # Optional: Build metadata
```

## Verification Steps

### 1. Verify OCI Layout Structure

```bash
# Check oci-layout file exists and is valid JSON
cat oci-layout | jq .

# Expected output:
# {"imageLayoutVersion": "1.0.0"}
```

### 2. Verify Index.json

```bash
# Check index.json references exactly one manifest
cat index.json | jq '.manifests | length'
# Should output: 1

# Verify manifest media type
cat index.json | jq '.manifests[0].mediaType'
# Should output: "application/vnd.oci.image.manifest.v1+json"
```

### 3. Verify Manifest

```bash
# Extract manifest digest from index
MANIFEST_DIGEST=$(cat index.json | jq -r '.manifests[0].digest | split(":")[1]')

# Verify manifest exists
ls -la blobs/sha256/$MANIFEST_DIGEST

# Check manifest structure
cat blobs/sha256/$MANIFEST_DIGEST | jq '.'

# Verify media types
cat blobs/sha256/$MANIFEST_DIGEST | jq '.config.mediaType'
# Should output: "application/vnd.adp.config.v1+json"

cat blobs/sha256/$MANIFEST_DIGEST | jq '.layers[0].mediaType'
# Should output: "application/vnd.adp.package.v1+tar"
```

### 4. Verify Config Blob

```bash
# Extract config digest from manifest
CONFIG_DIGEST=$(cat blobs/sha256/$MANIFEST_DIGEST | jq -r '.config.digest | split(":")[1]')

# Verify config exists
ls -la blobs/sha256/$CONFIG_DIGEST

# Check config structure
cat blobs/sha256/$CONFIG_DIGEST | jq '.'

# Verify required fields
cat blobs/sha256/$CONFIG_DIGEST | jq '.agent_id, .adp_version'
```

### 5. Verify Package Layer

```bash
# Extract layer digest from manifest
LAYER_DIGEST=$(cat blobs/sha256/$MANIFEST_DIGEST | jq -r '.layers[0].digest | split(":")[1]')

# Verify layer exists
ls -la blobs/sha256/$LAYER_DIGEST

# Extract and inspect layer contents
mkdir -p /tmp/layer-extract
tar -xf blobs/sha256/$LAYER_DIGEST -C /tmp/layer-extract

# Verify required file exists
ls -la /tmp/layer-extract/adp/agent.yaml

# Validate ADP manifest
cat /tmp/layer-extract/adp/agent.yaml | yq . | jq .
```

### 6. Verify Digest Integrity

```bash
# Verify config digest matches content
CONFIG_DIGEST=$(cat blobs/sha256/$MANIFEST_DIGEST | jq -r '.config.digest | split(":")[1]')
CALCULATED_DIGEST=$(sha256sum blobs/sha256/$CONFIG_DIGEST | cut -d' ' -f1)
echo "Expected: $CONFIG_DIGEST"
echo "Calculated: $CALCULATED_DIGEST"
# Should match

# Verify layer digest matches content
LAYER_DIGEST=$(cat blobs/sha256/$MANIFEST_DIGEST | jq -r '.layers[0].digest | split(":")[1]')
CALCULATED_DIGEST=$(sha256sum blobs/sha256/$LAYER_DIGEST | cut -d' ' -f1)
echo "Expected: $LAYER_DIGEST"
echo "Calculated: $CALCULATED_DIGEST"
# Should match
```

### 7. Verify Annotations

```bash
# Check OCI annotations
cat blobs/sha256/$MANIFEST_DIGEST | jq '.annotations'

# Should include:
# - org.opencontainers.image.title: agent id
# - io.adp.version: ADP version string
```

## Using SDKs for Verification

### Python SDK

```python
from adp_sdk.adpkg import ADPackage

# Open and validate OCI package
pkg = ADPackage.open("oci-package")
adp = pkg.load_adp()

# Verify structure
assert pkg.has_adp_manifest()
assert pkg.has_metadata()
```

### TypeScript SDK

```typescript
import { openPackage } from "./dist";

const pkg = await openPackage("oci-package");
const adp = await pkg.loadADP();

// Verify structure
console.assert(pkg.hasADPManifest());
console.assert(pkg.hasMetadata());
```

## Creating an OCI Package

See the SDK documentation for creating OCI packages:

- Python: `sdk/python/adp_sdk/adpkg.py`
- TypeScript: `sdk/typescript/src/adpkg.ts`
- Rust: `sdk/rust/src/adpkg.rs`
- Go: `sdk/go/adpkg/adpkg.go`

Example (Python):
```python
from adp_sdk.adpkg import ADPackage

# Create OCI package from directory
pkg = ADPackage.create_from_directory(
    source_dir="examples/acme-analytics",
    output_dir="examples/oci-package"
)
```

## Notes

- All blobs are content-addressable by SHA-256 digest
- The package layer is a tar archive, not a directory
- Config and manifest are JSON files
- Digest verification ensures integrity
- Media types identify blob formats

