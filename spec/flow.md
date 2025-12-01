# ADP Flow Specification (AFG v0.1)

Flows describe how an agent routes work across nodes. A flow is a directed graph with typed nodes, edges, and optional UI metadata.

## Structure
- `flow.id` (required)
- `flow.graph`:
  - `nodes[]` (required): each with `id`, `kind`, optional `label`, `params`, `ui`.
  - `edges[]`: `from`, `to`, optional `condition` (simple expression string).
  - `start_nodes[]`, `end_nodes[]`: arrays of node ids.
- `ui` (optional): diagram hints under nodes: `label`, `icon`, `color`, `position {x,y}`.
- `extensions`: vendor-specific keys under `extensions.x_vendor`.

## Node kinds
`input`, `output`, `llm`, `tool`, `router`, `retriever`, `evaluator`, `subflow`.

## ACME flow example
`examples/flow/acme-flow-example.yaml` shows the ACME Analytics Flow (planner → executor → synthesizer) with UI hints.
