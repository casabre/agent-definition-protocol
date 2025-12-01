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

#[test]
fn test_create_and_open() {
    let tmp = tempdir().unwrap();
    build_source(tmp.path());
    let oci_dir = tmp.path().join("oci");
    create_adpkg(tmp.path().to_str().unwrap(), oci_dir.to_str().unwrap()).unwrap();

    // Validate manifest wiring and that output directory is not recursively packaged.
    let index_path = oci_dir.join("index.json");
    assert!(index_path.exists());
    let index: serde_json::Value = serde_json::from_slice(&fs::read(&index_path).unwrap()).unwrap();
    let manifest_digest = index["manifests"][0]["digest"].as_str().unwrap();
    let manifest_blob = adp_sdk::adpkg::blob_path(&oci_dir, manifest_digest);
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_blob).unwrap()).unwrap();
    let layer_digest = manifest["layers"][0]["digest"].as_str().unwrap();
    let layer_blob = adp_sdk::adpkg::blob_path(&oci_dir, layer_digest);
    let mut archive = tar::Archive::new(fs::File::open(&layer_blob).unwrap());
    let mut names = vec![];
    for entry in archive.entries().unwrap() {
        let e = entry.unwrap();
        let path = e.path().unwrap().to_path_buf();
        names.push(path);
    }
    assert!(names.contains(&std::path::Path::new("adp/agent.yaml").to_path_buf()));
    assert!(!names.iter().any(|p| p.starts_with("oci")));

    let adp = open_adpkg(oci_dir.to_str().unwrap()).unwrap();
    assert_eq!(adp.id, "agent.test");
}
