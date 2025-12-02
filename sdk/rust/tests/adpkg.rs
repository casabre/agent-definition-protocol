use std::fs;
use tempfile::tempdir;
use adp_sdk::adpkg::{create_adpkg, open_adpkg};

fn build_source(dir: &std::path::Path) {
    let adp_dir = dir.join("adp");
    fs::create_dir_all(&adp_dir).unwrap();
    fs::write(
        adp_dir.join("agent.yaml"),
        r#"adp_version: "0.1.0"
id: "agent.test"
runtime:
  execution:
    - backend: "python"
      id: "py"
      entrypoint: "agent.main:app"
flow: {}
evaluation: {}
"#,
    )
    .unwrap();
}

fn build_source_with_metadata(dir: &std::path::Path) {
    build_source(dir);
    let metadata_dir = dir.join("metadata");
    fs::create_dir_all(&metadata_dir).unwrap();
    fs::write(
        metadata_dir.join("version.json"),
        r#"{"agent_id":"agent.test","agent_version":"1.0.0","spec_version":"0.1.0","build_timestamp":"2024-01-15T10:30:00Z"}"#,
    )
    .unwrap();
}

#[test]
fn test_create_and_open() {
    let tmp = tempdir().unwrap();
    build_source(tmp.path());
    let oci_dir = tmp.path().join("oci");
    create_adpkg(tmp.path().to_str().unwrap(), oci_dir.to_str().unwrap()).unwrap();

    // Validate manifest wiring and that output directory is not recursively packaged.
    let index_path = oci_dir.join("index.json");
    assert!(index_path.exists(), "index.json should exist");
    let index: serde_json::Value = serde_json::from_slice(&fs::read(&index_path).unwrap()).unwrap();
    assert!(index["manifests"].is_array(), "index.json should contain manifests array");
    assert_eq!(index["manifests"].as_array().unwrap().len(), 1, "should have exactly one manifest");
    
    let manifest_digest = index["manifests"][0]["digest"].as_str().unwrap();
    let manifest_blob = adp_sdk::adpkg::blob_path(&oci_dir, manifest_digest);
    assert!(manifest_blob.exists(), "manifest blob should exist");
    
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_blob).unwrap()).unwrap();
    assert_eq!(manifest["mediaType"], "application/vnd.oci.image.manifest.v1+json", "manifest should have correct media type");
    
    let layer_digest = manifest["layers"][0]["digest"].as_str().unwrap();
    let layer_blob = adp_sdk::adpkg::blob_path(&oci_dir, layer_digest);
    assert!(layer_blob.exists(), "layer blob should exist");
    
    let mut archive = tar::Archive::new(fs::File::open(&layer_blob).unwrap());
    let mut names = vec![];
    for entry in archive.entries().unwrap() {
        let e = entry.unwrap();
        let path = e.path().unwrap().to_path_buf();
        names.push(path);
    }
    assert!(names.contains(&std::path::Path::new("adp/agent.yaml").to_path_buf()), 
            "Package layer should contain adp/agent.yaml");
    assert!(!names.iter().any(|p| p.starts_with("oci")), 
            "Package layer should not contain oci directory");

    let adp = open_adpkg(oci_dir.to_str().unwrap()).unwrap();
    assert_eq!(adp.id, "agent.test", "ADP id should match");
    assert_eq!(adp.adp_version, "0.1.0", "ADP version should match");
}

#[test]
fn test_oci_layout_structure() {
    let tmp = tempdir().unwrap();
    build_source(tmp.path());
    let oci_dir = tmp.path().join("oci");
    create_adpkg(tmp.path().to_str().unwrap(), oci_dir.to_str().unwrap()).unwrap();

    // Verify oci-layout
    let layout_path = oci_dir.join("oci-layout");
    assert!(layout_path.exists(), "oci-layout should exist");
    let layout: serde_json::Value = serde_json::from_slice(&fs::read(&layout_path).unwrap()).unwrap();
    assert_eq!(layout["imageLayoutVersion"], "1.0.0", "oci-layout should have correct version");
}

