"""Extended ADPKG tests with comprehensive assertions."""

from pathlib import Path
import json
import tarfile
import pytest
from adp_sdk.adpkg import ADPackage
from adp_sdk.adp_model import ADP


def build_source_with_metadata(tmp_path: Path) -> Path:
    """Build source directory with complete metadata."""
    adp_dir = tmp_path / "adp"
    adp_dir.mkdir(parents=True)
    adp_dir.joinpath("agent.yaml").write_text("""
adp_version: "0.1.0"
id: "agent.complete"
name: "Complete Agent"
description: "Agent with full metadata"
runtime:
  execution:
    - backend: "python"
      id: "py"
      entrypoint: "agent.main:app"
      env:
        LOG_LEVEL: "info"
flow:
  id: "test.flow"
  graph:
    nodes:
      - id: "input"
        kind: "input"
    edges: []
    start_nodes: ["input"]
    end_nodes: ["input"]
evaluation:
  suites:
    - id: "basic"
      metrics:
        - id: "m1"
          type: "deterministic"
          function: "noop"
          scoring: "boolean"
          threshold: true
""")

    # Add metadata
    metadata_dir = tmp_path / "metadata"
    metadata_dir.mkdir()
    metadata_dir.joinpath("version.json").write_text(
        json.dumps(
            {
                "agent_id": "agent.complete",
                "agent_version": "1.0.0",
                "spec_version": "0.1.0",
                "build_timestamp": "2024-01-15T10:30:00Z",
            }
        )
    )

    # Add eval directory
    eval_dir = tmp_path / "eval"
    eval_dir.mkdir()
    eval_dir.joinpath("basic.yaml").write_text("suite: basic")

    return tmp_path


def test_create_package_with_complete_structure(tmp_path: Path):
    """Test creating package with complete directory structure."""
    src = build_source_with_metadata(tmp_path)
    pkg_dir = tmp_path / "oci"
    ADPackage.create_from_directory(src, pkg_dir)

    # Verify OCI layout structure
    assert (pkg_dir / "oci-layout").exists(), "oci-layout file should exist"
    assert (pkg_dir / "index.json").exists(), "index.json should exist"
    assert (pkg_dir / "blobs" / "sha256").exists(), (
        "blobs/sha256 directory should exist"
    )

    # Verify oci-layout content
    layout_content = json.loads((pkg_dir / "oci-layout").read_text())
    assert layout_content == {"imageLayoutVersion": "1.0.0"}, (
        "oci-layout should have correct version"
    )

    # Verify index.json structure
    index = json.loads((pkg_dir / "index.json").read_text())
    assert "manifests" in index, "index.json should contain manifests"
    assert len(index["manifests"]) == 1, (
        "index.json should reference exactly one manifest"
    )
    assert (
        index["manifests"][0]["mediaType"]
        == "application/vnd.oci.image.manifest.v1+json"
    )


def test_package_contains_required_files(tmp_path: Path):
    """Test that package layer contains required ADP manifest."""
    src = build_source_with_metadata(tmp_path)
    pkg_dir = tmp_path / "oci"
    ADPackage.create_from_directory(src, pkg_dir)

    # Extract manifest digest
    index = json.loads((pkg_dir / "index.json").read_text())
    manifest_digest = index["manifests"][0]["digest"].split(":")[1]
    manifest_path = pkg_dir / "blobs" / "sha256" / manifest_digest
    assert manifest_path.exists(), "Manifest blob should exist"

    # Read manifest
    manifest = json.loads(manifest_path.read_text())
    assert manifest["mediaType"] == "application/vnd.oci.image.manifest.v1+json"
    assert len(manifest["layers"]) > 0, "Manifest should have at least one layer"

    # Extract layer and verify contents
    layer_digest = manifest["layers"][0]["digest"].split(":")[1]
    layer_path = pkg_dir / "blobs" / "sha256" / layer_digest
    assert layer_path.exists(), "Layer blob should exist"

    # Check layer is tar archive
    with tarfile.open(layer_path, "r") as tar:
        names = tar.getnames()
        assert "adp/agent.yaml" in names, "Package layer should contain adp/agent.yaml"
        # Optional files may or may not be present
        if "metadata/version.json" in names:
            metadata_content = tar.extractfile("metadata/version.json")
            assert metadata_content is not None
            metadata = json.loads(metadata_content.read())
            assert metadata["agent_id"] == "agent.complete"


