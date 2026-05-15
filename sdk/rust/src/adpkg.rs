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
        if let Ok(path) = e.path() {
            if path == std::path::Path::new("adp/agent.yaml") {
                let mut buf = String::new();
                e.read_to_string(&mut buf)?;
                let adp: Adp = serde_yaml::from_str(&buf)?;
                return Ok(adp);
            }
        }
    }
    Err("adp/agent.yaml not found".into())
}
