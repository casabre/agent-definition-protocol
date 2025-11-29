# Agent Definition Protocol (ADP)

Agent Definition Protocol (ADP) is a lightweight, open specification for describing AI agents, how they run, and how they are shipped. It treats agents like first-class software artifacts: you declare their runtime, tools, memory, evaluation, and governance once, then move them between laptops, CI, and production without rewriting glue.

**Define an agent once, run it anywhere, and keep evaluation and governance attached using your existing tools.**

Why care? ADP keeps the critical context glued to the agent: models and entrypoints, container recipe, evaluation suites/KPIs, telemetry, and guardrails. You get faster rollouts with portable ADPKG bundles and predictable deployments via ACS—without compromising accuracy, efficiency, security, or governance.

## What is included

- **ADP**: YAML/JSON manifest for agent identity, runtime, tools, memory, evaluation, governance, and deployment.
- **ACS**: YAML container recipe for building/running an ADP agent image.
- **ADPKG**: OPC/ZIP package bundling the manifest, container spec, evaluations, tools, metadata, and source.
- **CLI**: `adp` command to validate manifests, pack/unpack `.adpkg`, and inspect basics.

## Minimal ADP example

```yaml
adp_version: "0.1"
id: "agent.acme.analytics"
name: "Acme Analytics Agent"
description: "Answers analytic questions over Acme telemetry."
owner: "acme-labs"
tags: ["analytics", "demo"]
runtime:
  framework: "langgraph"
  entrypoint: "acme_agents.main:app"
  language: "python"
  models:
    - provider: "openai"
      name: "gpt-4o"
skills:
  - id: "answer"
    name: "Answer queries"
    description: "Respond with concise answers from indexed telemetry"
    tags: ["qa"]
    examples: ["What is p95 latency?"]
    input_modes: ["text"]
    output_modes: ["text"]

# ... see specification/core/adp-v0.1.md for full fields
```

## CLI usage

```bash
adp validate --adp examples/acme-analytics/adp/agent.yaml
adp pack \
  --agent examples/acme-analytics/adp/agent.yaml \
  --acs examples/acme-analytics/acs/container.yaml \
  --src examples/acme-analytics/src \
  --eval examples/acme-analytics/eval \
  --out acme-analytics-0.1.0.adpkg
```

## Getting started

- Read the specs in `specification/core/` and walkthroughs in `docs/`.
- Try the example under `examples/acme-analytics`.
- Install the CLI from `cli/` and run `adp --help`.

## Contributing

Contributions are welcome! See `CONTRIBUTING.md`, `GOVERNANCE.md`, and `CODE_OF_CONDUCT.md`.
