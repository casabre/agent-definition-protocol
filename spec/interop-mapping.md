# Interoperability Mapping (Informative)

This document maps ADP concepts to adjacent specifications to ease integration.

---

## A2A Agent Card

ADP is the source of truth for agent identity and capabilities. The A2A AgentCard is a **read projection** derived from ADP fields. Inline declaration (`interop.a2a.agent_card`) gives full control; absent declaration falls back to automatic derivation.

### Derivation Table

| A2A Agent Card field | Derived from ADP field | Notes |
|---|---|---|
| `name` | `adp.name` | Direct mapping |
| `description` | `adp.description` | Direct mapping |
| `url` | `interop.a2a.card_url` | URL where the card is served |
| `version` | `adp_version` | ADP schema version |
| `capabilities.streaming` | `adp.streaming.enabled` | Derived if not set explicitly in `agent_card.capabilities` |
| `authentication` | `interop.a2a.agent_card.authentication` | Not auto-derived; must be explicit |
| `skills[]` (public) | `adp.skills[]` where `visibility = "public"` (default) | Skills visible to all callers |
| `skills[]` (auth extension) | `adp.skills[]` where `visibility = "authenticated"` | Skills visible to authenticated callers only |

ADP uses `snake_case`; A2A wire format uses `camelCase`. Runners MUST translate when serving the card:
- `input_modes` → `inputModes`
- `output_modes` → `outputModes`
- `documentation_url` → `documentationUrl`

### Serving Pattern

The agent card SHOULD be served at `interop.a2a.card_url` (typically `/.well-known/agent.json`).

```yaml
interop:
  a2a:
    card_url: "https://agent.example.com/.well-known/agent.json"
    agent_card:
      name: "My Agent"
      url: "https://agent.example.com/"
      version: "1.0.0"
      capabilities:
        streaming: true
      authentication:
        schemes: ["Bearer"]
```

---

## A2A Authenticated Extension Card

The authenticated extension card is a **delta** over the public card. It is returned to authenticated callers only and computed at serve time — not persisted.

### Merge Semantics

- **Arrays** (skills[], schemes[], scopes[]): additive — public array + extension array, deduplicated by `id`
- **Scalars** (capabilities booleans): extension value overrides public value
- **Extension-only fields** (`push_notification_config`): present in authenticated card only

### Skill Visibility Routing

`skills[].visibility` routes each skill to the correct card:

```yaml
skills:
  - id: "basic-query"
    name: "Answer metrics questions"
    visibility: "public"       # default — appears in public card
  - id: "export-raw"
    name: "Export raw telemetry"
    visibility: "authenticated" # appears only in authenticated extension
```

If `agent_card.skills` is declared inline, it **overrides** the derived public skill list entirely. If `authenticated_extension.skills` is declared inline, it is **merged** with skills derived from `visibility: "authenticated"` entries (union by `id`).

### Runner Responsibilities

A runner serving the agent card at `card_url` SHOULD:

1. Serve the public `agent_card` (or derived equivalent) to unauthenticated requests.
2. Authenticate the request using `agent_card.authentication.schemes`.
3. On successful authentication, merge `authenticated_extension` over the public card (additive arrays, override scalars) and serve the merged result.
4. NOT persist the merged result — compute it at serve time.

Runners that do not support authentication MUST serve only the public card regardless of any `authenticated_extension` declaration.

### Example

```yaml
interop:
  a2a:
    card_url: "https://agent.example.com/.well-known/agent.json"
    agent_card:
      name: "Analytics Agent"
      url: "https://agent.example.com/"
      version: "1.0.0"
      capabilities:
        streaming: true
        push_notifications: false
      authentication:
        schemes: ["Bearer"]
      default_input_modes: ["text/plain"]
      default_output_modes: ["application/json"]
    authenticated_extension:
      capabilities:
        push_notifications: true
      authentication:
        schemes: ["Bearer", "oauth2"]
        oauth2:
          authorization_url: "https://auth.example.com/oauth2/authorize"
          token_url: "https://auth.example.com/oauth2/token"
          scopes: ["agent:read", "agent:export"]
      push_notification_config:
        endpoint: "https://agent.example.com/notifications"
        auth_scheme: "Bearer"
```

---

## OpenTelemetry Metrics

- **Latency**: `evaluation.metrics[].telemetry.metric` SHOULD reference OTel semantic conventions (e.g., `rpc.server.duration`, `http.server.duration`).
- **Cost/tool usage**: `tool.calls`, `llm.tokens` SHOULD map to custom OTel metrics with clear units (tokens, USD).
- **Namespaces**: Use OTel attributes `agent.id`, `deployment.environment`, and `runtime.backend.id` to correlate telemetry with ADP definitions.
- **Flow tracing**: Include `flow.id` and `flow.node.id` span attributes for flow execution observability.

---

## Recommended Practices (Informative)

- **A2A**: If `interop.a2a.ref` is provided (legacy pattern), ensure it resolves to a valid AgentCard and that ids match. Prefer inline `agent_card` for v0.3.0 manifests.
- **OTel**: Include `agent.id`, `deployment.environment`, and `runtime.backend.id` as OTel resource attributes to correlate traces with ADP runtime entries.
- **HTTP/MCP/SQL tools**: Surface endpoint and connection info as OTel resource attributes where applicable.
- **Skill visibility**: Default `visibility: "public"` means all skills appear in the public card. Only explicitly set `visibility: "authenticated"` for skills that should be hidden from unauthenticated callers.
