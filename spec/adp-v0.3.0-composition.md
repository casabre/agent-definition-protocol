# ADP v0.3.0 — Composition: Id-Keyed Local Field Merge

**Status**: Draft  
**Added in**: ADP Spec v0.3.0  
**Applies to**: All `adp_version` values

---

## Motivation

The existing `extends:` field uses RFC 7396 JSON Merge Patch semantics for local fields: arrays replace entirely. This makes it impossible to update a single named entry in a list without repeating the entire list — fragile and verbose when base manifests carry many list items.

**Before (v0.2.0 RFC 7396 — full list required to change one entry):**

```yaml
extends: "./base-agent.yaml"

# Must repeat entire models list to change one model
runtime:
  models:
    - id: "gpt4"
      provider: "openai"
      model: "gpt-4o"        # only this changed; rest must be restated
    - id: "claude"
      provider: "anthropic"
      model: "claude-3"
    - id: "llama"
      provider: "meta"
      model: "llama-3"
```

**After (v0.3.0 id-keyed merge — update only what changes):**

```yaml
extends: "./base-agent.yaml"

# Only the changed entry; all other base entries are kept automatically
runtime:
  models:
    - id: "gpt4"
      model: "gpt-4o"
```

---

## Local Field Merge Semantics (v0.3.0)

When a manifest uses `extends:`, local fields (all top-level keys except `extends`, `import`, and `overrides`) are merged into the resolved base using **id-keyed semantics** rather than RFC 7396 array-replace.

### Rules

| Scenario | Rule |
|----------|------|
| Object key in local, key exists in base as object | Deep merge recursively (local wins on scalar collisions) |
| Object key in local, key absent from base | Add key with local value |
| Object key in local, key exists in base as non-object | Overwrite (local wins — patch replaces scalar/type mismatch) |
| List in local: **all** items have `id` | Id-keyed merge: match by `id`, update in-place; unmatched base entries kept; unknown local ids appended |
| List in local: **any** item lacks `id` | Replace entire base list |
| `null` value in local | Remove key from base |
| Within a matched list entry | Sub-fields recurse with same id-keyed semantics |

### Id-keyed list merge detail

When all items in a local list carry an `id` field:

1. Build an index of base list items by their `id`.
2. For each local item:
   - If `id` matches a base entry → deep-merge the matched entry (recursive id-keyed semantics).
   - If `id` is unknown → append to the end of the list.
3. Unmatched base entries are retained at their original positions.

This applies to any depth in the manifest tree — the same logic is used recursively within matched list entries.

---

## Resolution Order

```
1. extends   — load and recursively resolve base manifest
2. local     — id-keyed merge of local fields (composition keys excluded)
3. import    — additive merge of modules (arrays append)
4. overrides — RFC 6901 JSON Pointer patches (final word)
```

**Composition keys** (`extends`, `import`, `overrides`) are excluded from local field processing and do not appear in the resolved output.

---

## Examples

### Object deep merge

```yaml
# Base telemetry
telemetry:
  service_name: base-svc
  protocol: grpc
  sampling_rate: 1.0

# Child (local field)
telemetry:
  service_name: prod-svc   # overrides service_name
                            # protocol and sampling_rate are inherited
```

Result: `{service_name: prod-svc, protocol: grpc, sampling_rate: 1.0}`

### Id-keyed list merge — update in-place

```yaml
# Base models
runtime:
  models:
    - id: gpt4
      provider: openai
      model: gpt-4
    - id: claude
      provider: anthropic
      model: claude-3

# Child (local field — all items have id)
runtime:
  models:
    - id: gpt4
      model: gpt-4o        # patches only gpt4 entry; claude entry kept
```

Result: `[{id: gpt4, provider: openai, model: gpt-4o}, {id: claude, provider: anthropic, model: claude-3}]`

### Id-keyed list merge — new entry appended

```yaml
# Child (local field — id not in base)
runtime:
  models:
    - id: llama
      provider: meta
      model: llama-3.1
```

Result: base models list with `{id: llama, ...}` appended.

### List without `id` — replace

```yaml
# Base
telemetry:
  required_attributes:
    - gen_ai.system
    - gen_ai.request.model

# Child (local field — items have no id)
telemetry:
  required_attributes:
    - custom.team_id     # replaces entire list
```

### Null removes key

```yaml
# Child (local field)
telemetry: null          # removes telemetry section entirely
```

### Local fields and `overrides:` coexisting

Local fields apply before `overrides:`, so `overrides:` has the final word on any path:

```yaml
extends: "./base.yaml"

telemetry:
  service_name: prod-svc   # applied at step 2 (local)

overrides:
  - path: /telemetry/protocol
    value: http/protobuf    # applied at step 4 (final word)
    op: set
```

### Full pipeline — all composition mechanisms active

```yaml
adp_version: "0.3.0"
id: "agent.prod"
extends: "./base-agent.yaml"    # step 1: load and merge base
import:                          # step 3: additive module merge
  - id: safety-evals
    from: "./safety-module.yaml"

# step 2: id-keyed local field merge
runtime:
  models:
    - id: primary
      model: gpt-4o
telemetry:
  service_name: agent-prod

overrides:                       # step 4: JSON Pointer final word
  - path: /telemetry/sampling_rate
    value: 0.1
    op: set
```

---

## When to Use Local Fields vs `overrides:`

| Use local fields when… | Use `overrides:` when… |
|------------------------|------------------------|
| Updating named fields on id-carrying list entries | Deleting a key (`op: delete`) |
| Overriding scalar fields in nested objects | Appending to a list tail (`op: append`) |
| Adding a new section absent from base | Targeting an unnamed list entry by index |
| Readability is a priority | Exact path precision is needed |

Both can coexist in the same manifest; local fields run first.

---

## `adp_version` Scope

Id-keyed local field merge applies to manifests declaring any `adp_version` (`"0.1.0"`, `"0.2.0"`, `"0.3.0"`). The behavior is always active when a manifest uses `extends:`; no opt-in is required.

---

## Standard References

- **`extends:`** base loading: RFC 7396 JSON Merge Patch (objects deep-merge, `null` removes key)
- **`overrides:`** path-based ops: RFC 6901 JSON Pointer
- **Id-keyed list merge** (step 2 local fields): ADP-specific design; no external RFC
