# Getting Started

1. Read the normative specs in `spec/`.
2. Explore the examples under `examples/` (runtime, flow, evaluation, full ADP).
3. Validate with schemas:

   ```bash
   ./scripts/validate.sh
   ```

4. Use SDKs (Python/TS/Rust/Go) for programmatic validation/OCI packaging (see README).

5. Run tests inside a virtual environment:

   ```bash
   python -m venv .venv
   source .venv/bin/activate
   python -m pip install -e sdk/python pytest
   python -m pytest sdk/python/tests
   ```

For detailed field descriptions, see `spec/` and `schemas/`. Composition is a future feature (see `roadmap.md`).
