use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use zip::{write::FileOptions, ZipArchive, ZipWriter};

use crate::adp::{load_adp, Adp};
use crate::validation::validate_adp;

pub fn open_adpkg(path: &str) -> Result<Adp, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut adp_file = archive.by_name("adp/agent.yaml")?;
    let mut contents = String::new();
    adp_file.read_to_string(&mut contents)?;
    let adp: Adp = serde_yaml::from_str(&contents)?;
    Ok(adp)
}

pub fn create_adpkg(src_dir: &str, out_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let adp_path = Path::new(src_dir).join("adp/agent.yaml");
    let adp = load_adp(adp_path.to_str().unwrap())?;
    validate_adp(&adp)?;
    let file = File::create(out_path)?;
    let mut zip = ZipWriter::new(file);
    let opts = FileOptions::default();
    zip.start_file("adp/agent.yaml", opts)?;
    let data = std::fs::read_to_string(adp_path)?;
    zip.write_all(data.as_bytes())?;
    zip.finish()?;
    Ok(())
}
