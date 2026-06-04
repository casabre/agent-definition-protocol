# ADP ↔ AgentSpec Interoperability Mapping

**Status**: Informative  
**Applies to**: ADP v0.1.0+  
**AgentSpec reference**: oracle/agent-spec (versions 25.4.x – 26.x)

---

## Positioning

ADP and Oracle AgentSpec are complementary, not competing, specifications.

**AgentSpec** is a portable *agent composition language*: it describes agents and flows with strongly-typed inputs/outputs, explicit data-flow wiring between node ports, and named LLM configuration components. Its scope ends at the definition boundary — execution is delegated to a separate runtime adapter (WayFlow, LangGraph, AutoGen, CrewAI).

**ADP** is a *full-stack agent lifecycle manifest*: it wraps a flow graph with runtime backend configuration, evaluation suites, governance controls, OCI packaging, and deployment targets. ADP's flow graph (AFG) is intentionally thinner than AgentSpec's — it uses implicit `state.context[node.id]` passing rather than explicit port wiring, keeping manifests concise at the cost of some structural expressiveness.

The `interop.agentspec` block lets an ADP manifest declare its AgentSpec counterpart and supply identity bridges so that tooling can convert between the two formats without losing component identity.

---

## Concept Translation

| ADP concept | AgentSpec equivalent | Notes |
|---|---|---|
| `id` (URI-like string) | `id` (UUID) | Different identity schemes. Use `component_id` in `interop.agentspec` to bridge. |
| `runtime.execution[]` | `llm_config` (named component) | ADP models LLM provider as a runtime backend; AgentSpec models it as a named LLM config object. Use `llm_map` to bridge. |
| `flow.graph` (AFG) | `component_type: Flow` | Both are directed graphs with nodes and edges. Structural differences noted below. |
| `flow.graph.nodes[kind=llm]` | `AgentNode` | ADP's `llm` node is the closest equivalent to AgentSpec's conversational `AgentNode`. |
| `flow.graph.nodes[kind=tool]` | `ToolNode` | Maps to a ToolNode wrapping a `ServerTool` or `McpTool`. |
| `flow.graph.nodes[kind=router]` | `BranchingNode` | ADP uses `condition` expressions on edges; AgentSpec uses a `mapping` dict on BranchingNode. |
| `flow.graph.nodes[kind=input]` | `InputMessageNode` | Entry point. |
| `flow.graph.nodes[kind=output]` | `OutputMessageNode` | Terminal node. |
| `flow.graph.nodes[kind=subflow]` | Nested `Flow` via `$component_ref` | Subflow composition is supported in both specs with different reference mechanisms. |
| `flow.graph.edges[].from/to` | `control_flow_connections[].from_node/to_node` | Both describe execution order between nodes. |
| `tools.mcp_servers[]` | `McpTool` component | ADP's `mcp_servers` catalog maps to AgentSpec `McpTool`; the tool is then referenced from a `ToolNode`. |
| `tools.http_apis[]` | `ServerTool` (HTTP variant) | AgentSpec's `ServerTool` covers HTTP-callable functions. |
| `skills[]` | `Agent.inputs[]` / `Agent.outputs[]` | ADP skills declare capabilities exposed to callers; AgentSpec inputs/outputs type the agent's I/O contract. |
| `prompts.system` | `Agent.system_prompt` | Direct equivalent; AgentSpec uses `{{variable}}` template syntax for interpolation. |

---

## ADP-Only Concepts (no AgentSpec equivalent)

These ADP concepts have no counterpart in AgentSpec. They are not translated and are not represented in any AgentSpec-generated file.

| ADP concept | Why AgentSpec has no equivalent |
|---|---|
| `evaluation` suites and promotion policies | AgentSpec defines flow structure, not quality gates or CI/CD promotion. |
| `governance` (guardrails, cost limits, interrupts) | Out of scope for a composition language; belongs to the runtime or platform layer. |
| `deployment` targets (dev / staging / prod) | AgentSpec leaves deployment entirely to the runtime or the host platform. |
| OCI packaging (`adpkg`) | Packaging and signing is not part of AgentSpec's model. |
| `runtime.execution[].backend` (docker/wasm/python/ts) | AgentSpec treats execution backend as a runtime adapter concern, not a manifest concern. |
| `memory` stores and retention policies | AgentSpec has no native memory model. |
| `observability` and `telemetry` | Not in AgentSpec scope. |

---

## AgentSpec-Only Concepts (no ADP equivalent)

These AgentSpec concepts are not currently modeled in ADP. They represent the primary structural gap between the two specs.

| AgentSpec concept | Status in ADP |
|---|---|
| `data_flow_connections` — explicit typed port-to-port wiring between node inputs and outputs | **Not modeled.** ADP uses implicit `state.context[node.id]` passing. This is the primary structural gap. Considered a candidate for a future optional AFG extension. |
| `$referenced_components` / `$component_ref` — component sharing across a single document | **Not modeled.** ADP uses `subagents[]` and `import` for cross-manifest composition but has no intra-document component reuse pattern. |
| Typed `inputs[]` / `outputs[]` on every component (JSON Schema–typed port definitions) | **Not modeled at the flow level.** ADP `skills[]` approximate this at the agent boundary only. |
| Named `llm_config` components (reusable, named LLM definitions) | **Not modeled as a first-class concept.** ADP binds LLM provider configuration to `runtime.execution[]` entries. |

