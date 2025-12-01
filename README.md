# Agent Definition Protocol (ADP)

Agent Definition Protocol (ADP) is a lightweight, open specification for describing AI agents, how they run, and how they are shipped. It treats agents like first-class software artifacts: declare runtime, tools, flows, evaluation, and governance once, then move them between laptops, CI, and production without rewriting glue.

**Define an agent once, run it anywhere, and keep evaluation and governance attached using your existing tools.**

Why care? ADP v0.2.0 keeps the critical context glued to the agent: multi-backend runtimes (Docker/WASM/Python/TS/binary/custom), flow graphs (AFG v0.1), evaluation suites/KPIs, composition (extends/import/overrides), telemetry, and guardrails. You get faster rollouts with portable packages and predictable deployments—without compromising accuracy, efficiency, security, or governance.

## What is included

- **ADP v0.2.0**: YAML/JSON manifest for identity, multi-backend runtime, flow, tools, memory, evaluation (suites + KPIs), composition (extends/import/overrides), governance, and deployment.
- **ADP-Minimal** and **ADP-Full**: Minimal required fields vs. full fidelity.
- **Runtime spec**: Multi-backend model (docker/wasm/python/typescript/binary/custom) with env/resources/logging/healthcheck.
- **Flow (AFG v0.1)**: Graph of nodes/edges with UI metadata and vendor extensions.
- **Evaluation v0.2.0**: Deterministic, LLM-judge, and telemetry evaluators with triggers and aggregation.
- **CLI/SDKs**: `adp` CLI and SDKs (Python, TypeScript, Rust, Go) for validate/pack/unpack/inspect.
- **ACME examples**: Full agent plus focused runtime/flow/eval/minimal/composition examples.

## Minimal ADP example (v0.2.0)

```yaml
adp_version: "0.2.0"
id: "agent.acme.analytics"
name: "Acme Analytics Agent"
runtime:
  execution:
    - backend: "python"
      id: "python-backend"
      entrypoint: "acme_agents.main:app"
flow: {}
evaluation: {}
```

## CLI usage

```bash
# Validate the full ACME example against schemas
adp validate --adp examples/adp/acme-full-agent.yaml

# Or use the helper script (jsonschema)
./scripts/validate.sh
```

## Getting started

- Read the specs in `spec/` and walkthroughs in `docs/`.
- Try the ACME examples under `examples/` (minimal, runtime, flow, evaluation, composition, full ADP).
- Validate with `./scripts/validate.sh` or `adp validate --adp examples/adp/acme-full-agent.yaml`.
- Install the CLI from `cli/` and run `adp --help`.
- SDKs live under `sdk/` (Python, TypeScript, Rust, Go) for programmatic ADP/ADPKG handling.

Future integrations: ADP is designed to plug into Agent-Dojo and AgentHub for conformance and distribution pipelines.

## Contributing

Contributions are welcome! See `CONTRIBUTING.md`, `GOVERNANCE.md`, and `CODE_OF_CONDUCT.md`.
