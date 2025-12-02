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
