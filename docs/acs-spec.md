# ACS Specification Overview

The Agent Container Spec (ACS) defines how to build and run containers for ADP agents. See `specification/core/acs-v0.1.md` (legacy) and schema `schemas/acs.schema.json` for details.

Highlights:

- Base image, build working directory, copy steps, dependencies.
- Runtime command, args, environment, ports, and healthcheck.
- Telemetry bindings (e.g., OTEL) and evaluation bindings.

Validate an ACS file:

```bash
adp validate --acs examples/acme-analytics/acs/container.yaml
```
