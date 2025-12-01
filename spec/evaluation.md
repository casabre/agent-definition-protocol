# ADP Evaluation Specification v0.1.0

Evaluations attach quality and operational checks to agents. A suite is a set of metrics/evaluators plus scoring and promotion rules.

## Conformance and terminology
- **Normative language**: MUST/SHOULD/MAY follow RFC 2119.
- **Conformance classes**:
  - **ADP-Full**: MUST define at least one suite with at least one metric, specify `type`, scoring, and thresholds. KPI URLs SHOULD be provided when available.
  - **ADP-Minimal**: MAY set `evaluation: {}` but SHOULD evolve to ADP-Full for deployment.

## Evaluator types
- **deterministic**: Rule/fixture-based checks (functions, schema validators).
- **llm_judge**: LLM-as-a-judge scoring with prompt/rubric.
- **telemetry**: Uses metrics (latency, cost, tool efficiency) from telemetry systems.

## Metric/evaluator fields (required unless noted)
- `id`, `type` (deterministic | llm_judge | telemetry)
- Deterministic: `function`, optional `schema_path`
- LLM judge: `model`, `system_prompt`, `rubric` (list or string)
- Telemetry: `metric`, optional `max_ms`/threshold fields
- Common: `scoring` method, `threshold`, optional `execution.on` triggers, `aggregation`

## Suite structure
- `suite.id`, `suite.description`
- `suite.metrics[]` (evaluators)
- `scoring`: `{ combine, weights }`
- `promotion_threshold`: minimum composite score to pass

## Metric/KPI conventions
- Metric ids SHOULD come from a controlled vocabulary: `latency.p95_ms`, `groundedness.score`, `cost.usd`, `tool.success_rate`.
- `threshold` SHOULD be numeric for quantitative metrics; boolean is allowed only for deterministic pass/fail checks.
- KPI URLs SHOULD be provided for dashboards and MUST be valid URIs when present.
- LLM-judge evaluators SHOULD document model version and temperature; to improve reproducibility, temperature SHOULD be 0 unless specified.
- Telemetry metrics SHOULD align to OpenTelemetry semantic conventions; if custom, define unit and aggregation explicitly.

## ACME evaluation example
See `examples/evaluation/acme-eval-suite.yaml` for groundedness, latency, and JSON validity metrics with weighted scoring.
