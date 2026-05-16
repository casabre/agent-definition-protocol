# Runtime-to-Flow Binding

Normative for ADP v0.1.1. RFC 2119 terms apply.

This document specifies how flow nodes are bound to runtime backends. It defines the backend compatibility matrix, the default binding rule, the explicit override mechanism, the condition expression format for conditional edges, and the graph construction algorithm for runner implementers.

## Backend Compatibility Matrix

The following table defines which `backend` values in `runtime.execution[]` are compatible with each flow node kind.

| Node kind | Compatible `backend` values |
|---|---|
| `llm` | `openai`, `anthropic`, `vllm`, `bedrock`, `langchain`, `litellm`, any value starting with `llm:` |
| `tool` | `python`, `docker`, `http`, `mcp`, `wasm`, any value starting with `tool:` |
| `retriever` | `pinecone`, `weaviate`, `chroma`, `redis`, `pgvector`, any value starting with `memory:` |
| `evaluator` | any (evaluators are not backend-specific; use `runtime_ref` for explicit selection) |
| `router`, `input`, `output` | any (control-flow nodes carry no backend affinity) |

Implementors SHOULD register proprietary backends via `extensions.x_<vendor>.backend_compat` rather than modifying this table. Example:

```yaml
extensions:
  x_acme:
    backend_compat:
      llm: ["acme-llm-v2", "acme-llm:fast"]
      tool: ["acme-tool-runner"]
```

## Default Binding Rule

A runner MUST select the first `runtime.execution[]` entry whose `backend` value is compatible (per the matrix above) with the node kind.

If no compatible entry exists for a node, the runner MUST fail with a clear error before execution starts. The error MUST name the node ID and the missing backend type. Example:

```
Error: no compatible runtime backend for node 'synthesizer' (kind: llm).
Available backends: [docker]. Add an openai/anthropic/vllm/bedrock/langchain/litellm or llm:* entry to runtime.execution[].
```

Example manifest with two backends — planner uses the first `openai` entry by default, executor uses the first `python` entry:

```yaml
runtime:
  execution:
    - id: llm-backend
      backend: openai
      model: gpt-4o
    - id: tool-backend
      backend: python
      entrypoint: tools/runner.py

flow:
  id: my-flow
  graph:
    nodes:
      - id: planner
        kind: llm
      - id: executor
        kind: tool
      - id: result
        kind: output
    edges:
      - from: planner
        to: executor
      - from: executor
        to: result
    start_nodes: [planner]
    end_nodes: [result]
```

## Explicit Backend Override (`runtime_ref`)

Any flow node MAY declare a `runtime_ref` field containing the `id` of a `runtime.execution[]` entry. When present, `runtime_ref` overrides the default binding rule for that node.

If `runtime_ref` names an ID that does not exist in `runtime.execution[]`, the runner MUST fail with a clear error before execution starts.

Example — two LLM backends; `synthesizer` explicitly binds to the faster model:

```yaml
runtime:
  execution:
    - id: llm-standard
      backend: openai
      model: gpt-4o
    - id: llm-fast
      backend: openai
      model: gpt-4o-mini

flow:
  graph:
    nodes:
      - id: planner
        kind: llm
        # no runtime_ref — uses llm-standard (first compatible)
      - id: synthesizer
        kind: llm
        runtime_ref: llm-fast   # explicit override
```

## Deployment-Time Override (v0.2.0 Gap)

`runtime_ref` is a manifest-level declaration and cannot be overridden at deployment time without changing the manifest. Per-environment backend substitution (e.g., in Kubernetes) is not specified in v0.1.1. Runners that require this SHOULD use `extensions.x_<vendor>.runtime_override` as a convention until a standard mechanism is defined. This is recorded as a v0.2.0 design item.

## Graph Construction Algorithm

The following algorithm SHOULD be used by runner implementers to build an executable graph from an ADP manifest. Pseudocode is framework-agnostic; see [framework-interop.md](framework-interop.md) for framework-specific implementations.

```
GIVEN: ADP manifest M, validated against schema and semantics

1. Validate M.flow.graph for duplicate node IDs. MUST fail if any node.id appears more than once.

2. Build node_table = {node.id: node} from M.flow.graph.nodes[]

3. For each node in node_table:
     backend_entry = resolve_backend(node, M.runtime.execution[], compat_matrix)
       where resolve_backend:
         if node.runtime_ref present:
           entry = find entry where entry.id == node.runtime_ref
           if not found: FAIL with error
         else:
           entry = first entry where entry.backend is compatible (per matrix)
           if not found: FAIL with error
     callable = runtime_adapter(backend_entry).make_callable(node)
     graph.add_node(node.id, callable)

4. For each edge in M.flow.graph.edges[]:
     if edge.condition present:
       validate condition expression syntax (see §Condition Expression Format)
       routing_fn = compile_condition(edge.condition)
       graph.add_conditional_edge(edge.from, routing_fn, edge.to)
     else:
       graph.add_edge(edge.from, edge.to)

5. graph.set_entry_points(M.flow.graph.start_nodes[])
6. graph.set_exit_points(M.flow.graph.end_nodes[])

7. RETURN compiled graph
```

## Condition Expression Format

`edge.condition` is a **boolean expression string** evaluated against the current execution state at runtime.

### Syntax

```
<key> <op> <value>
```

| Part | Description |
|---|---|
| `<key>` | Dot-notation path into the state dict (see §Key Path Format) |
| `<op>` | Comparison operator (see §Operators) |
| `<value>` | JSON literal or unquoted string token |

### Key Path Format

Path segments are separated by `.`. The path is evaluated by descending into the state dict segment by segment.

- **First segment**: state root key — one of `inputs`, `context`, `memory`, `tool_responses`
- **For `context` and `tool_responses`**: second segment is the `node.id` of the node that wrote the value (per ADR D2/D3); subsequent segments traverse the written value
- **For `inputs`**: second segment is a field name in the invocation data

Examples:

| Path | Resolves to |
|---|---|
| `context.router_node.decision` | `state["context"]["router_node"]["decision"]` |
| `tool_responses.search_tool.0.url` | `state["tool_responses"]["search_tool"][0]["url"]` |
| `inputs.query` | `state["inputs"]["query"]` |

### Operators

Runners MUST support `==` and `!=`. Runners SHOULD support the full set.

| Operator | Meaning |
|---|---|
| `==` | Equal (MUST support) |
| `!=` | Not equal (MUST support) |
| `>` | Greater than |
| `>=` | Greater than or equal |
| `<` | Less than |
| `<=` | Less than or equal |
| `contains` | String or list contains value |
| `not_contains` | String or list does not contain value |

### Error Behavior

If a runner encounters an unsupported operator or a malformed expression, it MUST fail with a clear error before execution starts. Runners MUST NOT silently evaluate malformed conditions to `true` or `false`.

### Complex Logic

Boolean logic combining multiple conditions (`AND`, `OR`, `NOT`) is deferred to v0.2.0. For v0.1.1:
- **AND semantics**: use multiple sequential edges; both must fire for the path to complete
- **OR semantics**: use multiple conditional edges from the same source node; the first matching condition wins

## Parallel Branch Write Semantics

See [esp.md §Parallel Branch Write Semantics](esp.md#parallel-branch-write-semantics).

## References

- [ADP Flow Schema](../schemas/flow.schema.json) — `runtime_ref` field in node definition
- [Framework Interop Guide](framework-interop.md) — LangGraph/AutoGen/SK/CrewAI mapping
- [ESP Specification](esp.md) — State model and node execution semantics
- [ADR: ESP Node Semantics](decisions/esp-node-semantics.md) — D1–D7 decisions
