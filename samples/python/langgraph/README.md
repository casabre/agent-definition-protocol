# LangGraph Sample (Python)

Minimal LangGraph agent packaged for ADP v0.1.0.

Structure:
- `adp/agent.yaml`: ADP manifest referencing the python backend entrypoint `samples.langgraph.agent:main`.
- `src/samples/langgraph/agent.py`: Minimal LangGraph graph that prints a greeting.

Setup:
```bash
python -m venv .venv
source .venv/bin/activate
python -m pip install langgraph==0.2.1 pyyaml
```

Run locally:
```bash
python samples/python/langgraph/src/samples/langgraph/agent.py
```

Package (OCI layout via Python SDK):
```bash
python - <<'PY'
from pathlib import Path
from adp_sdk.adpkg import ADPackage
src = Path('samples/python/langgraph')
out = Path('samples/python/langgraph-oci')
out.mkdir(exist_ok=True)
ADPackage.create_from_directory(src, out)
print('Wrote OCI ADPKG to', out)
PY
```
