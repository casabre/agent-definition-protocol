# ADPKG Packaging Overview

ADPKG is an OPC/ZIP package for shipping agents, with `.adpkg` as the recommended extension. The normative description is in `specification/core/adpkg-v0.1.md`.

Layout highlights:

- `/adp/agent.yaml` and `/acs/container.yaml` are required.
- `/eval`, `/tools`, `/src`, `/docs` may be included.
- `/metadata/version.json` captures version/build metadata.
- `[Content_Types].xml` declares MIME types for package parts.

Pack an agent:

```bash
adp pack --agent examples/acme-analytics/adp/agent.yaml --acs examples/acme-analytics/acs/container.yaml --src examples/acme-analytics/src --eval examples/acme-analytics/eval --out acme-analytics-0.1.0.adpkg
```
