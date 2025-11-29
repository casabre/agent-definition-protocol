# Getting Started

1. Read the normative specs in `specification/core/`.
2. Explore the example at `examples/acme-analytics`.
3. Install the CLI from `cli/`:

   ```bash
   python -m pip install -e cli
   adp --help
   ```

4. Validate an agent and container spec:

   ```bash
   adp validate --adp examples/acme-analytics/adp/agent.yaml --acs examples/acme-analytics/acs/container.yaml
   ```

5. Pack and unpack a package:

   ```bash
   adp pack --agent examples/acme-analytics/adp/agent.yaml --acs examples/acme-analytics/acs/container.yaml --src examples/acme-analytics/src --eval examples/acme-analytics/eval --out acme-analytics-0.1.0.adpkg
   adp unpack --pkg acme-analytics-0.1.0.adpkg --out ./unpacked
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
