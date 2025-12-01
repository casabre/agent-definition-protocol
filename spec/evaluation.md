# ADP Evaluation Specification v0.2.0

Evaluations attach quality and operational checks to agents. A suite is a set of metrics/evaluators plus scoring and promotion rules.

## Evaluator types
- **deterministic**: Rule/fixture-based checks (functions, schema validators).
- **llm_judge**: LLM-as-a-judge scoring with prompt/rubric.
- **telemetry**: Uses metrics (latency, cost, tool efficiency) from telemetry systems.

## Metric/evaluator fields
- `id` (required)
- `type` (deterministic | llm_judge | telemetry)
- Deterministic: `function`, optional `schema_path`
- LLM judge: `model`, `system_prompt`, `rubric` (list or string)
- Telemetry: `metric`, optional `max_ms`/threshold fields
- Common: `scoring` method, `threshold`, optional `execution.on` triggers, `aggregation`

## Suite structure
- `suite.id`, `suite.description`
- `suite.metrics[]` (evaluators)
- `scoring`: `{ combine, weights }`
- `promotion_threshold`: minimum composite score to pass

## ACME evaluation example
See `examples/evaluation/acme-eval-suite.yaml` for groundedness, latency, and JSON validity metrics with weighted scoring.
