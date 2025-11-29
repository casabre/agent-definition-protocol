"""Utilities for packing and unpacking ADPKG files."""
from __future__ import annotations

import json
import zipfile
from collections.abc import Iterable
from datetime import UTC, datetime
from pathlib import Path

import yaml

CONTENT_TYPES_XML = """<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">
  <Default Extension=\"yaml\" ContentType=\"application/x-yaml\"/>
  <Default Extension=\"json\" ContentType=\"application/json\"/>
  <Override PartName=\"/adp/agent.yaml\" ContentType=\"application/vnd.adp.agent+yaml\"/>
  <Override PartName=\"/acs/container.yaml\" ContentType=\"application/vnd.adp.container+yaml\"/>
</Types>
"""


def _iter_files(root: Path) -> Iterable[Path]:
    for path in root.rglob("*"):
        if path.is_file():
            yield path


def _write_tree(zipf: zipfile.ZipFile, source: Path, target_prefix: str) -> None:
    for file_path in _iter_files(source):
        relative = file_path.relative_to(source)
        arcname = f"{target_prefix}/{relative.as_posix()}"
        zipf.write(file_path, arcname)


def _build_metadata(agent_path: Path) -> dict:
    try:
        data = yaml.safe_load(agent_path.read_text()) or {}
    except FileNotFoundError:
        data = {}
    return {
        "agent_id": data.get("id", "unknown"),
        "agent_version": data.get("version", "0.1.0"),
        "spec_version": data.get("adp_version", "0.1"),
        "build_timestamp": datetime.now(UTC).isoformat(),
    }


def pack_adpkg(agent: Path, acs: Path, src: Path, eval_dir: Path, out_path: Path) -> Path:
    """Create an ADPKG zip with the expected layout."""
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out_path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("[Content_Types].xml", CONTENT_TYPES_XML)
        zf.write(agent, "adp/agent.yaml")
        zf.write(acs, "acs/container.yaml")
        if src.exists():
            _write_tree(zf, src, "src")
        if eval_dir.exists():
            _write_tree(zf, eval_dir, "eval")
        metadata = _build_metadata(agent)
        zf.writestr("metadata/version.json", json.dumps(metadata, indent=2))
    return out_path


def unpack_adpkg(pkg_path: Path, out_dir: Path) -> Path:
    """Unpack an ADPKG into a directory."""
    out_dir.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(pkg_path, "r") as zf:
        zf.extractall(out_dir)
    return out_dir
