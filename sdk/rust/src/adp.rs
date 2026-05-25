use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RuntimeEntry {
    pub backend: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entrypoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub backend_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_manager: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelStructuredOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_ref: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
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
    // v0.3.0 model parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_streaming_api: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_output: Option<ModelStructuredOutput>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Runtime {
    pub execution: Vec<RuntimeEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<Model>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_hints: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GuardrailRail {
    pub id: String,
    pub provider: String,
    pub policy_ref: String,
    pub mode: Option<String>,
    pub categories: Option<Vec<String>>,
    pub threshold: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Guardrails {
    pub input: Vec<GuardrailRail>,
    pub output: Vec<GuardrailRail>,
    pub on_violation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Telemetry {
    pub endpoint: Option<String>,
    pub protocol: Option<String>,
    pub service_name: Option<String>,
    pub sampling_rate: Option<f64>,
    pub required_attributes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportEntry {
    pub id: String,
    #[serde(rename = "from")]
    pub from_uri: String,
    #[serde(default)]
    pub sections: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverrideEntry {
    pub path: String,
    pub value: Option<serde_json::Value>,
    #[serde(default = "default_op")]
    pub op: String,
}

fn default_op() -> String {
    "set".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subagent {
    pub id: String,
    #[serde(rename = "ref")]
    pub ref_uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invocation_mode: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Adp {
    pub adp_version: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conformance_class: Option<String>,
    pub runtime: Runtime,
    pub flow: serde_yaml::Value,
    pub evaluation: serde_yaml::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,
    #[serde(rename = "import", skip_serializing_if = "Option::is_none")]
    pub imports: Option<Vec<ImportEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<Vec<OverrideEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guardrails: Option<Guardrails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telemetry: Option<Telemetry>,
    // v0.3.0 fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagents: Option<Vec<Subagent>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_testing: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governance: Option<serde_json::Value>,
}

impl Default for Adp {
    fn default() -> Self {
        Adp {
            adp_version: String::new(),
            id: String::new(),
            conformance_class: None,
            runtime: Runtime {
                execution: Vec::new(),
                models: None,
                adapter_hints: None,
            },
            flow: serde_yaml::Value::Null,
            evaluation: serde_yaml::Value::Null,
            extends: None,
            imports: None,
            overrides: None,
            guardrails: None,
            telemetry: None,
            subagents: None,
            hooks: None,
            pipeline: None,
            streaming: None,
            x_testing: None,
            tools: None,
            governance: None,
        }
    }
}

pub fn load_adp(path: &str) -> Result<Adp, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(Path::new(path))?;
    Ok(serde_yaml::from_str(&data)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_op_via_deserialization() {
        let entry: OverrideEntry = serde_json::from_str(r#"{"path": "/id", "value": "test"}"#).unwrap();
        assert_eq!(entry.op, "set");
    }
}
