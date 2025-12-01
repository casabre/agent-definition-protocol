from pathlib import Path
import sys

sys.path.append(str(Path(__file__).resolve().parents[1]))

from adp_sdk.adpkg import ADPackage  # type: ignore
from adp_sdk.adp_model import ADP  # type: ignore


def build_source(tmp_path: Path) -> Path:
    adp_dir = tmp_path / "adp"
    adp_dir.mkdir(parents=True)
    adp_dir.joinpath("agent.yaml").write_text(
        """
        adp_version: "0.1.0"
        id: "agent.test"
        runtime:
          execution:
            - backend: "python"
              id: "py"
              entrypoint: "agent.main:app"
        flow: {}
        evaluation: {}
        """
    )
    (tmp_path / "acs").mkdir()
    (tmp_path / "acs" / "container.yaml").write_text("base_image: python:3.12\n")
    (tmp_path / "metadata").mkdir()
    (tmp_path / "metadata" / "version.json").write_text("{}\n")
    return tmp_path


def test_create_and_read_oci_package(tmp_path: Path) -> None:
    src = build_source(tmp_path)
    pkg_dir = tmp_path / "oci"
    pkg = ADPackage.create_from_directory(src, pkg_dir)

    # Verify layout
    assert (pkg_dir / "oci-layout").exists()
    assert (pkg_dir / "index.json").exists()
    assert (pkg_dir / "blobs" / "sha256").exists()

    # Read back ADP
    adp = pkg.read_adp()
    assert isinstance(adp, ADP)
    assert adp.id == "agent.test"
    assert pkg.list_blobs(), "blobs should not be empty"


def test_validation_failure(tmp_path: Path) -> None:
    src = tmp_path / "src"
    src.mkdir()
    (src / "adp").mkdir()
    (src / "adp" / "agent.yaml").write_text("adp_version: '0.1.0'\nid: bad\n")
    pkg_dir = tmp_path / "oci"
    try:
        ADPackage.create_from_directory(src, pkg_dir)
    except Exception:
        assert not (pkg_dir / "index.json").exists()
    else:
        raise AssertionError("expected validation failure")


def test_fixture_validation(tmp_path: Path) -> None:
    fixture = Path(__file__).resolve().parents[2].parent / "fixtures" / "adp_full.yaml"
    src = tmp_path / "src"
    (src / "adp").mkdir(parents=True)
    (src / "adp" / "agent.yaml").write_text(fixture.read_text())
    pkg_dir = tmp_path / "oci"
    pkg = ADPackage.create_from_directory(src, pkg_dir)
    adp = pkg.read_adp()
    assert adp.id == "fixture.acme.full"
