# Metrics and KPI Catalog (v0.1.0)

This catalog standardizes metric identifiers, units, and scales for ADP evaluation/KPIs. RFC 2119 terms apply.

## Latency
- `latency.p50_ms`, `latency.p95_ms`, `latency.p99_ms` (unit: ms, higher is worse) — thresholds SHOULD be numeric.

## Cost
- `cost.usd` (unit: USD total), `cost.usd_per_call` — thresholds SHOULD be numeric.

## Groundedness / Quality
- `groundedness.score` (scale 1–5) — LLM-judge SHOULD set temperature=0; threshold SHOULD be numeric (e.g., >=4.0).
- `json_validity` (boolean) — deterministic schema checks.

## Tooling success
- `tool.success_rate` (0–1), `tool.error_rate` (0–1) — telemetry-based.

## Throughput
- `throughput.qps` (queries per second) — telemetry-based.

LLM-judge reproducibility: use fixed model version, temperature 0, and document prompt/rubric. Custom metrics MUST define unit and scale.
