# Getting Started

1. Read the normative specs in `spec/`.
2. Explore the examples under `examples/` (runtime, flow, evaluation, full ADP).
3. Install the CLI from `cli/`:

   ```bash
   python -m pip install -e cli
   adp --help
   ```

4. Validate an agent and container spec:

   ```bash
   adp validate --adp examples/adp/acme-full-agent.yaml
   ```

5. Pack and unpack a package:

   ```bash
   ./scripts/validate.sh
   ```

6. Run tests inside a virtual environment:

   ```bash
   python -m venv .venv
   source .venv/bin/activate
   python -m pip install -e cli pytest
   python -m pytest cli/tests
   ```

For detailed field descriptions, see:

- ADP: `specification/core/adp-v0.1.md`
- ACS: `specification/core/acs-v0.1.md`
- ADPKG: `specification/core/adpkg-v0.1.md`
