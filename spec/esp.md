# Execution Semantics Profile (ESP) for ADP v0.2.0

**Status**: Draft / Proposal  
**Version**: 0.2.0  
**Date**: 2024

## Introduction & Scope

The Execution Semantics Profile (ESP) defines **how ADP agents are interpreted and executed** by conformant runners. ESP fills semantic gaps identified in ADP v0.1.0 by specifying execution behavior without mandating implementation details.

### What ESP Is

- **Semantic specification**: Defines *what* must happen during execution, not *how* it is implemented
- **Framework-neutral**: Compatible with LangGraph, LangChain, CrewAI, and custom frameworks
- **Additive**: Extends ADP v0.1.0 without breaking existing manifests
- **Optional but recommended**: Runners MAY implement ESP for deterministic, interoperable behavior

### What ESP Is Not

- **Not a runtime**: ESP does not provide executable code or runtime implementations
- **Not framework-specific**: ESP does not prescribe LangGraph, LangChain, or any specific framework
- **Not implementation-prescriptive**: ESP does not mandate threads, event loops, or scheduling algorithms
- **Not a replacement**: ESP does not replace `spec/runtime.md` or `spec/flow.md`; it adds execution semantics

### Scope

ESP defines semantics for:

- **Flow graph execution**: How nodes execute and edges are traversed
- **State management**: How data flows between nodes
- **Tool integration**: How tools are bound to nodes and invoked
- **Model and prompt resolution**: How references resolve to concrete implementations
- **Error handling**: Basic failure semantics and error propagation

ESP does **not** define:

- **Runtime backend execution**: How Docker/WASM/Python backends execute (covered by `spec/runtime.md`)
- **Tool protocol details**: MCP protocol, HTTP API contracts, SQL dialects (referenced, not specified)
- **LLM provider APIs**: OpenAI, Anthropic, or other provider-specific APIs (abstracted via model references)
- **Advanced features**: Streaming, checkpointing, parallelism (marked as optional capabilities)

### Relationship to ADP v0.1.0

- ADP v0.1.0 defines **structure** (what fields exist, what they contain)
- ESP defines **semantics** (how those fields are interpreted during execution)
- ADP v0.1.0 manifests remain valid; ESP adds optional execution guidance
- Runners MAY implement ESP semantics for v0.1.0 manifests without manifest changes

### Conformance

A runner is **ESP-conformant** if it correctly implements ESP semantics for flow execution, state management, tool binding, and reference resolution. ESP conformance is **optional**; runners MAY implement partial ESP support or custom execution models.

## Terminology

This section defines key terms used throughout ESP. Terms align with existing ADP specifications where applicable.

### Core Concepts

- **Runner**: An implementation that executes ADP agents. A runner interprets ADP manifests and executes flow graphs according to ESP semantics. Runners MAY be standalone executables, libraries, or services.

- **Run**: A single execution of an agent flow graph. A run begins with an invocation and proceeds through flow nodes until termination (reaching an output node or failure).

- **Invocation**: External input that triggers a run. An invocation provides initial input data and MAY include metadata (user ID, session ID, etc.).