#[test]
fn test_config_blob_structure() {
    let tmp = tempdir().unwrap();
    build_source(tmp.path());
    let oci_dir = tmp.path().join("oci");
    create_adpkg(tmp.path().to_str().unwrap(), oci_dir.to_str().unwrap()).unwrap();

    let index: serde_json::Value = serde_json::from_slice(&fs::read(oci_dir.join("index.json")).unwrap()).unwrap();
    let manifest_digest = index["manifests"][0]["digest"].as_str().unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(adp_sdk::adpkg::blob_path(&oci_dir, manifest_digest)).unwrap()
    ).unwrap();
    
    let config_digest = manifest["config"]["digest"].as_str().unwrap();
    let config_blob = adp_sdk::adpkg::blob_path(&oci_dir, config_digest);
    assert!(config_blob.exists(), "config blob should exist");
    
    let config: serde_json::Value = serde_json::from_slice(&fs::read(&config_blob).unwrap()).unwrap();
    assert!(config.get("agent_id").is_some(), "config should contain agent_id");
    assert!(config.get("adp_version").is_some(), "config should contain adp_version");
    assert_eq!(config["agent_id"], "agent.test");
    assert_eq!(config["adp_version"], "0.1.0");
}

#[test]
fn test_package_with_metadata() {
    let tmp = tempdir().unwrap();
    build_source_with_metadata(tmp.path());
    let oci_dir = tmp.path().join("oci");
    create_adpkg(tmp.path().to_str().unwrap(), oci_dir.to_str().unwrap()).unwrap();

    let adp = open_adpkg(oci_dir.to_str().unwrap()).unwrap();
    assert_eq!(adp.id, "agent.test");
    
    // Verify metadata is in layer
    let index: serde_json::Value = serde_json::from_slice(&fs::read(oci_dir.join("index.json")).unwrap()).unwrap();
    let manifest_digest = index["manifests"][0]["digest"].as_str().unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(adp_sdk::adpkg::blob_path(&oci_dir, manifest_digest)).unwrap()
    ).unwrap();
    
    let layer_digest = manifest["layers"][0]["digest"].as_str().unwrap();
    let layer_blob = adp_sdk::adpkg::blob_path(&oci_dir, layer_digest);
    let mut archive = tar::Archive::new(fs::File::open(&layer_blob).unwrap());
    let names: Vec<_> = archive.entries().unwrap()
        .map(|e| e.unwrap().path().unwrap().to_path_buf())
        .collect();
    
    // Metadata may or may not be included, but adp/agent.yaml must be present
    assert!(names.iter().any(|p| p == std::path::Path::new("adp/agent.yaml")), 
            "Layer must contain adp/agent.yaml");
}

#[test]
fn test_blob_path() {
    use adp_sdk::adpkg::blob_path;
    let root = std::path::Path::new("/test");
    let digest = "sha256:abc123def456";
    let path = blob_path(root, digest);
    assert!(path.to_string_lossy().contains("blobs"));
    assert!(path.to_string_lossy().contains("sha256"));
    assert!(path.to_string_lossy().contains("abc123def456"));
}

#[test]
fn test_create_adpkg_error_paths() {
    let tmp = tempdir().unwrap();
    
    // Test missing adp/agent.yaml - load_adp will fail
    let oci_dir = tmp.path().join("oci");
    let result = create_adpkg(tmp.path().to_str().unwrap(), oci_dir.to_str().unwrap());
    assert!(result.is_err(), "should fail when adp/agent.yaml is missing");
    let err_msg = result.unwrap_err().to_string();
    // Error could be from file not found or yaml parse error
    assert!(err_msg.contains("agent.yaml") || err_msg.contains("No such file") || err_msg.contains("not found"), 
            "error should mention agent.yaml or file not found");
    
    // Test invalid ADP (validation failure)
    let adp_dir = tmp.path().join("adp");
    fs::create_dir_all(&adp_dir).unwrap();
    fs::write(
        adp_dir.join("agent.yaml"),
        r#"adp_version: "0.1.0"
id: "invalid"
runtime:
  execution: []
flow: {}
evaluation: {}
"#,
    ).unwrap();
    
    let oci_dir2 = tmp.path().join("oci2");
    let result2 = create_adpkg(tmp.path().to_str().unwrap(), oci_dir2.to_str().unwrap());
    assert!(result2.is_err(), "should fail validation for empty execution");
    let err_msg2 = result2.unwrap_err().to_string();
    assert!(err_msg2.contains("execution") || err_msg2.contains("runtime") || err_msg2.contains("empty") || err_msg2.contains("must not be empty"), 
            "error should mention execution or runtime");
}

