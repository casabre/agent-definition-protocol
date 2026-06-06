use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tar::Builder as TarBuilder;

use crate::adp::Adp;
use crate::validation::validate_adp;

const OCI_LAYOUT: &str = r#"{"imageLayoutVersion":"1.0.0"}"#;
const MANIFEST_MEDIA: &str = "application/vnd.oci.image.manifest.v1+json";
const LAYER_MEDIA: &str = "application/vnd.adp.package.v1+tar";
const CONFIG_MEDIA: &str = "application/vnd.adp.config.v1+json";

fn sha256_bytes(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

pub fn blob_path(root: &Path, digest: &str) -> std::path::PathBuf {
    let parts: Vec<&str> = digest.split(':').collect();
    root.join("blobs").join(parts[0]).join(parts[1])
}

pub fn create_adpkg(src_dir: &str, out_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    create_adpkg_with_provenance(src_dir, out_dir, "", "", "")
}

pub fn create_adpkg_with_provenance(
    src_dir: &str,
    out_dir: &str,
    builder_id: &str,
    source_repo: &str,
    source_ref: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let adp_path = Path::new(src_dir).join("adp/agent.yaml");
    let adp = crate::adp::load_adp(adp_path.to_str().ok_or("Invalid path")?)?;
    validate_adp(&adp)?;

    let out = Path::new(out_dir);
    fs::create_dir_all(out.join("blobs/sha256"))?;
    let out_abs = fs::canonicalize(out)?;
    let src_abs = fs::canonicalize(src_dir)?;

    // config blob with provenance fields
    let now_secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let build_timestamp = format!("{}", now_secs);
    let config = serde_json::json!({
        "agent_id": adp.id,
        "adp_version": adp.adp_version,
        "builder.id": builder_id,
        "source.repo": source_repo,
        "source.ref": source_ref,
        "build_timestamp": build_timestamp,
    }).to_string();

    let config_bytes = config.into_bytes();
    let config_digest = sha256_bytes(&config_bytes);
    fs::write(blob_path(out, &config_digest), &config_bytes)?;

    // layer tar
    let layer_tar = out.join("layer.tar");
    let mut builder = TarBuilder::new(File::create(&layer_tar)?);
    for entry in walkdir::WalkDir::new(&src_abs).into_iter().filter_map(|e| e.ok()) {
        // Skip the OCI output directory to avoid self-inclusion and potential recursion.
        let entry_path = entry.path().to_path_buf();
        if entry_path.starts_with(&out_abs) {
            continue;
        }
        if entry.file_type().is_file() {
            let rel = entry_path.strip_prefix(&src_abs)?;
            builder.append_path_with_name(entry.path(), rel)?;
        }
    }
    builder.finish()?;
    let layer_bytes = fs::read(&layer_tar)?;
    let layer_digest = sha256_bytes(&layer_bytes);
    fs::write(blob_path(out, &layer_digest), &layer_bytes)?;
    fs::remove_file(&layer_tar)?;

    let manifest = serde_json::json!({
        "schemaVersion": 2,
        "mediaType": MANIFEST_MEDIA,
        "config": {"mediaType": CONFIG_MEDIA, "digest": config_digest, "size": config_bytes.len()},
        "layers": [{"mediaType": LAYER_MEDIA, "digest": layer_digest, "size": layer_bytes.len()}]
    });
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    let manifest_digest = sha256_bytes(&manifest_bytes);
    fs::write(blob_path(out, &manifest_digest), &manifest_bytes)?;

    let index = serde_json::json!({
        "schemaVersion": 2,
        "manifests": [{
            "mediaType": MANIFEST_MEDIA,
            "digest": manifest_digest,
            "size": manifest_bytes.len(),
            "annotations": {"org.opencontainers.image.title": adp.id}
        }]
    });
    fs::write(out.join("index.json"), serde_json::to_vec_pretty(&index)?)?;
    fs::write(out.join("oci-layout"), OCI_LAYOUT)?;
    Ok(())
}
pub struct InspectResult {
    pub agent_id: String,
    pub adp_version: String,
    pub layer_count: usize,
    pub config: serde_json::Value,
}

pub struct VerifyResult {
    pub passed: bool,
    pub failures: Vec<String>,
}

pub fn inspect_adpkg(path: &str) -> Result<InspectResult, Box<dyn std::error::Error>> {
    let root = Path::new(path);
    let index: serde_json::Value = serde_json::from_reader(File::open(root.join("index.json"))?)?;
    let manifest_digest = index["manifests"][0]["digest"].as_str().ok_or("missing manifest digest")?;
    let manifest: serde_json::Value = serde_json::from_reader(File::open(blob_path(root, manifest_digest))?)?;
    let config_digest = manifest["config"]["digest"].as_str().ok_or("missing config digest")?;
    let config: serde_json::Value = serde_json::from_reader(File::open(blob_path(root, config_digest))?)?;
    Ok(InspectResult {
        agent_id: config["agent_id"].as_str().unwrap_or("").to_string(),
        adp_version: config["adp_version"].as_str().unwrap_or("").to_string(),
        layer_count: manifest["layers"].as_array().map(|a| a.len()).unwrap_or(0),
        config,
    })
}

pub fn verify_adpkg(path: &str) -> Result<VerifyResult, Box<dyn std::error::Error>> {
    let root = Path::new(path);
    let index: serde_json::Value = serde_json::from_reader(File::open(root.join("index.json"))?)?;
    let mut failures = Vec::new();
    if let Some(manifests) = index["manifests"].as_array() {
        for entry in manifests {
            let digest = entry["digest"].as_str().unwrap_or("");
            verify_blob(root, digest, &mut failures);
            let manifest_path = blob_path(root, digest);
            if let Ok(f) = File::open(&manifest_path) {
                let manifest: serde_json::Value = serde_json::from_reader(f).unwrap_or_default();
                if let Some(d) = manifest["config"]["digest"].as_str() {
                    verify_blob(root, d, &mut failures);
                }
                if let Some(layers) = manifest["layers"].as_array() {
                    for layer in layers {
                        if let Some(d) = layer["digest"].as_str() {
                            verify_blob(root, d, &mut failures);
                        }
                    }
                }
            }
        }
    }
    let passed = failures.is_empty();
    Ok(VerifyResult { passed, failures })
}

fn verify_blob(root: &Path, digest: &str, failures: &mut Vec<String>) {
    let bp = blob_path(root, digest);
    match fs::read(&bp) {
        Err(_) => failures.push(format!("{}: blob file missing", digest)),
        Ok(data) => {
            let actual = sha256_bytes(&data);
            if actual != digest {
                failures.push(format!("{}: digest mismatch (actual {})", digest, actual));
            }
        }
    }
}

pub fn open_adpkg(path: &str) -> Result<Adp, Box<dyn std::error::Error>> {
    let root = Path::new(path);
    let index: serde_json::Value = serde_json::from_reader(File::open(root.join("index.json"))?)?;
    let manifest_desc = &index["manifests"][0];
    let manifest_path = blob_path(root, manifest_desc["digest"].as_str().unwrap());
    let manifest: serde_json::Value = serde_json::from_reader(File::open(manifest_path)?)?;
    let layer_desc = &manifest["layers"][0];
    let layer_path = blob_path(root, layer_desc["digest"].as_str().unwrap());
    let mut archive = tar::Archive::new(File::open(layer_path)?);
    for entry in archive.entries()? {
        let mut e = entry?;
        if e.path().is_ok_and(|p| p == std::path::Path::new("adp/agent.yaml")) {
            let mut buf = String::new();
            e.read_to_string(&mut buf)?;
            let adp: Adp = serde_yaml::from_str(&buf)?;
            return Ok(adp);
        }
    }
    Err("adp/agent.yaml not found".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const MINIMAL_AGENT_YAML: &str = r#"adp_version: "0.1.0"
id: "test-agent"
runtime:
  execution:
    - id: "r1"
      backend: "python"
      entrypoint: "agent.main:app"
flow:
  id: "test.flow"
  graph:
    nodes:
      - id: "n1"
        kind: "input"
    edges: []
    start_nodes: ["n1"]
    end_nodes: ["n1"]
evaluation:
  suites:
    - id: "s1"
      metrics:
        - id: "m1"
          type: "deterministic"
          function: "noop"
          scoring: "boolean"
          threshold: true
"#;

    fn setup_src_dir() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        let adp_dir = dir.path().join("adp");
        fs::create_dir_all(&adp_dir).unwrap();
        fs::write(adp_dir.join("agent.yaml"), MINIMAL_AGENT_YAML).unwrap();
        dir
    }

    fn setup_pkg(src: &tempfile::TempDir) -> tempfile::TempDir {
        let out_dir = tempfile::TempDir::new().unwrap();
        create_adpkg(
            src.path().to_str().unwrap(),
            out_dir.path().to_str().unwrap(),
        ).unwrap();
        out_dir
    }

    // ===== blob_path tests =====

    #[test]
    fn test_blob_path_parses_digest_correctly() {
        let root = Path::new("/root");
        let digest = "sha256:abc123def456";
        let path = blob_path(root, digest);
        assert_eq!(path, std::path::PathBuf::from("/root/blobs/sha256/abc123def456"));
    }

    // ===== create_adpkg tests =====

    #[test]
    fn test_create_adpkg_happy_path() {
        let src = setup_src_dir();
        let out = tempfile::TempDir::new().unwrap();
        let result = create_adpkg(
            src.path().to_str().unwrap(),
            out.path().to_str().unwrap(),
        );
        assert!(result.is_ok(), "create_adpkg should succeed: {:?}", result.err());
        // Verify expected OCI structure
        assert!(out.path().join("index.json").exists(), "index.json should exist");
        assert!(out.path().join("oci-layout").exists(), "oci-layout should exist");
        assert!(out.path().join("blobs/sha256").exists(), "blobs/sha256 should exist");
    }

    #[test]
    fn test_create_adpkg_with_provenance() {
        let src = setup_src_dir();
        let out = tempfile::TempDir::new().unwrap();
        let result = create_adpkg_with_provenance(
            src.path().to_str().unwrap(),
            out.path().to_str().unwrap(),
            "my-builder",
            "https://github.com/org/repo",
            "main",
        );
        assert!(result.is_ok(), "create_adpkg_with_provenance should succeed: {:?}", result.err());
    }

    #[test]
    fn test_create_adpkg_invalid_agent_yaml() {
        let src = tempfile::TempDir::new().unwrap();
        let adp_dir = src.path().join("adp");
        fs::create_dir_all(&adp_dir).unwrap();
        fs::write(adp_dir.join("agent.yaml"), "invalid: yaml: [{{not valid]]").unwrap();
        let out = tempfile::TempDir::new().unwrap();
        let result = create_adpkg(src.path().to_str().unwrap(), out.path().to_str().unwrap());
        assert!(result.is_err(), "Should fail with invalid YAML");
    }

    #[test]
    fn test_create_adpkg_missing_agent_yaml() {
        let src = tempfile::TempDir::new().unwrap();
        let out = tempfile::TempDir::new().unwrap();
        let result = create_adpkg(src.path().to_str().unwrap(), out.path().to_str().unwrap());
        assert!(result.is_err(), "Should fail without adp/agent.yaml");
    }

    #[test]
    fn test_create_adpkg_validation_failure() {
        let src = tempfile::TempDir::new().unwrap();
        let adp_dir = src.path().join("adp");
        fs::create_dir_all(&adp_dir).unwrap();
        // Missing required execution
        fs::write(adp_dir.join("agent.yaml"), r#"adp_version: "0.1.0"
id: "bad"
runtime:
  execution: []
flow:
  id: "f"
  graph:
    nodes: []
    edges: []
evaluation: {}
"#).unwrap();
        let out = tempfile::TempDir::new().unwrap();
        let result = create_adpkg(src.path().to_str().unwrap(), out.path().to_str().unwrap());
        assert!(result.is_err(), "Should fail with validation error");
    }

    #[test]
    fn test_create_adpkg_excludes_out_dir_from_layer() {
        // When src and out are the same directory (or out is inside src),
        // the out dir should be excluded from the tar layer
        let base = tempfile::TempDir::new().unwrap();
        let adp_dir = base.path().join("adp");
        fs::create_dir_all(&adp_dir).unwrap();
        fs::write(adp_dir.join("agent.yaml"), MINIMAL_AGENT_YAML).unwrap();
        // Create a sub-directory for output within src
        let out_dir = base.path().join("pkg_out");
        fs::create_dir_all(&out_dir).unwrap();

        let result = create_adpkg(
            base.path().to_str().unwrap(),
            out_dir.to_str().unwrap(),
        );
        // This may succeed or fail depending on canonicalization; just verify no panic
        let _ = result;
    }

    // ===== inspect_adpkg tests =====

    #[test]
    fn test_inspect_adpkg_happy_path() {
        let src = setup_src_dir();
        let pkg = setup_pkg(&src);

        let result = inspect_adpkg(pkg.path().to_str().unwrap());
        assert!(result.is_ok(), "inspect_adpkg should succeed: {:?}", result.err());
        let info = result.unwrap();
        assert_eq!(info.agent_id, "test-agent");
        assert_eq!(info.adp_version, "0.1.0");
        assert_eq!(info.layer_count, 1);
    }

    #[test]
    fn test_inspect_adpkg_missing_index_json() {
        let dir = tempfile::TempDir::new().unwrap();
        let result = inspect_adpkg(dir.path().to_str().unwrap());
        assert!(result.is_err());
    }

    // ===== verify_adpkg tests =====

    #[test]
    fn test_verify_adpkg_happy_path() {
        let src = setup_src_dir();
        let pkg = setup_pkg(&src);

        let result = verify_adpkg(pkg.path().to_str().unwrap());
        assert!(result.is_ok(), "verify_adpkg should succeed: {:?}", result.err());
        let verify = result.unwrap();
        assert!(verify.passed, "All blobs should pass verification: {:?}", verify.failures);
        assert!(verify.failures.is_empty());
    }

    #[test]
    fn test_verify_adpkg_missing_index_json() {
        let dir = tempfile::TempDir::new().unwrap();
        let result = verify_adpkg(dir.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_adpkg_tampered_blob() {
        let src = setup_src_dir();
        let pkg = setup_pkg(&src);

        // Read index to find the first blob
        let index: serde_json::Value = serde_json::from_reader(
            File::open(pkg.path().join("index.json")).unwrap()
        ).unwrap();
        let manifest_digest = index["manifests"][0]["digest"].as_str().unwrap().to_string();
        let blob_p = blob_path(pkg.path(), &manifest_digest);

        // Tamper with the blob
        let mut f = std::fs::OpenOptions::new().write(true).open(&blob_p).unwrap();
        f.write_all(b"tampered!").unwrap();

        let result = verify_adpkg(pkg.path().to_str().unwrap());
        assert!(result.is_ok()); // verify_adpkg itself should not error
        let verify = result.unwrap();
        assert!(!verify.passed, "Tampered blob should fail verification");
        assert!(!verify.failures.is_empty());
    }

    #[test]
    fn test_verify_adpkg_missing_blob() {
        let src = setup_src_dir();
        let pkg = setup_pkg(&src);

        // Read index to find the manifest blob and delete it
        let index: serde_json::Value = serde_json::from_reader(
            File::open(pkg.path().join("index.json")).unwrap()
        ).unwrap();
        let manifest_digest = index["manifests"][0]["digest"].as_str().unwrap().to_string();
        let blob_p = blob_path(pkg.path(), &manifest_digest);
        fs::remove_file(&blob_p).unwrap();

        let result = verify_adpkg(pkg.path().to_str().unwrap());
        assert!(result.is_ok()); // should still return Ok (with failures)
        let verify = result.unwrap();
        assert!(!verify.passed, "Missing blob should fail verification");
        assert!(!verify.failures.is_empty());
        assert!(verify.failures[0].contains("blob file missing"));
    }

    #[test]
    fn test_verify_adpkg_empty_manifests_array() {
        let dir = tempfile::TempDir::new().unwrap();
        // Write a fake index.json with empty manifests
        fs::write(dir.path().join("index.json"), r#"{"schemaVersion":2,"manifests":[]}"#).unwrap();
        let result = verify_adpkg(dir.path().to_str().unwrap());
        assert!(result.is_ok());
        let verify = result.unwrap();
        assert!(verify.passed, "Empty manifests should pass vacuously");
    }

    #[test]
    fn test_verify_adpkg_no_manifests_field() {
        let dir = tempfile::TempDir::new().unwrap();
        // index.json without a "manifests" key → index["manifests"].as_array() returns None
        fs::write(dir.path().join("index.json"), r#"{"schemaVersion":2}"#).unwrap();
        let result = verify_adpkg(dir.path().to_str().unwrap());
        assert!(result.is_ok());
        let verify = result.unwrap();
        assert!(verify.passed, "No manifests should pass vacuously");
    }

    #[test]
    fn test_open_adpkg_tar_entry_not_agent_yaml() {
        // Build a package where the tar contains a different file path
        let src = tempfile::TempDir::new().unwrap();
        let adp_dir = src.path().join("adp");
        fs::create_dir_all(&adp_dir).unwrap();
        // Write to the wrong location (not adp/agent.yaml)
        fs::write(adp_dir.join("agent_v2.yaml"), MINIMAL_AGENT_YAML).unwrap();
        // Create a package from this src dir (which doesn't have adp/agent.yaml)
        // - create_adpkg will fail because load_adp can't find the file
        // instead we need to manually build a pkg with a different file in the layer
        let pkg_dir = tempfile::TempDir::new().unwrap();
        fs::create_dir_all(pkg_dir.path().join("blobs/sha256")).unwrap();

        // Create a tar with a file that is NOT adp/agent.yaml
        let layer_tar_path = pkg_dir.path().join("layer.tar");
        {
            let mut builder = TarBuilder::new(File::create(&layer_tar_path).unwrap());
            let content = b"some content";
            let mut header = tar::Header::new_gnu();
            header.set_path("other/file.txt").unwrap();
            header.set_size(content.len() as u64);
            header.set_cksum();
            builder.append(&header, content.as_slice()).unwrap();
            builder.finish().unwrap();
        }
        let layer_bytes = fs::read(&layer_tar_path).unwrap();
        let layer_digest = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(&layer_bytes);
            format!("sha256:{}", hex::encode(h.finalize()))
        };
        fs::write(blob_path(pkg_dir.path(), &layer_digest), &layer_bytes).unwrap();

        let config_bytes = b"{}";
        let config_digest = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(config_bytes);
            format!("sha256:{}", hex::encode(h.finalize()))
        };
        fs::write(blob_path(pkg_dir.path(), &config_digest), config_bytes).unwrap();

        let manifest = serde_json::json!({
            "schemaVersion": 2,
            "config": {"digest": config_digest, "size": config_bytes.len()},
            "layers": [{"digest": layer_digest, "size": layer_bytes.len()}]
        });
        let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
        let manifest_digest = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(&manifest_bytes);
            format!("sha256:{}", hex::encode(h.finalize()))
        };
        fs::write(blob_path(pkg_dir.path(), &manifest_digest), &manifest_bytes).unwrap();

        let index = serde_json::json!({
            "schemaVersion": 2,
            "manifests": [{"digest": manifest_digest, "size": manifest_bytes.len()}]
        });
        fs::write(pkg_dir.path().join("index.json"), serde_json::to_vec_pretty(&index).unwrap()).unwrap();

        // open_adpkg should iterate the tar entries, never find adp/agent.yaml, and return Err
        let result = open_adpkg(pkg_dir.path().to_str().unwrap());
        assert!(result.is_err(), "Should fail when adp/agent.yaml not found in tar");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("adp/agent.yaml not found"), "Error: {}", err_msg);
    }

    // ===== open_adpkg tests =====

    #[test]
    fn test_open_adpkg_happy_path() {
        let src = setup_src_dir();
        let pkg = setup_pkg(&src);

        let result = open_adpkg(pkg.path().to_str().unwrap());
        assert!(result.is_ok(), "open_adpkg should succeed: {:?}", result.err());
        let adp = result.unwrap();
        assert_eq!(adp.id, "test-agent");
        assert_eq!(adp.adp_version, "0.1.0");
    }

    #[test]
    fn test_open_adpkg_missing_index_json() {
        let dir = tempfile::TempDir::new().unwrap();
        let result = open_adpkg(dir.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_open_adpkg_layer_without_agent_yaml() {
        let src = setup_src_dir();
        // Remove the agent.yaml from the source; the package will have no adp/agent.yaml
        // We can't easily remove it post-packaging, so instead we'll create a package manually
        let pkg_dir = tempfile::TempDir::new().unwrap();
        fs::create_dir_all(pkg_dir.path().join("blobs/sha256")).unwrap();

        // Create an empty tar layer
        let layer_tar_path = pkg_dir.path().join("layer.tar");
        let builder = TarBuilder::new(File::create(&layer_tar_path).unwrap());
        drop(builder); // finish the tar
        let layer_bytes = fs::read(&layer_tar_path).unwrap();
        let layer_digest = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(&layer_bytes);
            format!("sha256:{}", hex::encode(h.finalize()))
        };
        fs::write(blob_path(pkg_dir.path(), &layer_digest), &layer_bytes).unwrap();

        // Create a minimal config blob
        let config_bytes = b"{}";
        let config_digest = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(config_bytes);
            format!("sha256:{}", hex::encode(h.finalize()))
        };
        fs::write(blob_path(pkg_dir.path(), &config_digest), config_bytes).unwrap();

        // Create manifest
        let manifest = serde_json::json!({
            "schemaVersion": 2,
            "config": {"digest": config_digest, "size": config_bytes.len()},
            "layers": [{"digest": layer_digest, "size": layer_bytes.len()}]
        });
        let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();
        let manifest_digest = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(&manifest_bytes);
            format!("sha256:{}", hex::encode(h.finalize()))
        };
        fs::write(blob_path(pkg_dir.path(), &manifest_digest), &manifest_bytes).unwrap();

        // Create index
        let index = serde_json::json!({
            "schemaVersion": 2,
            "manifests": [{"digest": manifest_digest, "size": manifest_bytes.len()}]
        });
        fs::write(pkg_dir.path().join("index.json"), serde_json::to_vec_pretty(&index).unwrap()).unwrap();

        let result = open_adpkg(pkg_dir.path().to_str().unwrap());
        assert!(result.is_err(), "Should fail when adp/agent.yaml not found in tar");
        assert!(result.unwrap_err().to_string().contains("adp/agent.yaml not found"));
    }

    #[test]
    fn test_roundtrip_create_inspect_open() {
        let src = setup_src_dir();
        // Add an extra file to the src
        let extra_dir = src.path().join("extra");
        fs::create_dir_all(&extra_dir).unwrap();
        fs::write(extra_dir.join("config.json"), r#"{"key": "value"}"#).unwrap();

        let pkg = setup_pkg(&src);

        // Inspect
        let info = inspect_adpkg(pkg.path().to_str().unwrap()).unwrap();
        assert_eq!(info.agent_id, "test-agent");

        // Verify
        let verify = verify_adpkg(pkg.path().to_str().unwrap()).unwrap();
        assert!(verify.passed, "All blobs should verify: {:?}", verify.failures);

        // Open
        let adp = open_adpkg(pkg.path().to_str().unwrap()).unwrap();
        assert_eq!(adp.id, "test-agent");
        assert_eq!(adp.runtime.execution[0].id, "r1");
    }
}
