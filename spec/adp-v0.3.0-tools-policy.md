# ADP v0.3.0 Tools Policy Specification

**Agent Definition Protocol — Tooling Policy v0.3.0**

> **Version**: v0.3.0  
> **Status**: Draft  
> **Editor**: ADP Working Group  
> **Last Updated**: 2026-05-25  

---

## Abstract

This document adds a `policy` sub-object to every tool definition in `tools.mcp_servers[]`, `tools.http_apis[]`, and `tools.sql_functions[]`. All fields are optional; no existing manifests break. The policy covers retry, timeout, cache, and rate-limit configurations.

---

## Table of Contents

1. [Policy Schema](#1-policy-schema)
2. [Load Strategy](#2-load-strategy)
3. [ESP Amendment](#3-esp-amendment)
4. [Cache Key Syntax](#4-cache-key-syntax)
5. [Semantic Validation Check](#5-semantic-validation-check)

---

## 1. Policy Schema

All tool types now support an optional `policy` object and `load_strategy` field:

```yaml
tools:
  http_apis:
    - id: "weather-api"
      base_url: "https://api.weather.example"
      description: "Get weather data for a city"
      auth:
        scheme: "api_key"
        env_var: "WEATHER_API_KEY"
      load_strategy: "eager"  # eager | lazy | on_demand
      policy:
        retry:
          max_attempts: 3
          backoff: "exponential"          # fixed | linear | exponential
          backoff_base_ms: 500
          max_delay_ms: 10000
          retryable_status_codes: [429, 502, 503, 504]
        timeout_ms: 8000
        rate_limit:
          requests_per_minute: 60
          burst: 10
          on_limit_exceeded: "queue"      # queue | fail | warn
        cache:
          enabled: true
          ttl_seconds: 300
          key_fields: ["params.city"]     # dot-path without $ prefix
          scope: "agent"                  # agent | session | user
```

### Policy Fields

| Field | Type | Default | Description |
|---|---|---|---|
| `retry.max_attempts` | integer | 3 | Maximum retry attempts |
| `retry.backoff` | enum | `"exponential"` | Backoff strategy: `fixed`, `linear`, `exponential` |
| `retry.backoff_base_ms` | integer | 500 | Base delay in milliseconds |
| `retry.max_delay_ms` | integer | 10000 | Maximum delay in milliseconds |
| `retry.retryable_status_codes` | array | [] | HTTP status codes to retry on |
| `timeout_ms` | integer | - | Timeout in milliseconds (required) |
| `rate_limit.requests_per_minute` | integer | - | Rate limit requests per minute |
| `rate_limit.burst` | integer | 0 | Burst capacity |
| `rate_limit.on_limit_exceeded` | enum | `"queue"` | Action: `queue`, `fail`, `warn` |
| `cache.enabled` | boolean | false | Enable caching |
| `cache.ttl_seconds` | integer | 0 | Cache TTL in seconds |
| `cache.key_fields` | array | [] | Fields to include in cache key (dot-path notation) |
| `cache.scope` | enum | `"agent"` | Cache scope: `agent`, `session`, `user` |

---

## 2. Load Strategy

Progressive/JIT tool loading with three strategies:

| Strategy | Description | When Schema Shown |
|---|---|---|
| `eager` | Full schema loaded at invocation start | Always shown in system prompt |
| `lazy` | Schema loaded on first actual use by the flow | Not shown in initial system prompt |
| `on_demand` | Only brief description shown upfront; full schema injected when LLM emits tool-request signal | Description shown in system prompt; schema injected dynamically |

**Important**: For `load_strategy: "on_demand"`, runners MUST expose `tool.description` in the system prompt even when the full schema is withheld. This allows the LLM to know the tool exists.

**Example with on_demand:**
```yaml
tools:
  http_apis:
    - id: "specialized-api"
      description: "A rarely used API for special cases"
      base_url: "https://api.special.example"
      load_strategy: "on_demand"
      policy:
        timeout_ms: 5000
```

The LLM sees only the description initially. When it requests `specialized-api`, the runner injects the full schema.

---

## 3. ESP Amendment

Policy is applied as a wrapper **around** the existing Tool Invocation Semantics:

1. Check `policy.rate_limit` — if exceeded, queue/fail/warn per `on_limit_exceeded`
2. Check `policy.cache` — on cache hit, write to `tool_responses` with `"cached": true` and skip the actual call
3. Apply `policy.timeout_ms` — abort invocation if exceeded; treat as transient failure
4. On failure: apply `policy.retry` (exponential backoff per config)
5. Cache result on success (if `policy.cache.enabled`)

### Cache Hits in `tool_responses`

```json
{
  "tool_responses": {
    "weather-api-call": {
      "params": {"city": "Berlin"},
      "response": {"temp": 18},
      "cached": true,
      "cache_hit_at": "2026-05-25T08:00:00Z"
    }
  }
}
```

---

## 4. Cache Key Syntax

**Cache key syntax uses dot-path notation without `$` prefix.**

- `params.city` means the `city` field inside the invocation `params` object
- This is consistent with ADP's existing field reference style in edge conditions and ESP state paths
- JSONPath (`$.params.city`) is NOT used

**Examples:**
```yaml
# Single field
key_fields: ["params.city"]

# Multiple fields
key_fields: ["params.city", "params.country"]

# Nested fields
key_fields: ["params.location.city", "params.location.country"]
```

---

## 5. Semantic Validation Check

- **Check 17**: `policy.cache.key_fields[]` entries MUST use dot-path notation (no `$` prefix, no bracket notation). Validators emit error for malformed paths.

**Invalid examples (fail Check 17):**
```yaml
# WRONG: Uses $ prefix (JSONPath)
key_fields: ["$.params.city"]

# WRONG: Uses bracket notation
key_fields: ["params['city']"]

# WRONG: Uses array index with bracket
key_fields: ["params.cities[0]"]
```

**Valid examples (pass Check 17):**
```yaml
# CORRECT: Simple dot-path
key_fields: ["params.city"]

# CORRECT: Nested dot-path
key_fields: ["params.location.city"]
```

---

## Appendix: Sandbox Tools

The `tools.sandbox[]` type is defined separately in [`adp-v0.3.0-sandbox.md`](adp-v0.3.0-sandbox.md) and does not use `load_strategy` or `policy` fields. Sandbox tools are invoked via explicit `tool_ref` in flow nodes, not loaded into the system prompt.

---

**Expert skills applied**: `role-senior-software-engineer`, `role-senior-agentic-ai-developer`

*This document is part of the ADP v0.3.0 specification. See [adp-v0.3.0.md](adp-v0.3.0.md) for the master specification.*