- **State**: A data structure passed between flow nodes during a run. State contains inputs, working context, tool responses, and optional memory. See [State Model](#state-model) for structure.

- **Node execution**: The process of executing a single flow node. Node execution reads from state, performs node-specific operations, and updates state.

- **Edge traversal**: The process of following an edge from one node to another. Edge traversal MAY be conditional based on edge conditions.

- **Edge condition**: An expression that determines whether an edge is traversable. Conditions are evaluated against current state.

### Flow Graph Terms

- **Start node**: A node listed in `flow.graph.start_nodes[]`. A run begins execution at start nodes.

- **End node**: A node listed in `flow.graph.end_nodes[]`. Reaching an end node terminates a run successfully.

- **Node kind**: The type of a flow node (`input`, `output`, `llm`, `tool`, `router`, `retriever`, `evaluator`, `subflow`). Node kind determines execution semantics.

- **Tool node**: A flow node with `kind: "tool"`. Tool nodes invoke external capabilities.

- **LLM node**: A flow node with `kind: "llm"`. LLM nodes invoke language models.

### Tool and Reference Terms

- **Tool binding**: The connection between a tool node and a tool definition (`tools.mcp_servers[]`, `tools.http_apis[]`, or `tools.sql_functions[]`). Binding is established via `tool_ref`.

- **Tool reference** (`tool_ref`): A field on tool nodes that references a tool ID. `tool_ref` MUST match an ID from `tools.*` arrays.

- **Model reference** (`model_ref`): A field on LLM nodes that references a model configuration. Model references resolve to concrete model identifiers or configurations.

- **Prompt reference** (`system_prompt_ref`, `prompt_ref`): Fields that reference prompt templates. Prompt references resolve to prompt text, possibly with variable substitution.

### State Terms

- **Inputs**: External data provided to a run via invocation. Inputs are immutable during a run.

- **Context**: Working data accumulated during a run. Context is updated by nodes and passed between nodes.

- **Memory**: Optional persisted data accessible across runs. Memory is runner-dependent and MAY include vector stores, databases, or caches.

- **Tool responses**: Accumulated results from tool invocations. Tool responses are indexed by tool ID and invocation sequence.

### Execution Terms

- **Observable ordering**: The order in which node executions and state updates are observable to external systems (telemetry, logging, evaluation). Observable ordering MUST be preserved even if internal execution is parallel or asynchronous. Example: If node A executes before node B, telemetry/logging MUST show A's execution before B's, even if internal execution is parallel.

- **Node readiness**: A node is ready for execution when all required input edges have been traversed and conditions (if any) are satisfied.

- **Run termination**: A run terminates when it reaches an end node (success) or encounters an unrecoverable failure (failure).

- **Branch failure**: In flows with multiple paths, a branch MAY fail independently without terminating the entire run if other paths remain viable.

- **Branch viability**: A branch is viable if it has at least one traversable edge leading to an end node or another viable branch.

- **State merge**: When nodes execute in parallel, their state updates MUST be merged correctly. Merge semantics: last-writer-wins for conflicting fields, array concatenation for `tool_responses`.

- **Node requires all inputs**: Some nodes (e.g., `router` with `strategy: "all"`) require all incoming edges to be traversed before execution. Default behavior: node executes when any incoming edge is traversed.

### Reference Resolution Terms

- **Resolution**: The process of converting a reference (`model_ref`, `prompt_ref`, `tool_ref`) into a concrete implementation or value.

- **Resolution failure**: When a reference cannot be resolved (missing definition, invalid ID, etc.). Runners MUST fail gracefully on resolution failure.

- **Model registry**: A mapping from model references to concrete model identifiers (e.g., `"primary"` → `"gpt-4"`). Model registries are runner-specific.

### Error Terms

- **Node failure**: When a node execution encounters an error that prevents successful completion.

- **Permanent failure**: A failure that cannot be recovered through retry (e.g., invalid input, missing resource, authentication failure).

- **Transient failure**: A failure that MAY be recoverable through retry (e.g., network timeout, rate limit).

- **Error propagation**: How errors from one node affect subsequent nodes and the overall run.

## Execution Model Overview

ESP defines an abstract execution model for ADP flow graphs. This model specifies *what* must happen during execution without prescribing *how* it is implemented.

### Graph Traversal Model

A flow graph (`flow.graph`) executes as a **directed graph traversal**. Execution proceeds from start nodes (`flow.graph.start_nodes[]`) through intermediate nodes to end nodes (`flow.graph.end_nodes[]`).

**Key principles**:

1. **Node readiness**: A node is ready for execution when:
   - All incoming edges from start nodes or previously executed nodes have been traversed, OR
   - For nodes with multiple incoming edges, at least one incoming edge has been traversed (unless the node requires all inputs)
   - Any edge conditions on traversed edges evaluate to `true` (see [Edge Condition Evaluation](#edge-condition-evaluation))

2. **Node execution**: When a node is ready, the runner MUST execute it. Node execution:
   - Reads from the current state
   - Performs node-specific operations (see [Flow Node Semantics](#flow-node-semantics))
   - Updates state
   - Marks the node as executed

3. **Edge traversal**: After a node executes, the runner evaluates outgoing edges:
   - For each outgoing edge, if an edge condition exists, the runner evaluates it against current state
   - If the condition evaluates to `true` (or no condition exists), the edge is traversable
   - Traversable edges lead to target nodes that become ready for execution

4. **Termination**: Execution terminates when:
   - An end node (`flow.graph.end_nodes[]`) is reached → **successful termination**
   - A permanent failure occurs and no viable paths remain → **failure termination**
   - All active paths reach dead ends (no traversable edges) → **failure termination**

### Observable Ordering Constraints

ESP requires that **observable ordering** be preserved. Observable ordering means:

- **Node execution order**: The order in which nodes execute MUST be consistent with the graph structure. If node A must execute before node B (due to edges), then A's execution MUST be observable before B's execution.

- **State update order**: State updates from node executions MUST be observable in execution order. If node A updates state before node B, then A's updates MUST be visible to B.

- **External observability**: Telemetry, logging, and evaluation systems MUST observe node executions and state updates in the order they occur, even if internal execution is parallel or asynchronous.

**Implementation flexibility**: Runners MAY execute nodes in parallel, use asynchronous scheduling, or optimize execution order, but MUST preserve observable ordering constraints.

### Execution Model Constraints

ESP does **not** mandate:

- **Concurrency model**: Runners MAY use threads, processes, coroutines, event loops, or sequential execution
- **Scheduling algorithm**: Runners MAY use FIFO, priority queues, or custom schedulers
- **State storage**: Runners MAY store state in memory, databases, or distributed systems
- **Error recovery**: Runners MAY implement retries, circuit breakers, or other recovery mechanisms (subject to [Error & Failure Semantics](#error--failure-semantics))

ESP **does** mandate:

- **Graph structure compliance**: Execution MUST respect the graph structure (nodes, edges, start/end nodes)
- **Node readiness**: Nodes MUST NOT execute before they are ready
- **State consistency**: State updates MUST be consistent with observable ordering
- **Termination conditions**: Runs MUST terminate according to termination rules

### Edge Condition Evaluation

Edges MAY have a `condition` field (string expression). When an edge is evaluated for traversal:

1. If no `condition` is present, the edge is traversable (always traversed)
2. If a `condition` is present, the runner evaluates it against current state
3. The condition expression MUST evaluate to a boolean value
4. If the condition evaluates to `true`, the edge is traversable
5. If the condition evaluates to `false`, the edge is not traversed

**Condition expression language**: ESP RECOMMENDS JSONPath expressions (RFC 9535 subset) for edge conditions:
- `$.context.status == "ready"` (string comparison)
- `$.context.count > 10` (numeric comparison)
- `$.inputs.user_id` (field access)

Runners MAY support alternative expression languages but SHOULD support JSONPath for interoperability. Runners MUST document their supported expression language and MUST fail gracefully if a condition cannot be evaluated (see [Error & Failure Semantics](#error--failure-semantics)).

### Multi-Path Execution

Flow graphs MAY have multiple paths (branches). ESP allows:

- **Parallel paths**: Multiple nodes MAY be ready simultaneously and MAY execute in parallel (subject to observable ordering)
- **Conditional paths**: Edge conditions determine which paths are taken
- **Branch failure**: One branch MAY fail without terminating the entire run if other branches remain viable

Runners MUST handle multi-path execution correctly, ensuring that:
- All viable paths are explored (unless explicitly terminated)
- Branch failures do not affect other branches (unless the failure is permanent and unrecoverable)
- Observable ordering is preserved across branches

### Example Execution Flow

Consider a simple flow:

```yaml
graph:
  nodes:
    - id: "input"
      kind: "input"
    - id: "process"
      kind: "llm"
    - id: "output"
      kind: "output"
  edges:
    - { from: "input", to: "process" }
    - { from: "process", to: "output" }
  start_nodes: ["input"]
  end_nodes: ["output"]
```

Execution proceeds as:

1. **Start**: Run begins at `input` node
2. **Input execution**: `input` node executes, initializes state with invocation data
3. **Edge traversal**: Edge from `input` to `process` is traversed (no condition)
4. **Process readiness**: `process` node becomes ready
5. **Process execution**: `process` node executes, invokes LLM, updates state
6. **Edge traversal**: Edge from `process` to `output` is traversed
7. **Output readiness**: `output` node becomes ready
8. **Output execution**: `output` node executes, returns final result
9. **Termination**: Run terminates successfully (end node reached)

## State Model

ESP defines a minimal state structure that MUST be preserved by all ESP-conformant runners. State is passed between nodes during a run and updated by node executions.

### State Structure

State is a JSON-like object with the following core fields:

```json
{
  "inputs": { ... },
  "context": { ... },
  "memory": { ... },
  "tool_responses": { ... }
}
```

#### Core Fields

- **`inputs`** (object, required): External invocation input. `inputs` is **immutable** during a run. It contains the initial data provided to the agent when the run begins. Runners MUST NOT modify `inputs` after initialization.

- **`context`** (object, required): Working scratchpad per run. `context` is **mutable** and accumulates data during execution. Nodes read from and write to `context`. `context` is run-scoped and does not persist across runs.

- **`memory`** (object, optional): Optional persisted information accessible across runs. `memory` is runner-dependent and MAY include:
  - Vector store references
  - Database connections
  - Cached data
  - Session state

Runners MAY omit `memory` if persistence is not supported. If present, `memory` MAY be read-only or read-write depending on runner capabilities.

- **`tool_responses`** (object, required): Accumulation of tool calls and outputs. `tool_responses` is indexed by tool ID and invocation sequence. Each tool invocation appends to `tool_responses[<tool_id>]` with:
  - Invocation parameters
  - Response data
  - Error information (if any)
  - Timestamp (optional, runner-dependent)

### State Passing Between Nodes

State is passed between nodes as follows:

1. **Initialization**: When a run begins, state is initialized with:
   - `inputs`: Set to invocation data
   - `context`: Set to empty object `{}` or initial context from `input` node
   - `memory`: Set to persisted memory (if available) or empty object `{}`
   - `tool_responses`: Set to empty object `{}`

2. **Node execution**: When a node executes:
   - Node reads from current state (typically `context`, `inputs`, `tool_responses`)
   - Node performs operations (LLM call, tool invocation, etc.)
   - Node updates state (typically `context` and/or `tool_responses`)

3. **State propagation**: After node execution, updated state is passed to subsequent nodes via edge traversal.

4. **State consistency**: Runners MUST ensure that state updates are consistent with observable ordering (see [Execution Model Overview](#execution-model-overview)). If nodes execute in parallel, state updates MUST be merged correctly.

### State Updates by Node Type

Each node type updates state according to its semantics:

- **`input`**: Initializes `inputs` and `context` from invocation data
- **`output`**: Reads from `context` to produce final result (does not update state)
- **`llm`**: Reads from `context`, invokes LLM, writes response to `context`
- **`tool`**: Reads from `context`, invokes tool, writes result to `tool_responses` and optionally `context`
- **`router`**: Reads from `context`, determines routing, may update `context` with routing decisions
- **`retriever`**: Reads from `context` and `memory`, queries vector store, writes results to `context`
- **`evaluator`**: Reads from `context`, runs evaluation metrics, writes results to `context`
- **`subflow`**: Executes subflow with subset of state, merges results back into `context`

See [Flow Node Semantics](#flow-node-semantics) for detailed per-node semantics.

### State Extensions

Runners MAY extend the state structure with additional fields (e.g., `metadata`, `telemetry`, `extensions`). Runners MUST:

- Preserve core fields (`inputs`, `context`, `memory`, `tool_responses`)
- Not modify core field semantics
- Document any extensions
- Handle missing extensions gracefully (runners that don't support extensions SHOULD ignore them)

### State Schema (Informative)

While ESP does not mandate a JSON Schema for state, the following structure is recommended:

```json
{
  "inputs": {
    "type": "object",
    "description": "Immutable invocation input"
  },
  "context": {
    "type": "object",
    "description": "Mutable working scratchpad"
  },
  "memory": {
    "type": "object",
    "description": "Optional persisted memory",
    "optional": true
  },
  "tool_responses": {
    "type": "object",
    "description": "Tool invocation results",
    "additionalProperties": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "params": { "type": "object" },
          "response": { "type": "any" },
          "error": { "type": "object", "optional": true },
          "timestamp": { "type": "string", "optional": true }
        }
      }
    }
  }
}
```

Runners MAY enforce stricter schemas or validation, but MUST accept state conforming to this structure.

## Flow Node Semantics

[To be written - detailed semantics per node kind]
- **input**: Receives external invocation, initializes state
- **output**: Returns final result, terminates run
- **llm**: Invokes LLM with model_ref and prompt_ref, updates context
- **tool**: Invokes tool via tool_ref, updates tool_responses
- **router**: Routes to next nodes based on strategy
- **retriever**: Queries memory/vector store, updates context
- **evaluator**: Runs evaluation metrics, updates context
- **subflow**: Executes referenced subflow (future)

## Tool Binding Semantics

ESP defines how tools declared in `tools.*` arrays are bound to flow nodes and invoked during execution.

### Tool Reference Field

In ADP v0.2.0, tool nodes (`kind: "tool"`) MAY include a `tool_ref` field that references a tool definition:

```yaml
flow:
  graph:
    nodes:
      - id: "call-api"
        kind: "tool"
        tool_ref: "metrics-api"  # References tools.http_apis[].id
```

The `tool_ref` field is a string that MUST match an `id` from one of the following tool arrays:
- `tools.mcp_servers[].id`
- `tools.http_apis[].id`
- `tools.sql_functions[].id`

### Tool Resolution

When a tool node executes:

1. **Resolution**: The runner resolves `tool_ref` to a tool definition:
   - Searches `tools.mcp_servers[]` for matching `id`
   - If not found, searches `tools.http_apis[]` for matching `id`
   - If not found, searches `tools.sql_functions[]` for matching `id`
   - If not found, resolution fails (see [Error & Failure Semantics](#error--failure-semantics))

2. **Tool type determination**: Based on which array contains the matching `id`, the runner determines the tool type:
   - MCP server → `tools.mcp_servers[]`
   - HTTP API → `tools.http_apis[]`
   - SQL function → `tools.sql_functions[]`

3. **Tool configuration**: The runner reads tool configuration from the matched tool definition (endpoint, transport, connection, etc.)

### Tool Invocation Semantics

Tool invocation proceeds as follows:

1. **Parameter extraction**: The runner extracts tool invocation parameters from state:
   - Parameters MAY come from `context` (e.g., `context.query`, `context.filters`)
   - Parameters MAY come from `node.params` (node-specific parameters)
   - Parameters MAY be derived from state structure (runner-dependent)

2. **Tool invocation**: The runner invokes the tool according to its type:
   - **MCP server**: Establishes transport connection (stdio, HTTP, etc.), sends MCP protocol messages
   - **HTTP API**: Constructs HTTP request with `base_url`, path, method, headers, body
   - **SQL function**: Establishes database connection, executes SQL function with parameters

3. **Response handling**: The runner receives tool response:
   - **Success**: Response data is written to state
   - **Failure**: Error information is written to state (see [Error & Failure Semantics](#error--failure-semantics))

### State Updates from Tool Invocation

After tool invocation:

1. **Tool responses**: The runner MUST append to `tool_responses[<tool_id>]`:
   ```json
   {
     "tool_responses": {
       "metrics-api": [
         {
           "params": { "query": "..." },
           "response": { "data": [...] },
           "timestamp": "2024-01-01T00:00:00Z"
         }
       ]
     }
   }
   ```

2. **Context updates**: The runner MAY update `context` with tool response data:
   - Response data MAY be written to `context.tool_results[<tool_id>]`
   - Response data MAY be merged into `context` directly
   - Response data MAY be transformed before writing to `context`

Runners MUST document their tool response handling behavior.

### Tool Parameter Mapping

ESP does not mandate a specific parameter mapping strategy. Runners MAY:

- Use `node.params` directly as tool parameters
- Extract parameters from `context` using field names
- Use parameter templates or transformations
- Support custom parameter mapping via extensions

Runners MUST document their parameter mapping strategy and MUST fail gracefully if required parameters are missing.

### Tool Authentication

Tool definitions MAY include authentication information:

- **HTTP APIs**: `tools.http_apis[].auth` field (free-form string, runner-dependent)
- **SQL functions**: `tools.sql_functions[].connection` MAY include credentials
- **MCP servers**: Authentication is protocol-dependent

Runners MUST handle authentication according to tool type and configuration. Authentication failures MUST be treated as permanent failures (see [Error & Failure Semantics](#error--failure-semantics)).

### Backward Compatibility

For ADP v0.1.0 manifests without `tool_ref`:

- **Fallback behavior**: Runners MAY infer tool binding using:
  1. `node.params.tool_id` (if present)
  2. `node.id` matching tool ID (if exact match)
  3. Custom inference logic (runner-dependent)
- **ESP conformance**: Runners SHOULD require `tool_ref` for ESP conformance but MUST support v0.1.0 fallbacks
- **Documentation**: Runners MUST document their v0.1.0 fallback behavior

ESP-conformant runners SHOULD require `tool_ref` for tool nodes to ensure deterministic tool binding.

## Model & Prompt Resolution

ESP defines how model references (`model_ref`) and prompt references (`system_prompt_ref`, `prompt_ref`) are resolved to concrete implementations.

### Model Reference Resolution

LLM nodes (`kind: "llm"`) MAY include a `model_ref` field that references a model configuration:

```yaml
flow:
  graph:
    nodes:
      - id: "planner"
        kind: "llm"
        model_ref: "primary"  # References a model configuration
```

#### Resolution Strategy

Runners MUST resolve `model_ref` using the following strategy (in order):

1. **Runtime models** (ADP v0.2.0): If `runtime.models[]` exists, search for matching `id`:
   ```yaml
   runtime:
     models:
       - id: "primary"
         provider: "openai"
         model: "gpt-4"
         api_key_env: "OPENAI_API_KEY"
   ```
   If found, use the model configuration from `runtime.models[]`.

2. **Model registry**: If not found in `runtime.models[]`, resolve via runner's model registry:
   - Runners MAY maintain a model registry mapping `model_ref` to concrete model identifiers
   - Model registries are runner-specific and MAY be configured via environment variables, config files, or APIs
   - Example: `"primary"` → `"gpt-4"` (OpenAI), `"claude-3-opus"` (Anthropic)

3. **Direct mapping**: If not found in registry, runners MAY treat `model_ref` as a direct model identifier:
   - For OpenAI: `model_ref` → OpenAI model ID (e.g., `"gpt-4"`, `"gpt-3.5-turbo"`)
   - For Anthropic: `model_ref` → Anthropic model ID (e.g., `"claude-3-opus"`, `"claude-3-sonnet"`)
   - For other providers: Provider-specific model identifiers

4. **Resolution failure**: If `model_ref` cannot be resolved, the runner MUST fail gracefully (see [Error & Failure Semantics](#error--failure-semantics)).

#### Model Configuration (ADP v0.2.0)

ADP v0.2.0 introduces `runtime.models[]` for explicit model configuration:

```yaml
runtime:
  models:
    - id: "primary"
      provider: "openai"  # or "anthropic", "custom", etc.
      model: "gpt-4"
      api_key_env: "OPENAI_API_KEY"  # Environment variable name
      base_url: "https://api.openai.com/v1"  # Optional, provider-specific
      temperature: 0.0  # Optional, default parameters
      max_tokens: 4096  # Optional
```

Runners MUST support `runtime.models[]` for ESP conformance. Runners MAY support additional provider-specific fields via extensions.

#### Provider Abstraction

ESP does not mandate specific LLM providers (OpenAI, Anthropic, etc.). Examples in this specification (e.g., `"gpt-4"`, `"claude-3-opus"`) are **illustrative only** and do not imply provider requirements. Runners MAY:

- Map `provider` values to their supported providers
- Support custom providers via extensions
- Use provider-specific APIs and authentication methods

Runners MUST document their supported providers and model resolution behavior.

### Prompt Reference Resolution

LLM nodes MAY include prompt references (`system_prompt_ref`, `prompt_ref`) that resolve to prompt text:

```yaml
flow:
  graph:
    nodes:
      - id: "planner"
        kind: "llm"
        system_prompt_ref: "prompts.roles.planner"
        prompt_ref: "prompts.system"
```

#### Resolution Strategy

Runners MUST resolve prompt references using dot-notation path resolution:

1. **Path parsing**: Parse the reference as a dot-separated path (e.g., `"prompts.roles.planner"` → `["prompts", "roles", "planner"]`)

2. **Resolution**:
   - Start from the root of the ADP manifest
   - Traverse the path, resolving each segment:
     - `prompts` → `prompts` object in manifest
     - `roles` → `prompts.roles` array or object
     - `planner` → Element in `prompts.roles` (by index if array, by key if object)

3. **Array resolution**: If a path segment resolves to an array:
   - If next segment is numeric (e.g., `"prompts.roles.0"`), use array index
   - If next segment is non-numeric, search array for matching `id` field (if objects) or match string value
   - If no match, resolution fails

4. **Object resolution**: If a path segment resolves to an object:
   - Use next segment as key to access object property
   - If property doesn't exist, resolution fails

5. **String extraction**: Final resolved value MUST be a string (prompt text). If resolved value is not a string, runners MAY:
   - Convert to string (if convertible)
   - Fail resolution (recommended for strict conformance)

#### Prompt Structure Examples

**Example 1: Simple prompts**
```yaml
prompts:
  system: "You are a helpful assistant."
```
- `system_prompt_ref: "prompts.system"` → `"You are a helpful assistant."`

**Example 2: Roles array**
```yaml
prompts:
  roles:
    - "planner"
    - "executor"
```
- `system_prompt_ref: "prompts.roles.0"` → `"planner"`
- `system_prompt_ref: "prompts.roles.1"` → `"executor"`

**Example 3: Roles object**
```yaml
prompts:
  roles:
    planner: "You are a planning agent."
    executor: "You are an execution agent."
```
- `system_prompt_ref: "prompts.roles.planner"` → `"You are a planning agent."`

**Example 4: Nested structure**
```yaml
prompts:
  system:
    default: "You are helpful."
    specialized: "You are specialized."
  roles:
    - id: "planner"
      prompt: "Plan the task."
```
- `system_prompt_ref: "prompts.system.default"` → `"You are helpful."`
- `system_prompt_ref: "prompts.roles.0.prompt"` → `"Plan the task."`

#### Prompt Variable Substitution

Runners MAY support variable substitution in prompts:

- Variables MAY be referenced as `{{variable_name}}` or `${variable_name}`
- Variables are resolved from `context` or `inputs`
- Example: `"Hello, {{user_name}}"` with `context.user_name = "Alice"` → `"Hello, Alice"`

Variable substitution is **optional**; runners MUST document their support and behavior.

#### Resolution Failure

If a prompt reference cannot be resolved:

1. **Missing path**: Path doesn't exist in manifest → Resolution failure
2. **Type mismatch**: Resolved value is not a string → Resolution failure (or conversion, runner-dependent)
3. **Empty value**: Resolved value is empty string → Runners MAY treat as valid (empty prompt) or fail

Runners MUST fail gracefully on resolution failure (see [Error & Failure Semantics](#error--failure-semantics)).

### Backward Compatibility

For ADP v0.1.0 manifests:

- **Model references**: Runners MAY resolve `model_ref` via model registry or direct mapping
- **Prompt references**: Runners MUST support dot-notation resolution for existing `prompts.*` structures
- **Missing runtime.models[]**: Runners SHOULD fall back to model registry or direct mapping

ESP-conformant runners SHOULD require `runtime.models[]` for explicit model configuration but MUST support v0.1.0 manifests without it.

## Error & Failure Semantics

ESP defines minimal error and failure semantics for agent execution. ESP does not mandate advanced retry policies or complex error recovery; it establishes basic expectations for failure handling.

### Node Failure

A **node failure** occurs when a node execution encounters an error that prevents successful completion. Node failures are categorized as:

- **Permanent failure**: A failure that cannot be recovered through retry. Examples:
  - Invalid input data (type mismatch, missing required fields)
  - Missing resource (model not found, tool endpoint unreachable)
  - Authentication failure (invalid API key, expired token)
  - Configuration error (invalid tool configuration, malformed prompt)

- **Transient failure**: A failure that MAY be recoverable through retry. Examples:
  - Network timeout
  - Rate limit exceeded (with backoff)
  - Temporary service unavailability
  - Resource exhaustion (with retry after delay)

Runners MUST distinguish between permanent and transient failures. Runners MAY use error codes, exception types, or heuristics to categorize failures.

### Failure Handling Requirements

ESP mandates the following failure handling behavior:

1. **Permanent failure handling**: On permanent failure:
   - The runner MUST either:
     - **Stop the run**: Terminate execution immediately and return error to caller
     - **Mark branch as failed**: In multi-path flows, mark the failed branch as failed and continue other branches if viable
   - The runner MUST NOT hide permanent failures from the caller
   - The runner MUST include error information in the run result (error message, node ID, failure type)

2. **Transient failure handling**: On transient failure:
   - Runners MAY implement retries (with backoff, exponential delay, etc.)
   - Runners MUST eventually report failure if retries are exhausted
   - Runners MUST NOT retry indefinitely
   - Runners SHOULD document their retry policies

3. **Error propagation**: Errors MUST propagate correctly:
   - Node failures MUST be observable in state (e.g., `context.errors[<node_id>]`)
   - Node failures MUST affect downstream nodes appropriately:
     - If a node fails, downstream nodes that depend on its output SHOULD NOT execute (unless they can handle missing input)
     - If a node fails, alternative paths (via conditional edges) MAY still be viable

### Error Information

When a node fails, runners MUST capture:

- **Node ID**: Identifier of the failed node
- **Failure type**: Permanent or transient
- **Error message**: Human-readable error description
- **Error details**: Additional context (error code, stack trace, etc., runner-dependent)

Error information MUST be accessible via:
- Run result (returned to caller)
- State updates (`context.errors`, `tool_responses[<tool_id>].error`, etc.)
- Telemetry/logging (runner-dependent)

### Multi-Path Failure Handling

In flows with multiple paths (branches):

1. **Branch independence**: Branch failures MUST NOT affect other branches unless:
   - The failure is permanent and prevents shared resources from being available
   - The failure occurs in a node that all branches depend on

2. **Branch termination**: A branch is terminated when:
   - A permanent failure occurs and no alternative paths exist
   - All paths from the branch lead to failed nodes
   - An end node is reached (successful branch completion)

3. **Run termination**: A run terminates when:
   - All branches terminate (successfully or with failure)
   - A critical node fails (runner-dependent definition of "critical")
   - An explicit termination condition is met

### Resolution Failures

Resolution failures (model_ref, prompt_ref, tool_ref cannot be resolved) are **permanent failures**:

- Runners MUST fail the node execution immediately
- Runners MUST NOT attempt retry (resolution failures are not transient)
- Runners MUST include resolution error details in error information

### Tool Invocation Failures

Tool invocation failures are handled according to tool type:

- **HTTP API failures**: Network errors are transient; HTTP 4xx errors are permanent; HTTP 5xx errors MAY be transient
- **MCP server failures**: Transport errors are transient; protocol errors are permanent
- **SQL function failures**: Connection errors are transient; SQL syntax errors are permanent

Runners MUST categorize tool failures appropriately and handle them according to failure type.

### Retry Policies (Optional)

Runners MAY implement retry policies for transient failures:

- **Retry limits**: Maximum number of retries (e.g., 3 attempts)
- **Backoff strategies**: Exponential backoff, linear backoff, fixed delay
- **Retry conditions**: Which errors trigger retries (transient failures only)

ESP does not mandate specific retry policies. Runners MUST document their retry behavior and MUST NOT retry permanent failures.

### Error Recovery (Optional)

Runners MAY implement error recovery mechanisms:

- **Fallback nodes**: Alternative nodes to execute on failure
- **Error handlers**: Special nodes that process errors and decide on recovery
- **Circuit breakers**: Temporarily disable failing components

ESP does not mandate error recovery. Runners MUST document their recovery mechanisms if implemented.

### Backward Compatibility

For ADP v0.1.0 manifests:

- Runners MUST handle errors gracefully even if error handling is not explicitly configured
- Runners MAY use default error handling behavior
- Runners MUST document their default error handling

ESP-conformant runners SHOULD provide explicit error handling configuration but MUST support v0.1.0 manifests with default behavior.

## Runtime Coordination

**Status**: Deferred to v0.3.0

Runtime coordination (how `runtime.execution[]` backends relate to flow nodes) is intentionally deferred to a future version. For v0.2.0:

- Runners coordinate runtime backends as needed
- No explicit `runtime_ref` field is required
- Flow nodes execute using available runtime capabilities
- Runners MAY map flow nodes to runtime backends implicitly

Future versions MAY introduce:
- `runtime_ref` field on flow nodes
- Explicit backend-to-node mapping
- Backend lifecycle management

## Optional Runner Capabilities

[To be written - advanced features]
- Parallel node execution
- Streaming responses
- Checkpointing and resumption
- Advanced retry policies
- Custom scheduling strategies

## Conformance Requirements

[To be written in Task 8]
- ESP-conformant runners MUST:
  - Correctly interpret flow.graph according to ESP
  - Implement state model semantics
  - Implement tool binding semantics
  - Implement model/prompt resolution semantics
- Concrete conformance scenarios
- Testable requirements

## Backward Compatibility

- ADP v0.1.0 manifests remain valid
- ESP is additive: new fields are optional
- Runners MAY support ESP semantics for v0.1.0 manifests
- ESP-conformant runners MUST accept v0.1.0 manifests

## ADP v0.2.0 Changes

ESP introduces the following changes for ADP v0.2.0:

### New Fields

- **`flow.graph.nodes[].tool_ref`** (optional): References a tool ID from `tools.*` arrays. Required for ESP-conformant tool binding.

- **`runtime.models[]`** (optional): Array of model configurations. Each model includes:
  - `id`: Model reference identifier
  - `provider`: Provider name (e.g., "openai", "anthropic")
  - `model`: Provider-specific model identifier
  - `api_key_env`: Environment variable name for API key
  - Additional provider-specific fields (optional)

### Enhanced Semantics

- **Flow execution**: ESP defines execution semantics for flow graphs (node readiness, edge traversal, state passing)
- **State model**: ESP defines minimal state structure (`inputs`, `context`, `memory`, `tool_responses`)
- **Tool binding**: ESP defines how tools are bound to nodes via `tool_ref`
- **Reference resolution**: ESP defines how `model_ref` and prompt references resolve
- **Error handling**: ESP defines basic failure semantics (permanent vs transient)

### Schema Updates Required

The following schema updates are required for ADP v0.2.0:

- **`schemas/flow.schema.json`**: Add `tool_ref` field to node definition (optional string)
- **`schemas/runtime.schema.json`**: Add `models[]` array to runtime definition (optional array of model objects)

## References

- [ADP Specification](adp-v0.1.0.md)
- [Runtime Specification](runtime.md)
- [Flow Specification](flow.md)
- [Evaluation Specification](evaluation.md)
- [Conformance Program](conformance.md)
