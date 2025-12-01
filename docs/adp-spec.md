# ADP Specification Overview

ADP is a portable manifest for defining AI agents across frameworks and runtimes. The normative text lives in `spec/` and is backed by `schemas/adp.schema.json`.

Key concepts:

- Identity and metadata: `adp_version`, `id`, `name`, `description`, `owner`, `tags`.
- Runtime: framework, entrypoint, language, models, optional OCI image reference.
- Skills: capabilities exposed by the agent with examples and IO modes.
- Tools: MCP, HTTP APIs, SQL functions.
- Memory: vector store binding.
- Evaluation: suites, KPIs (with URLs/targets), and promotion policy.
- Governance and deployment: data scopes, telemetry, environments.

Validate an ADP file with the provided schema using `scripts/validate.sh` or the SDKs.
