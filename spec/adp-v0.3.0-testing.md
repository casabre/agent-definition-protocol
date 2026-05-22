# ADP Testing Harness (v0.3.0)

**Status: Normative for v0.3.0. Replaces the v0.2.0 RFC/preview.**

The `x_testing` section provides the **testing harness** layer of ADP — portable, framework-neutral scaffolding for testing an agent in isolation. Define once, run in any conformant CI or evaluation platform.

---

## Agent Harness Context

ADP wraps an agent across four layers:

| Layer | ADP fields | Role |
|---|---|---|
| Execution harness | `runtime`, `pipeline`, `streaming` | How the agent runs |
| Observation harness | `telemetry`, `hooks` | What can be seen |
| Safety harness | `guardrails` | What is permitted |
| **Testing harness** | `x_testing` | **How the agent is tested** |

The testing harness answers: how do you invoke the agent under test, what fast-fail assertions run first, what scored evaluations run next, and what constitutes a passing run.

---

## Evaluation Pipeline

When a test scenario runs, execution follows this ordered pipeline:

```
AUT invocation → checkers[] (fast-fail) → evaluators[] (scored) → promotion_policy
```

1. **AUT invokes the agent** — the runner calls the agent using the AUT declaration (or derives invocation from `runtime.execution`).
2. **Checkers run first** — fast-fail pre-filters. Any checker failure short-circuits the scenario immediately. No evaluators run after a checker fails.
3. **Evaluators run in declared order** — each produces an `EvaluationResult` (`passed`, `score`, `reason`). Multiple evaluators CAN run per scenario.
4. **Promotion policy aggregates** — `promotion_threshold` determines the passing bar across evaluator scores.

---

## AUT — Agent Under Test

The AUT is an **optional override** of `runtime.execution`, not a parallel invocation declaration. If `aut` is absent, the runner uses `runtime.execution[0]` (or the first compatible backend).

**v0.3.0 AUT fields:**

```yaml
x_testing:
  aut:
    description: "Staging override for testing"
    execution_ref: "my-python-backend"   # optional: selects runtime.execution[] entry by id
    endpoint: "https://staging.example/invoke"  # override endpoint for testing
    timeout_seconds: 30
    env:
      TEST_MODE: "true"
    adapter:
      type: "function"            # test-specific adapter (not in runtime.execution)
      function_ref: "tests.runner:invoke_agent"
```

| Field | Description |
|---|---|
| `execution_ref` | References `runtime.execution[].id` to select backend. Absent → first compatible backend. |
| `endpoint` | Override endpoint for testing (e.g., staging URL). |
| `timeout_seconds` | Timeout per invocation. |
| `env` | Additional environment variables injected for testing. |
| `adapter.type` | Test-specific adapter: `"function"` (in-process) or `"sdk"` (delegate to SDK runner). |
| `adapter.function_ref` | Module:function for in-process invocation (type=function). |
| `adapter.sdk_adapter` | SDK adapter identifier (type=sdk), e.g. `"langgraph"`. |

### Migration from v0.2.0

The v0.2.0 AUT supported `adapter.type: "http"`, `"grpc"`, `"stdio"`, `"docker"`, `"oci"`. These types are **removed in v0.3.0** — they duplicate `runtime.execution` backends. Test invocation for those backends is derived automatically.

| v0.2.0 (old) | v0.3.0 (new) |
|---|---|
| `aut.id: "my-aut"` | Remove — AUT is a singleton |
| `aut.adapter.type: "http"` | Remove — derived from runtime |
| `aut.adapter.endpoint: "..."` | `aut.endpoint: "..."` |
| `aut.adapter.timeout_seconds: 30` | `aut.timeout_seconds: 30` |
| `aut.adapter.type: "function"` | `aut.adapter.type: "function"` (unchanged) |
| `aut.adapter.type: "sdk"` | `aut.adapter.type: "sdk"` (unchanged) |

---

## Checkers

Checkers are **fast-fail pre-filters** — lightweight assertions that run before any evaluator. They produce a boolean (`passed`/`failed`), not a score. A failed checker stops the scenario immediately.

```yaml
x_testing:
  checkers:
    - id: "response-schema"
      type: "json_schema"
      schema:
        type: object
        required: [invoice_id, total_amount, status]

    - id: "no-debug-output"
      type: "regex"
      pattern: "TRACE|DEBUG"
      invert: true           # pass when pattern does NOT match

    - id: "has-content"
      type: "script"
      runtime: "python"
      inline: |
        def check(output):
            return isinstance(output.get("result"), str) and len(output["result"]) > 0
```

### Checker Types

| Type | Required fields | Description |
|---|---|---|
| `json_schema` | `schema` | Validate agent output against an inline JSON Schema |
| `regex` | `pattern` | Match output text against a regex; `invert: true` to assert absence |
| `script` | `runtime` + `inline` or `script_ref` | Custom assertion; entrypoint: `def check(output: dict) -> bool` |

### Checker Contract

```
input:  output dict (agent response)
output: bool — True = pass, False = fail
```

Checkers do NOT produce scores. A failed checker emits a reason from the runner (e.g., "checker 'response-schema' failed: required field 'invoice_id' missing") and skips all remaining evaluators for that scenario.

---

## Evaluators

Evaluators are **scored evaluation backends** — each produces an `EvaluationResult`:

```json
{
  "passed": true,
  "score": 0.92,
  "reason": "Response is grounded and cites 3 of 4 required sources.",
  "metadata": {},
  "evaluator_id": "semantic-accuracy",
  "evaluator_type": "llm_judge"
}
```

### Evaluator Types

All four types share the same wire contract:

```
input:  {output: dict, context: dict, scenario: dict}
output: {passed: bool, score: float | null, reason: str, metadata: dict}
```

