#!/usr/bin/env bash
set -euo pipefail

# Package the LangGraph sample as an OCI-based ADPKG using the Python SDK
ROOT=$(cd "$(dirname "$0")" && pwd)
OUT=${1:-$ROOT/langgraph-oci}
mkdir -p "$OUT"
python - <<'PY'
from pathlib import Path
from adp_sdk.adpkg import ADPackage
root = Path("$ROOT")
out = Path("$OUT")
ADPackage.create_from_directory(root, out)
print(f"Wrote OCI ADPKG layout to {out}")
PY
