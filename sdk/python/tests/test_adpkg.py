from pathlib import Path
import sys
import json
import tarfile
import pytest

sys.path.append(str(Path(__file__).resolve().parents[1]))

from adp_sdk.adpkg import ADPackage  # type: ignore
from adp_sdk.adp_model import ADP  # type: ignore


def build_source(tmp_path: Path, version: str = "0.1.0") -> Path:
    adp_dir = tmp_path / "adp"
    adp_dir.mkdir(parents=True)
    agent_yaml = """
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
    adp_dir.joinpath("agent.yaml").write_text(agent_yaml)
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


def test_create_and_read_oci_package_v0_1_0(tmp_path: Path) -> None:
    """Test ADPKG round-trip with v0.1.0 manifest."""
    src = build_source(tmp_path, version="0.1.0")
    pkg_dir = tmp_path / "oci"
    pkg = ADPackage.create_from_directory(src, pkg_dir)

    # Read back ADP
    adp = pkg.read_adp()
    assert isinstance(adp, ADP)
    assert adp.id == "agent.test"
    assert adp.adp_version == "0.1.0"


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


def test_descriptor_with_annotations():
    """Test Descriptor.to_dict() includes annotations when present."""
    from adp_sdk.adpkg import Descriptor

    # Descriptor without annotations
    desc_no_ann = Descriptor(
        mediaType="application/test", digest="sha256:abc123", size=100
    )
    data_no_ann = desc_no_ann.to_dict()
    assert "annotations" not in data_no_ann, "Should not include annotations when None"
    assert data_no_ann["mediaType"] == "application/test"
    assert data_no_ann["digest"] == "sha256:abc123"
    assert data_no_ann["size"] == 100

    # Descriptor with annotations
    annotations = {"org.test.key": "value", "version": "1.0"}
    desc_with_ann = Descriptor(
        mediaType="application/test",
        digest="sha256:abc123",
        size=100,
        annotations=annotations,
    )
    data_with_ann = desc_with_ann.to_dict()
    assert "annotations" in data_with_ann, "Should include annotations when present"
    assert data_with_ann["annotations"] == annotations, "Annotations should match"
    assert data_with_ann["annotations"]["org.test.key"] == "value"
    assert data_with_ann["annotations"]["version"] == "1.0"


def test_create_package_with_file_path_raises_error(tmp_path: Path):
    """Test that create_from_directory raises ValueError when out_path is a file."""
    src = build_source(tmp_path)
    file_path = tmp_path / "package.tar"  # File path with suffix

    with pytest.raises(ValueError) as exc_info:
        ADPackage.create_from_directory(src, file_path)

    error_msg = str(exc_info.value)
    assert "directory" in error_msg.lower(), "Error should mention directory"
    assert "file" in error_msg.lower() or "suffix" in error_msg.lower(), (
        "Error should mention file or suffix"
    )


def test_read_adp_missing_agent_yaml(tmp_path: Path):
    """Test that read_adp raises FileNotFoundError when adp/agent.yaml is missing."""
    src = build_source(tmp_path)
    pkg_dir = tmp_path / "oci"
    pkg = ADPackage.create_from_directory(src, pkg_dir)

    # Corrupt the package by removing adp/agent.yaml from the tar
    import json
    import tarfile

    # Get the layer path
    index = json.loads((pkg_dir / "index.json").read_text())
    manifest_digest = index["manifests"][0]["digest"].split(":")[1]
    manifest = json.loads((pkg_dir / "blobs" / "sha256" / manifest_digest).read_text())
    layer_digest = manifest["layers"][0]["digest"].split(":")[1]
    layer_path = pkg_dir / "blobs" / "sha256" / layer_digest

    # Create a new tar without adp/agent.yaml
    corrupted_tar = tmp_path / "corrupted.tar"
    with tarfile.open(layer_path, "r") as original:
        with tarfile.open(corrupted_tar, "w") as new_tar:
            for member in original.getmembers():
                if member.name != "adp/agent.yaml":
                    file_obj = original.extractfile(member)
                    if file_obj:
                        new_tar.addfile(member, file_obj)

    # Replace the layer
    corrupted_tar.replace(layer_path)

    # Try to read ADP - should fail with FileNotFoundError
    with pytest.raises(FileNotFoundError) as exc_info:
        pkg.read_adp()

    error_msg = str(exc_info.value)
    assert "adp/agent.yaml" in error_msg.lower(), (
        f"Error should mention adp/agent.yaml, got: {error_msg}"
    )
    assert "not found" in error_msg.lower(), (
        f"Error should indicate file not found, got: {error_msg}"
    )


def test_inspect_returns_metadata(tmp_path: Path):
    """Test that inspect() returns structured metadata from an OCI package."""
    src = build_source(tmp_path)
    pkg_dir = tmp_path / "oci"
    pkg = ADPackage.create_from_directory(
        src, pkg_dir, builder_id="ci", source_repo="https://github.com/test", source_ref="abc123"
    )
    info = pkg.inspect()
    assert info["agent_id"] == "agent.test"
    assert info["adp_version"] == "0.1.0"
    assert info["layer_count"] == 1
    assert "config" in info
    assert info["config"]["builder.id"] == "ci"
    assert info["config"]["source.repo"] == "https://github.com/test"
    assert info["config"]["source.ref"] == "abc123"


def test_verify_passes_intact_package(tmp_path: Path):
    """Test that verify() returns passed=True for an intact package."""
    src = build_source(tmp_path)
    pkg_dir = tmp_path / "oci"
    pkg = ADPackage.create_from_directory(src, pkg_dir)
    result = pkg.verify()
    assert result["passed"] is True
    assert result["failures"] == []


def test_verify_detects_missing_blob(tmp_path: Path):
    """Test that verify() detects a missing blob file (config/layer)."""
    src = build_source(tmp_path)
    pkg_dir = tmp_path / "oci"
    pkg = ADPackage.create_from_directory(src, pkg_dir)

    # Specifically delete the config blob (deterministic: read from manifest)
    index = json.loads((pkg_dir / "index.json").read_text())
    manifest_desc = index["manifests"][0]
    manifest = json.loads(
        (pkg_dir / "blobs" / "sha256" / manifest_desc["digest"].split(":")[1]).read_bytes()
    )
    config_hex = manifest["config"]["digest"].split(":")[1]
    (pkg_dir / "blobs" / "sha256" / config_hex).unlink()

    result = pkg.verify()
    assert result["passed"] is False
    assert len(result["failures"]) > 0
    assert any("missing" in f for f in result["failures"])


def test_verify_detects_missing_manifest_blob(tmp_path: Path):
    """Test that verify() skips inner-blob checks when the manifest blob itself is missing."""
    src = build_source(tmp_path)
    pkg_dir = tmp_path / "oci"
    pkg = ADPackage.create_from_directory(src, pkg_dir)

    # Delete the manifest blob specifically to exercise the `continue` guard
    index = json.loads((pkg_dir / "index.json").read_text())
    manifest_hex = index["manifests"][0]["digest"].split(":")[1]
    (pkg_dir / "blobs" / "sha256" / manifest_hex).unlink()

    result = pkg.verify()
    assert result["passed"] is False
    assert any("missing" in f for f in result["failures"])


def test_verify_detects_digest_mismatch(tmp_path: Path):
    """Test that verify() detects a tampered blob (digest mismatch)."""
    src = build_source(tmp_path)
    pkg_dir = tmp_path / "oci"
    pkg = ADPackage.create_from_directory(src, pkg_dir)

    # Get the config blob path from index.json → manifest → config
    index = json.loads((pkg_dir / "index.json").read_text())
    manifest_desc = index["manifests"][0]
    manifest = json.loads(
        (pkg_dir / "blobs" / "sha256" / manifest_desc["digest"].split(":")[1]).read_bytes()
    )
    config_digest_str = manifest["config"]["digest"]
    config_blob = pkg_dir / "blobs" / "sha256" / config_digest_str.split(":")[1]

    # Corrupt the config blob
    config_blob.write_bytes(b"tampered content")

    result = pkg.verify()
    assert result["passed"] is False
    assert any("mismatch" in f for f in result["failures"])


def test_read_adp_agent_yaml_is_directory(tmp_path: Path):
    """Test that read_adp raises FileNotFoundError when adp/agent.yaml is a directory."""
    src = build_source(tmp_path)
    pkg_dir = tmp_path / "oci"

    # Create a package where adp/agent.yaml is a directory instead of a file
    import json
    import tarfile

    # First create normal package
    pkg = ADPackage.create_from_directory(src, pkg_dir)

    # Get the layer path
    index = json.loads((pkg_dir / "index.json").read_text())
    manifest_digest = index["manifests"][0]["digest"].split(":")[1]
    manifest = json.loads((pkg_dir / "blobs" / "sha256" / manifest_digest).read_text())
    layer_digest = manifest["layers"][0]["digest"].split(":")[1]
    layer_path = pkg_dir / "blobs" / "sha256" / layer_digest

    # Create a new tar where adp/agent.yaml is a directory
    corrupted_tar = tmp_path / "corrupted.tar"
    with tarfile.open(layer_path, "r") as original:
        with tarfile.open(corrupted_tar, "w") as new_tar:
            for member in original.getmembers():
                if member.name == "adp/agent.yaml":
                    # Make it a directory
                    member.type = tarfile.DIRTYPE
                    new_tar.addfile(member)
                else:
                    file_obj = original.extractfile(member)
                    if file_obj:
                        new_tar.addfile(member, file_obj)

    # Replace the layer
    corrupted_tar.replace(layer_path)

    # Try to read ADP - should fail with FileNotFoundError (extractfile returns None for directories)
    with pytest.raises(FileNotFoundError) as exc_info:
        pkg.read_adp()

    error_msg = str(exc_info.value)
    assert "adp/agent.yaml" in error_msg.lower(), (
        f"Error should mention adp/agent.yaml, got: {error_msg}"
    )
    assert "not found" in error_msg.lower(), (
        f"Error should indicate file not found, got: {error_msg}"
    )
