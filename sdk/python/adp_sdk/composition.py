from __future__ import annotations

import copy
from pathlib import Path
from typing import Any, Callable
from urllib.parse import urlparse
from urllib.request import urlopen

import yaml

from .adp_model import ADP
from .validation import validate_adp, validate_adp_semantics

Resolver = Callable[[str], str]

_MAX_DEPTH = 10


class CompositionError(Exception):
    pass


def resolve_adp(path: str | Path, resolver: Resolver | None = None) -> ADP:
    """Load and fully resolve an ADP manifest, applying extends/import/overrides.

    Raises CompositionError on cycles, unresolvable URIs, or invalid overrides.
    Runs both validate_adp and validate_adp_semantics on the merged result.
    """
    path = str(Path(path).resolve())
    raw = _load_uri(path, resolver)
    merged = _resolve_manifest(raw, base_uri=path, seen=set(), depth=0, resolver=resolver)
    adp = ADP.model_validate(merged)
    schema_errors = validate_adp(adp)
    semantic_errors = validate_adp_semantics(adp)
    all_errors = schema_errors + semantic_errors
    if all_errors:
        raise CompositionError("Resolved manifest is invalid:\n" + "\n".join(all_errors))
    return adp


def _resolve_manifest(
    data: dict,
    base_uri: str,
    seen: set,
    depth: int,
    resolver: Resolver | None,
) -> dict:
    if depth > _MAX_DEPTH:
        raise CompositionError(f"extends chain depth exceeded {_MAX_DEPTH}")
    if base_uri in seen:
        raise CompositionError(f"circular extends detected: {base_uri}")
    seen = seen | {base_uri}

    merged: dict = {}

    extends_uri = data.get("extends")
    if extends_uri:
        abs_uri = _resolve_uri(extends_uri, base_uri)
        base_raw = _load_uri(abs_uri, resolver)
        base_resolved = _resolve_manifest(base_raw, abs_uri, seen, depth + 1, resolver)
        merged = _deep_merge(merged, base_resolved)

    # Apply local fields over the extended base (RFC 7396 — local wins, arrays replace)
    local = {k: v for k, v in data.items() if k not in ("extends", "import", "overrides")}
    merged = _deep_merge(merged, local)

    # Apply imports additively AFTER local so both local and imported arrays coexist
    for entry in data.get("import", []):
        module_uri = _resolve_uri(entry["from"], base_uri)
        module_raw = _load_uri(module_uri, resolver)
        sections = entry.get("sections") or []
        if sections:
            module_raw = {k: v for k, v in module_raw.items() if k in sections}
        merged = _additive_merge(merged, module_raw)

    for override in data.get("overrides", []):
        merged = _apply_override(merged, override)

    return merged


def _deep_merge(base: dict, overlay: dict) -> dict:
    """RFC 7396 JSON Merge Patch: overlay wins; null removes; arrays replace."""
    result = copy.deepcopy(base)
    for key, val in overlay.items():
        if val is None:
            result.pop(key, None)
        elif isinstance(val, dict) and isinstance(result.get(key), dict):
            result[key] = _deep_merge(result[key], val)
        else:
            result[key] = copy.deepcopy(val)
    return result


def _additive_merge(base: dict, module: dict) -> dict:
    """Additive merge: arrays append; objects deep-merge; scalars: module wins."""
    result = copy.deepcopy(base)
    for key, val in module.items():
        if key not in result:
            result[key] = copy.deepcopy(val)
        elif isinstance(val, list) and isinstance(result[key], list):
            result[key] = result[key] + copy.deepcopy(val)
        elif isinstance(val, dict) and isinstance(result[key], dict):
            result[key] = _additive_merge(result[key], val)
        else:
            result[key] = copy.deepcopy(val)
    return result


def _apply_override(data: dict, override: dict) -> dict:
    path: str = override["path"]
    op: str = override.get("op", "set")
    value = override.get("value")

    if not path.startswith("/"):
        raise CompositionError(f"override path must start with '/': {path!r}")

    segments = [_unescape_pointer(s) for s in path.lstrip("/").split("/")]

    result = copy.deepcopy(data)

    if op == "delete":
        node = result
        for seg in segments[:-1]:
            node = _pointer_get(node, seg, path, allow_missing=True)
            if node is None:
                return result
        last = segments[-1]
        if isinstance(node, dict):
            node.pop(last, None)
        return result

    node = result
    for seg in segments[:-1]:
        node = _pointer_get(node, seg, path)
    last = segments[-1]

    if op == "set":
        if isinstance(node, dict):
            if last not in node:
                raise CompositionError(f"override set: path '{path}' does not exist")
            node[last] = copy.deepcopy(value)
        elif isinstance(node, list):
            idx = _to_index(last, path)
            node[idx] = copy.deepcopy(value)
        else:
            raise CompositionError(f"override set: cannot navigate path '{path}'")
    elif op == "append":
        target = _pointer_get(node, last, path)
        if not isinstance(target, list):
            raise CompositionError(f"override append: path '{path}' does not resolve to an array")
        target.append(copy.deepcopy(value))
    else:
        raise CompositionError(f"unknown override op: {op!r}")

    return result


def _pointer_get(node: Any, segment: str, path: str, allow_missing: bool = False) -> Any:
    if isinstance(node, dict):
        if allow_missing:
            return node.get(segment)
        if segment not in node:
            raise CompositionError(f"override: path segment '{segment}' not found (path: '{path}')")
        return node[segment]
    if isinstance(node, list):
        idx = _to_index(segment, path)
        return node[idx]
    raise CompositionError(f"override: cannot navigate into {type(node).__name__} at path '{path}'")


def _to_index(segment: str, path: str) -> int:
    try:
        return int(segment)
    except ValueError:
        raise CompositionError(f"override: array index '{segment}' is not an integer (path: '{path}')")


def _unescape_pointer(segment: str) -> str:
    return segment.replace("~1", "/").replace("~0", "~")


def _resolve_uri(uri: str, base_uri: str) -> str:
    parsed = urlparse(uri)
    if parsed.scheme in ("http", "https", "file"):
        return uri
    if parsed.scheme == "registry":
        raise CompositionError(
            f"registry:// URIs are not supported in v0.2.0; planned for v0.3.0: {uri!r}"
        )
    base_path = Path(base_uri)
    return str((base_path.parent / uri).resolve())


def _load_uri(uri: str, resolver: Resolver | None) -> dict:
    if resolver is not None:
        raw = resolver(uri)
        return yaml.safe_load(raw)
    parsed = urlparse(uri)
    if parsed.scheme in ("http", "https"):  # pragma: no cover
        with urlopen(uri, timeout=10) as resp:  # noqa: S310  # pragma: no cover
            return yaml.safe_load(resp.read())  # pragma: no cover
    path = Path(uri)
    if not path.exists():
        raise CompositionError(f"cannot resolve URI: {uri!r}")
    return yaml.safe_load(path.read_text())