#### `llm_judge`

```yaml
- id: "semantic-accuracy"
  type: "llm_judge"
  model: "gpt-4o"
  system_prompt: "Rate the response accuracy from 0.0 to 1.0."
  max_tokens: 256
  threshold: 0.8
```

Required: `model`. Optional: `system_prompt`, `max_tokens`, `rubric_ref`, `threshold`, `weight`.

`rubric_ref` accepts a path, URI, or git-pinned reference (`git+https://repo/path@sha`).

#### `script`

```yaml
- id: "field-checker"
  type: "script"
  runtime: "python"       # "python" | "bash" | "javascript"
  inline: |
    def evaluate(output, context):
        has_result = bool(output.get("result"))
        return {"passed": has_result, "score": 1.0 if has_result else 0.0, "reason": "result field present"}
  threshold: 1.0
```

Or reference an external file:
```yaml
  script_ref: "evals/check_billing.py"
  # git-pinned for reproducibility:
  script_ref: "git+https://github.com/acme/evals/check_billing.py@abc1234def"
```

Required: `runtime` + (`inline` or `script_ref`). Optional: `threshold`, `weight`.

**Git-pinned `script_ref`**: The runner clones the repo at the specified SHA, verifies `git rev-parse HEAD == sha`, then executes. SHA verification is mandatory — runners MUST NOT silently use a cached file without verification.

#### `container`

```yaml
- id: "schema-validator"
  type: "container"
  image: "ghcr.io/acme/billing-eval:v1.2"
  image_digest: "sha256:abc123...64hex"    # required for reproducibility
  timeout_seconds: 10
```

Required: `image`, `image_digest`.

**Container wire protocol (normative):**

- `input_format: "json_stdin"` (default): runner writes `{"output": <agent_output>, "context": <scenario_context>}` as a single JSON line to container stdin, then closes stdin.
- `output_format: "json_stdout"` (default): runner reads container stdout as a single JSON object: `{"passed": bool, "score": float | null, "reason": string}`. Additional keys are surfaced as `metadata`.
- `"file"` variants: runner chooses two host temp paths, bind-mounts them (`docker run -v /tmp/adp_in.json:/adp_input.json -v /tmp/adp_out.json:/adp_output.json`), injects `$ADP_INPUT_FILE` and `$ADP_OUTPUT_FILE` env vars. Container reads from `$ADP_INPUT_FILE`, writes output JSON to `$ADP_OUTPUT_FILE`. Runner reads host-side output file after exit; cleans up regardless of exit code.
- **Exit code semantics**: exit 0 = evaluator ran (pass/fail from JSON `passed` field). Exit non-zero = evaluator errored → `EvaluationResult(passed=False, score=None, reason="container exited with code N")`.
- `timeout_seconds` exceeded → runner kills container, treats as error result.

#### `deterministic`

```yaml
- id: "fn-eval"
  type: "deterministic"
  function_ref: "acme.evals:check_invoice_format"
```

Required: `function_ref` (`module:function`). The function receives `(output: dict, context: dict)` and returns `bool` or a result dict.

### `judges[]` — Deprecated Alias

`x_testing.judges[]` is a deprecated alias for `x_testing.evaluators[]`. Existing `judges[]` entries are interpreted as `type: "llm_judge"` entries. When only `judges[]` is present (no `evaluators[]`), validators emit: `WARNING: x_testing.judges[] is deprecated; migrate to x_testing.evaluators[]`.

When both `evaluators[]` and `judges[]` are present, `evaluators[]` wins on id collision — a `judges[]` entry with the same `id` as an `evaluators[]` entry is silently dropped.

---

## Parameters

```yaml
x_testing:
  parameters:
    max_concurrency: 4
    timeout_per_scenario_ms: 30000
    retry_on_failure: false
    max_retries: 0
```

| Field | Default | Description |
|---|---|---|
| `max_concurrency` | 1 | Max parallel scenario invocations |
| `timeout_per_scenario_ms` | — | Per-scenario wall-clock timeout |
| `retry_on_failure` | false | Retry failed scenarios |
| `max_retries` | 0 | Max retries when `retry_on_failure: true` |

---

## Full Example

```yaml
x_testing:
  aut:
    description: "Staging override for billing agent"
    endpoint: "https://staging.billing.acme.example/invoke"
    timeout_seconds: 30
    env:
      BILLING_MODE: "test"

  evaluators:
    - id: "semantic-accuracy"
      type: "llm_judge"
      model: "gpt-4o"
      system_prompt: "Rate the billing response accuracy from 0.0 to 1.0."
      threshold: 0.8

    - id: "schema-validator"
      type: "container"
      image: "ghcr.io/acme/billing-eval:v1.2"
      image_digest: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      timeout_seconds: 10

    - id: "field-checker"
      type: "script"
      runtime: "python"
      script_ref: "git+https://github.com/acme/evals/billing/check_fields.py@abc1234"
      threshold: 1.0

  checkers:
    - id: "response-schema"
      type: "json_schema"
      schema:
        type: object
        required: [invoice_id, total_amount, status]
    - id: "no-debug-output"
      type: "regex"
      pattern: "TRACE|DEBUG"
      invert: true

  parameters:
    max_concurrency: 4
    timeout_per_scenario_ms: 30000
    retry_on_failure: false
```

---

## Schema Reference

- `schemas/testing.schema.json` — standalone schema for `x_testing` block validation
- `schemas/adp.schema.json` — `x_testing` is an extension key (open; validated separately)
- `schemas/evaluation.schema.json` — `evaluator_ref` field links metrics to `x_testing.evaluators[]`

See `examples/testing/billing-test.yaml` for a complete `x_testing` usage example.