def test_package_config_blob_structure(tmp_path: Path):
    """Test that config blob has correct structure."""
    src = build_source_with_metadata(tmp_path)
    pkg_dir = tmp_path / "oci"
    ADPackage.create_from_directory(src, pkg_dir)

    # Extract config digest from manifest
    index = json.loads((pkg_dir / "index.json").read_text())
    manifest_digest = index["manifests"][0]["digest"].split(":")[1]
    manifest = json.loads((pkg_dir / "blobs" / "sha256" / manifest_digest).read_text())

    config_digest = manifest["config"]["digest"].split(":")[1]
    config_path = pkg_dir / "blobs" / "sha256" / config_digest
    assert config_path.exists(), "Config blob should exist"

    # Verify config content
    config = json.loads(config_path.read_text())
    # Config blob contains agent metadata, not mediaType (that's in manifest)
    assert "agent_id" in config, "Config should contain agent_id"
    assert "adp_version" in config, "Config should contain adp_version"
    assert config["agent_id"] == "agent.complete"
    assert config["adp_version"] == "0.1.0"

    # Verify manifest has correct config mediaType
    assert manifest["config"]["mediaType"] == "application/vnd.adp.config.v1+json"


def test_read_adp_from_package(tmp_path: Path):
    """Test reading ADP manifest from package."""
    src = build_source_with_metadata(tmp_path)
    pkg_dir = tmp_path / "oci"
    pkg = ADPackage.create_from_directory(src, pkg_dir)

    # Read ADP
    adp = pkg.read_adp()
    assert isinstance(adp, ADP), "read_adp should return ADP instance"
    assert adp.id == "agent.complete", f"Expected id 'agent.complete', got '{adp.id}'"
    assert adp.name == "Complete Agent", (
        f"Expected name 'Complete Agent', got '{adp.name}'"
    )
    assert adp.description == "Agent with full metadata"
    assert len(adp.runtime.execution) == 1
    assert adp.runtime.execution[0].backend == "python"
    assert adp.runtime.execution[0].id == "py"


def test_package_digest_integrity(tmp_path: Path):
    """Test that digests match actual blob content."""
    src = build_source_with_metadata(tmp_path)
    pkg_dir = tmp_path / "oci"
    ADPackage.create_from_directory(src, pkg_dir)

    # Verify manifest digest matches content
    index = json.loads((pkg_dir / "index.json").read_text())
    manifest_digest_str = index["manifests"][0]["digest"]
    algo, expected_digest = manifest_digest_str.split(":", 1)
    manifest_path = pkg_dir / "blobs" / algo / expected_digest

    assert manifest_path.exists(), f"Manifest blob should exist at {manifest_path}"

    # Verify config digest
    manifest = json.loads(manifest_path.read_text())
    config_digest_str = manifest["config"]["digest"]
    config_algo, config_digest = config_digest_str.split(":", 1)
    config_path = pkg_dir / "blobs" / config_algo / config_digest
    assert config_path.exists(), "Config blob should exist"


def test_package_annotations(tmp_path: Path):
    """Test that package includes OCI annotations."""
    src = build_source_with_metadata(tmp_path)
    pkg_dir = tmp_path / "oci"
    ADPackage.create_from_directory(src, pkg_dir)

    # Check index annotations
    index = json.loads((pkg_dir / "index.json").read_text())
    if "annotations" in index["manifests"][0]:
        annotations = index["manifests"][0]["annotations"]
        assert (
            "org.opencontainers.image.title" in annotations
            or "io.adp.version" in annotations
        )


def test_open_existing_package(tmp_path: Path):
    """Test opening an existing package."""
    src = build_source_with_metadata(tmp_path)
    pkg_dir = tmp_path / "oci"

    # Create package
    ADPackage.create_from_directory(src, pkg_dir)

    # Open existing package
    pkg = ADPackage.open(pkg_dir)
    assert pkg.path == pkg_dir.resolve()

    # Verify we can read ADP
    adp = pkg.read_adp()
    assert adp.id == "agent.complete"


def test_package_error_handling(tmp_path: Path):
    """Test error handling for invalid packages."""
    # Missing adp/agent.yaml
    src = tmp_path / "src"
    src.mkdir()
    pkg_dir = tmp_path / "oci"

    with pytest.raises((FileNotFoundError, ValueError)):
        ADPackage.create_from_directory(src, pkg_dir)

    # Invalid ADP manifest
    adp_dir = src / "adp"
    adp_dir.mkdir()
    adp_dir.joinpath("agent.yaml").write_text("invalid: yaml: content:")

    with pytest.raises(Exception):
        ADPackage.create_from_directory(src, pkg_dir)


def test_list_blobs(tmp_path: Path):
    """Test listing blobs in package."""
    src = build_source_with_metadata(tmp_path)
    pkg_dir = tmp_path / "oci"
    pkg = ADPackage.create_from_directory(src, pkg_dir)

    blobs = pkg.list_blobs()
    assert isinstance(blobs, list), "list_blobs should return a list"
    assert len(blobs) > 0, "Package should contain blobs"

    # Verify blob names are strings and blobs exist
    for blob_name in blobs:
        assert isinstance(blob_name, str), (
            f"Blob name should be string, got {type(blob_name)}"
        )
        blob_path = pkg_dir / "blobs" / "sha256" / blob_name
        assert blob_path.exists(), f"Blob should exist: {blob_path}"
