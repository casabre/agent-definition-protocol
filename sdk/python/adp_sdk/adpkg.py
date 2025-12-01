from __future__ import annotations

import json
import zipfile
from pathlib import Path
from typing import Iterable

from .adp_model import ADP
from .validation import validate_adp


CONTENT_TYPES = """<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\n  <Default Extension=\"yaml\" ContentType=\"application/x-yaml\"/>\n  <Default Extension=\"json\" ContentType=\"application/json\"/>\n  <Override PartName=\"/adp/agent.yaml\" ContentType=\"application/vnd.adp.agent+yaml\"/>\n  <Override PartName=\"/acs/container.yaml\" ContentType=\"application/vnd.adp.container+yaml\"/>\n  <Override PartName=\"/metadata/version.json\" ContentType=\"application/json\"/>\n</Types>\n"""


def _iter_files(root: Path) -> Iterable[Path]:
    for path in root.rglob("*"):
        if path.is_file():
            yield path


class ADPackage:
    """OPC-style ADPKG helper."""

    def __init__(self, path: Path):
        self.path = Path(path)

    @classmethod
    def open(cls, path: str | Path) -> "ADPackage":
        return cls(Path(path))

    @classmethod
    def create_from_directory(cls, src: str | Path, out_path: str | Path) -> "ADPackage":
        src_path = Path(src)
        out = Path(out_path)
        with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED) as zf:
            # OPC Content Types
            zf.writestr("[Content_Types].xml", CONTENT_TYPES)
            adp_path = src_path / "adp" / "agent.yaml"
            adp = ADP.from_file(adp_path)
            validate_adp(adp)
            zf.write(adp_path, "adp/agent.yaml")
            acs_path = src_path / "acs" / "container.yaml"
            if acs_path.exists():
                zf.write(acs_path, "acs/container.yaml")
            for sub in ["eval", "tools", "src", "metadata"]:
                subdir = src_path / sub
                if subdir.exists():
                    for f in _iter_files(subdir):
                        zf.write(f, f.relative_to(src_path).as_posix())
            zf.writestr("metadata/version.json", json.dumps({"agent_id": adp.id}, indent=2))
        return cls(out)

    def list_files(self) -> list[str]:
        with zipfile.ZipFile(self.path, "r") as zf:
            return zf.namelist()

    def read_adp(self) -> ADP:
        with zipfile.ZipFile(self.path, "r") as zf:
            data = zf.read("adp/agent.yaml").decode()
        return ADP.from_file(Path(self.path)) if False else ADP.model_validate(
            yaml.safe_load(data)  # type: ignore[name-defined]
        )
