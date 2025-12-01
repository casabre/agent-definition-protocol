# ADP Composition (extends, import, overrides)

ADP v0.2.0 supports composition to reuse and adjust agent definitions.

## extends
```yaml
extends:
  adp: "acme.base-agent@v1.0.0"
```
- Deep merge semantics.
- Local fields override inherited fields.

## import
```yaml
import:
  flow: "acme.modules.flows.analytics@v2.0.0"
  prompts: "acme.modules.prompts.analytics@v1.0.0"
  guardrails: "acme.modules.guardrails.strict@v0.9.2"
  evaluation: "acme.modules.evals.groundedness@v1.4.0"
```
- Module-level imports merged into the current ADP.

## overrides
```yaml
overrides:
  guardrails.behavior.max_steps: 64
  runtime.execution[0].env.DEBUG: "true"
```
- Patch-like updates using dotted/JSON-pointer style paths.
- Last writer wins.

## Example
See `examples/composition/acme-inheritance-example.yaml` combining all three.
