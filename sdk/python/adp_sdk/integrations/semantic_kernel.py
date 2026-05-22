"""
ADP → Semantic Kernel conversion utilities. Import-only: ADP → SK.
Export (SK → ADP) is deferred to v0.3.0.

Targets semantic-kernel >= 1.3 (Python SDK).
Note: KernelProcess is experimental in SK Python (stable in C#).

See spec/framework-interop.md §Semantic Kernel Mapping for the mapping guide.
"""
from __future__ import annotations
from typing import Any, Callable

try:
    import semantic_kernel as sk
    from semantic_kernel import Kernel
except ImportError:
    sk = None  # type: ignore[assignment]
    Kernel = None  # type: ignore[assignment, misc]

BackendFactory = Callable[[dict, dict], Any] | None


def _resolve_model(model_ref: str | None, models: list[dict]) -> dict | None:
    """Look up a model entry from runtime.models[] by ID."""
    if model_ref is None:
        return None
    for m in models:
        if m.get("id") == model_ref:
            return m
    return None


def _make_llm_step(node: dict, models: list[dict]) -> dict:
    """Build a step descriptor for an llm node, resolving model_ref from runtime.models."""
    model_ref = node.get("model_ref")
    model_entry = _resolve_model(model_ref, models)
    step: dict[str, Any] = {
        "id": node["id"],
        "kind": "llm",
        "model_ref": model_ref,
    }
    if model_entry is not None:
        step["provider"] = model_entry.get("provider")
        step["model"] = model_entry.get("model")
    if Kernel is not None:
        step["sk_construct"] = "KernelFunction"
    return step


def _make_tool_step(node: dict) -> dict:
    """Build a step descriptor for a tool node."""
    step: dict[str, Any] = {
        "id": node["id"],
        "kind": "tool",
        "tool_ref": node.get("tool_ref"),
    }
    if Kernel is not None:
        step["sk_construct"] = "KernelPlugin"
    return step


def _make_generic_step(node: dict) -> dict:
    """Build a step descriptor for any other node kind."""
    return {
        "id": node["id"],
        "kind": node.get("kind", "unknown"),
    }


def build_sk_from_adp(manifest: dict, backend_factory: BackendFactory = None) -> tuple:
    """Build a Semantic Kernel Kernel + process structure from an ADP manifest.

    Returns:
        (kernel: Kernel | dict, process_steps: list[dict])

    kernel is a Kernel instance (or mock dict if SK not installed).
    process_steps is a list of step descriptors with node metadata.

    KernelProcess is experimental in SK Python (stable in C#). This function
    returns process_steps as plain dicts to remain usable without SK installed
    (mock mode).
    """
    flow = manifest["flow"]["graph"]
    runtime = manifest.get("runtime", {})
    models = runtime.get("models", [])

    if Kernel is not None:
        kernel = Kernel()
        if backend_factory is not None:
            backend_factory(manifest, {"kernel": kernel})
    else:
        kernel = {"type": "mock_kernel"}

    process_steps: list[dict] = []
    for node in flow["nodes"]:
        kind = node.get("kind", "")
        if kind == "llm":
            step = _make_llm_step(node, models)
        elif kind == "tool":
            step = _make_tool_step(node)
        else:
            step = _make_generic_step(node)
        process_steps.append(step)

    return kernel, process_steps