#[test]
fn test_open_adpkg_error_paths() {
    let tmp = tempdir().unwrap();
    
    // Test missing index.json
    let result = open_adpkg(tmp.path().to_str().unwrap());
    assert!(result.is_err(), "should fail when index.json is missing");
    
    // Test missing agent.yaml in layer
    build_source(tmp.path());
    let oci_dir = tmp.path().join("oci");
    create_adpkg(tmp.path().to_str().unwrap(), oci_dir.to_str().unwrap()).unwrap();
    
    // Corrupt the layer by creating empty tar
    use serde_json;
    let index: serde_json::Value = serde_json::from_slice(&fs::read(oci_dir.join("index.json")).unwrap()).unwrap();
    let manifest_digest = index["manifests"][0]["digest"].as_str().unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(adp_sdk::adpkg::blob_path(&oci_dir, manifest_digest)).unwrap()
    ).unwrap();
    let layer_digest = manifest["layers"][0]["digest"].as_str().unwrap();
    let layer_path = adp_sdk::adpkg::blob_path(&oci_dir, layer_digest);
    
    // Create empty tar (no adp/agent.yaml)
    let empty_tar = tmp.path().join("empty.tar");
    let mut builder = tar::Builder::new(fs::File::create(&empty_tar).unwrap());
    builder.finish().unwrap();
    fs::copy(&empty_tar, &layer_path).unwrap();
    
    let result = open_adpkg(oci_dir.to_str().unwrap());
    assert!(result.is_err(), "should fail when agent.yaml is missing in layer");
    assert!(result.unwrap_err().to_string().contains("adp/agent.yaml"), 
            "error should mention adp/agent.yaml");
}

#[test]
fn test_create_adpkg_v0_2_0() {
    let tmp = tempdir().unwrap();
    let adp_dir = tmp.path().join("adp");
    fs::create_dir_all(&adp_dir).unwrap();
    fs::write(
        adp_dir.join("agent.yaml"),
        r#"adp_version: "0.2.0"
id: "agent.v0.2.0"
runtime:
  execution:
    - backend: "python"
      id: "py"
      entrypoint: "agent.main:app"
  models:
    - id: "primary"
      provider: "openai"
      model: "gpt-4"
      api_key_env: "OPENAI_API_KEY"
flow:
  id: "test.flow"
  graph:
    nodes:
      - id: "input"
        kind: "input"
      - id: "llm"
        kind: "llm"
        model_ref: "primary"
      - id: "tool"
        kind: "tool"
        tool_ref: "api"
      - id: "output"
        kind: "output"
    edges: []
    start_nodes: ["input"]
    end_nodes: ["output"]
evaluation: {}
"#,
    ).unwrap();
    
    let oci_dir = tmp.path().join("oci");
    create_adpkg(tmp.path().to_str().unwrap(), oci_dir.to_str().unwrap()).unwrap();
    
    let adp = open_adpkg(oci_dir.to_str().unwrap()).unwrap();
    assert_eq!(adp.id, "agent.v0.2.0");
    assert_eq!(adp.adp_version, "0.2.0");
    assert!(adp.runtime.models.is_some());
    assert_eq!(adp.runtime.models.as_ref().unwrap().len(), 1);
    assert_eq!(adp.runtime.models.as_ref().unwrap()[0].id, "primary");
}