---

## `interop.agentspec` Field Reference

All fields are optional. The block itself is optional.

```yaml
interop:
  agentspec:
    ref: "./agent.agentspec.yaml"
    version: "26.2.0"
    component_type: "Agent"
    component_id: "bdd2369b-82e6-488f-be2c-44f05b244cab"
    runtime_adapters: ["wayflow", "langgraph"]
    node_map:
      planner: "3a5bf0c0-9f28-47d8-99b4-be7da6a531c8"
      tool_call: "fc98ab56-0d30-4bdd-a84d-cab76d75e575"
    llm_map:
      - backend_id: "python-backend"
        agentspec_id: "3a5bf0c0-9f28-47d8-99b4-be7da6a531c8"
        agentspec_type: "OpenAiConfig"
```

| Field | Type | Description |
|---|---|---|
| `ref` | string | URI or relative file path to the AgentSpec YAML/JSON configuration file. |
| `version` | string | AgentSpec calendar version (pattern `YY.M.patch`, e.g., `26.2.0`). |
| `component_type` | `"Agent"` \| `"Flow"` | The AgentSpec root component type this manifest maps to. |
| `component_id` | UUID string | UUID of the root AgentSpec component. Preserved for round-trip fidelity. |
| `runtime_adapters` | string[] | AgentSpec-compatible runtime adapters that can execute this configuration (e.g., `wayflow`, `langgraph`, `autogen`, `crewai`). |
| `node_map` | object | Maps ADP flow node IDs (keys) to AgentSpec component UUIDs (values). All keys MUST match a node `id` in `flow.graph.nodes`. |
| `llm_map` | object[] | Maps ADP runtime execution backend IDs to AgentSpec LLM config components. `backend_id` MUST match an `id` in `runtime.execution[]`. |

---

## Worked Example

### ADP manifest side

```yaml
adp_version: "0.3.0"
id: "agent.acme.writing-assistant"
name: "Writing Assistant"

runtime:
  execution:
    - id: "primary"
      backend: "python"
      entrypoint: "acme_agents.writing:app"
      environment:
        python_version: "3.12"

flow:
  id: "acme.writing.flow"
  graph:
    nodes:
      - id: "input"
        kind: "input"
      - id: "assistant"
        kind: "llm"
        model_ref: "primary"
      - id: "output"
        kind: "output"
        output_ref: "assistant"
    edges:
      - { from: "input", to: "assistant" }
      - { from: "assistant", to: "output" }
    start_nodes: ["input"]
    end_nodes: ["output"]

tools:
  mcp_servers:
    - id: "synonyms"
      description: "Given a word, return a list of synonyms"
      transport: "http"
      endpoint: "https://tools.acme.example/synonyms"

evaluation: {}

interop:
  agentspec:
    ref: "./writing-assistant.agentspec.yaml"
    version: "26.2.0"
    component_type: "Agent"
    component_id: "bdd2369b-82e6-488f-be2c-44f05b244cab"
    runtime_adapters: ["wayflow"]
    node_map:
      assistant: "fc98ab56-0d30-4bdd-a84d-cab76d75e575"
    llm_map:
      - backend_id: "primary"
        agentspec_id: "3a5bf0c0-9f28-47d8-99b4-be7da6a531c8"
        agentspec_type: "OpenAiConfig"
```

### Corresponding AgentSpec counterpart (`writing-assistant.agentspec.yaml`)

```yaml
component_type: Agent
id: bdd2369b-82e6-488f-be2c-44f05b244cab
name: Writing Assistant
inputs:
  - title: user_name
    type: string
outputs: []
llm_config:
  component_type: OpenAiConfig
  id: 3a5bf0c0-9f28-47d8-99b4-be7da6a531c8
  name: openai-primary
  model_id: gpt-4o
system_prompt: "You are a helpful writing assistant. Welcome {{user_name}}."
tools:
  - component_type: ServerTool
    id: fc98ab56-0d30-4bdd-a84d-cab76d75e575
    name: get_synonyms
    description: Given a word, return the list of synonyms
    inputs:
      - title: word
        type: string
    outputs:
      - title: synonyms
        items:
          type: string
        type: array
agentspec_version: 26.2.0
```

The ADP manifest drives execution, evaluation, governance, and packaging. The AgentSpec file is the portable composition artifact for AgentSpec-compatible runtimes. The `component_id`, `node_map`, and `llm_map` fields in `interop.agentspec` link the two without embedding either format inside the other.

---

## Round-Trip Notes

When converting an AgentSpec configuration to an ADP manifest (or vice versa), use `component_id` and `node_map` to preserve identity:

- **AgentSpec → ADP**: Store the AgentSpec root component UUID in `component_id`. For each AgentSpec component that becomes a flow node, record its UUID as the value in `node_map[<adp_node_id>]`. Store AgentSpec LLM config UUIDs in `llm_map[].agentspec_id`.

- **ADP → AgentSpec**: Read `component_id` to restore the AgentSpec UUID rather than generating a new one. Reconstruct AgentSpec `data_flow_connections` from ADP's implicit `state.context` passing — ADP does not store explicit port wiring, so adapters must infer data flow from node kinds and edge structure.

The `data_flow_connections` gap means ADP → AgentSpec conversion is lossy at the data-wiring level unless the original AgentSpec file is retained alongside the ADP manifest (which is the recommended practice — use `ref` to point to it).
