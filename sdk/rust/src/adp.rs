use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeEntry {
    pub backend: String,
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Runtime {
    pub execution: Vec<RuntimeEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Adp {
    pub adp_version: String,
    pub id: String,
    pub runtime: Runtime,
    pub flow: serde_yaml::Value,
    pub evaluation: serde_yaml::Value,
}

pub fn load_adp(path: &str) -> Result<Adp, serde_yaml::Error> {
    let data = fs::read_to_string(Path::new(path)).unwrap();
    serde_yaml::from_str(&data)
}
