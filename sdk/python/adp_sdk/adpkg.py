from __future__ import annotations

import hashlib
import json
import tarfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterable, List

import yaml

from .adp_model import ADP
from .validation import validate_adp

OCI_LAYOUT = {"imageLayoutVersion": "1.0.0"}
MANIFEST_MEDIA_TYPE = "application/vnd.oci.image.manifest.v1+json"
LAYER_MEDIA_TYPE = "application/vnd.adp.package.v1+tar"
CONFIG_MEDIA_TYPE = "application/vnd.adp.config.v1+json"


@dataclass
class Descriptor:
    mediaType: str
    digest: str
    size: int
    annotations: dict | None = None

    def to_dict(self) -> dict:
        data = {
            "mediaType": self.mediaType,
            "digest": self.digest,
            "size": self.size,
        }
        if self.annotations:
            data["annotations"] = self.annotations
        return data


class ADPackage:
    """OCI-based ADP package helper."""

    def __init__(self, path: Path):
        self.path = Path(path)

    @staticmethod
    def _iter_files(root: Path) -> Iterable[Path]:
        for path in root.rglob("*"):
            if path.is_file():
                yield path

    @staticmethod
    def _blob_path(blobs: Path, digest: str) -> Path:
        algo, hexval = digest.split(":", 1)
        return blobs / algo / hexval

    @staticmethod
    def _hash_file(path: Path) -> tuple[str, int]:
        hasher = hashlib.sha256()
        size = 0
        with path.open("rb") as f:
            for chunk in iter(lambda: f.read(8192), b""):
                hasher.update(chunk)
                size += len(chunk)
        return f"sha256:{hasher.hexdigest()}", size

    @staticmethod
    def _hash_bytes(data: bytes) -> tuple[str, int]:
        h = hashlib.sha256(data).hexdigest()
        return f"sha256:{h}", len(data)

    @classmethod
    def create_from_directory(
        cls,
        src: str | Path,
        out_path: str | Path,
        builder_id: str = "",
        source_repo: str = "",
        source_ref: str = "",
    ) -> "ADPackage":
        src_path = Path(src)
        out_dir = Path(out_path)
        out_dir.mkdir(parents=True, exist_ok=True)
        if out_dir.suffix != "":
            raise ValueError(
                "OCI layout is a directory; provide a directory path, not a file"
            )

        adp_path = src_path / "adp" / "agent.yaml"
        adp = ADP.from_file(adp_path)
        validate_adp(adp)

        blobs = out_dir / "blobs" / "sha256"
        blobs.mkdir(parents=True, exist_ok=True)

        # Config blob with provenance fields
        config_bytes = json.dumps({
            "agent_id": adp.id,
            "adp_version": adp.adp_version,
            "builder.id": builder_id,
            "source.repo": source_repo,
            "source.ref": source_ref,
            "build_timestamp": datetime.now(timezone.utc).isoformat(),
        }).encode()
        config_digest, config_size = cls._hash_bytes(config_bytes)
        config_path = cls._blob_path(out_dir / "blobs", config_digest)
        config_path.parent.mkdir(parents=True, exist_ok=True)
        config_path.write_bytes(config_bytes)
        config_desc = Descriptor(CONFIG_MEDIA_TYPE, config_digest, config_size)

        # Layer blob: tar of src directory contents
        layer_tar = out_dir / "layer.tar"
        with tarfile.open(layer_tar, "w") as tar:
            for file_path in cls._iter_files(src_path):
                arcname = file_path.relative_to(src_path)
                tar.add(file_path, arcname=arcname)
        layer_digest, layer_size = cls._hash_file(layer_tar)
        layer_blob_path = cls._blob_path(out_dir / "blobs", layer_digest)
        layer_blob_path.parent.mkdir(parents=True, exist_ok=True)
        layer_blob_path.write_bytes(layer_tar.read_bytes())
        layer_tar.unlink()
        layer_desc = Descriptor(LAYER_MEDIA_TYPE, layer_digest, layer_size)

        manifest = {
            "schemaVersion": 2,
            "mediaType": MANIFEST_MEDIA_TYPE,
            "config": config_desc.to_dict(),
            "layers": [layer_desc.to_dict()],
        }
        manifest_bytes = json.dumps(manifest, indent=2).encode()
        manifest_digest, manifest_size = cls._hash_bytes(manifest_bytes)
        manifest_path = cls._blob_path(out_dir / "blobs", manifest_digest)
        manifest_path.parent.mkdir(parents=True, exist_ok=True)
        manifest_path.write_bytes(manifest_bytes)

        index = {
            "schemaVersion": 2,
            "manifests": [
                {
                    "mediaType": MANIFEST_MEDIA_TYPE,
                    "digest": manifest_digest,
                    "size": manifest_size,
                    "annotations": {"org.opencontainers.image.title": adp.id},
                }
            ],
        }
        (out_dir / "index.json").write_text(json.dumps(index, indent=2))
        (out_dir / "oci-layout").write_text(json.dumps(OCI_LAYOUT))
        return cls(out_dir)

    @classmethod
    def open(cls, path: str | Path) -> "ADPackage":
        return cls(Path(path))

    def inspect(self) -> dict:
        """Return structured metadata: agent_id, adp_version, layer count, config blob contents."""
        index = json.loads((self.path / "index.json").read_text())
        manifest_desc = index["manifests"][0]
        manifest_bytes = self._blob_path(self.path / "blobs", manifest_desc["digest"]).read_bytes()
        manifest = json.loads(manifest_bytes)
        config_bytes = self._blob_path(self.path / "blobs", manifest["config"]["digest"]).read_bytes()
        config = json.loads(config_bytes)
        return {
            "agent_id": config.get("agent_id"),
            "adp_version": config.get("adp_version"),
            "layer_count": len(manifest.get("layers", [])),
            "config": config,
        }

    def verify(self) -> dict:
        """Recompute SHA-256 of each blob and compare to stored digest. Returns pass/fail + failures."""
        failures: List[str] = []
        index = json.loads((self.path / "index.json").read_text())
        for entry in index.get("manifests", []):
            self._verify_blob(entry["digest"], failures)
            blob_path = self._blob_path(self.path / "blobs", entry["digest"])
            if not blob_path.exists():
                continue
            manifest = json.loads(blob_path.read_bytes())
            config_desc = manifest.get("config", {})
            if config_desc.get("digest"):
                self._verify_blob(config_desc["digest"], failures)
            for layer in manifest.get("layers", []):
                if layer.get("digest"):
                    self._verify_blob(layer["digest"], failures)
        return {"passed": len(failures) == 0, "failures": failures}

    def _verify_blob(self, digest: str, failures: List[str]) -> None:
        blob_path = self._blob_path(self.path / "blobs", digest)
        if not blob_path.exists():
            failures.append(f"{digest}: blob file missing")
            return
        actual_digest, _ = self._hash_bytes(blob_path.read_bytes())
        if actual_digest != digest:
            failures.append(f"{digest}: digest mismatch (actual {actual_digest})")

    def list_blobs(self) -> List[str]:
        return [p.name for p in (self.path / "blobs" / "sha256").glob("*")]

    def read_adp(self) -> ADP:
        # Extract layer tar and read adp/agent.yaml
        index = json.loads((self.path / "index.json").read_text())
        manifest_desc = index["manifests"][0]
        manifest = json.loads(
            (
                self.path
                / "blobs"
                / manifest_desc["digest"].replace("sha256:", "sha256/")
            ).read_text()
        )
        layer_desc = manifest["layers"][0]
        layer_path = (
            self.path / "blobs" / layer_desc["digest"].replace("sha256:", "sha256/")
        )
        with tarfile.open(layer_path, "r") as tar:
            try:
                member = tar.extractfile("adp/agent.yaml")
            except KeyError:
                raise FileNotFoundError("adp/agent.yaml not found in layer")
            if not member:
                raise FileNotFoundError("adp/agent.yaml not found in layer")
            data = member.read().decode()
        return ADP.model_validate(yaml.safe_load(data))
