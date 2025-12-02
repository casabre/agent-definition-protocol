use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct RuntimeEntry {
    pub backend: String,
    pub id: String,
    pub entrypoint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub provider: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_yaml::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Runtime {
    pub execution: Vec<RuntimeEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<Model>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Adp {
    pub adp_version: String,
    pub id: String,
    pub runtime: Runtime,
    pub flow: serde_yaml::Value,
    pub evaluation: serde_yaml::Value,
}

pub fn load_adp(path: &str) -> Result<Adp, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(Path::new(path))?;
    Ok(serde_yaml::from_str(&data)?)
}
