#!/usr/bin/env bash
set -euo pipefail

adp pack \
  --agent ./adp/agent.yaml \
  --acs ./acs/container.yaml \
  --src ./src \
  --eval ./eval \
  --out acme-analytics-0.2.0.adpkg
