use crate::adp::Adp;
use jsonschema::Resource;

const ADP_SCHEMA: &str = include_str!("../../../schemas/adp.schema.json");
const FLOW_SCHEMA: &str = include_str!("../../../schemas/flow.schema.json");
const RUNTIME_SCHEMA: &str = include_str!("../../../schemas/runtime.schema.json");
const EVALUATION_SCHEMA: &str = include_str!("../../../schemas/evaluation.schema.json");

pub fn validate_adp(adp: &Adp) -> Result<(), Box<dyn std::error::Error>> {
    if adp.adp_version != "0.1.0" {
        return Err(format!("adp_version must be 0.1.0, got {}", adp.adp_version).into());
    }
    if adp.id.is_empty() {
        return Err("id must not be empty".into());
    }
    if adp.runtime.execution.is_empty() {
        return Err("runtime.execution must not be empty".into());
    }
    let schema: serde_json::Value = serde_json::from_str(ADP_SCHEMA)?;
    let flow: serde_json::Value = serde_json::from_str(FLOW_SCHEMA)?;
    let runtime: serde_json::Value = serde_json::from_str(RUNTIME_SCHEMA)?;
    let evaluation: serde_json::Value = serde_json::from_str(EVALUATION_SCHEMA)?;
    let instance = serde_json::to_value(adp)?;
    let validator = jsonschema::options()
        .with_resource("https://casabre.github.io/agent-definition-protocol/schemas/flow.schema.json", Resource::from_contents(flow)?)
        .with_resource("https://casabre.github.io/agent-definition-protocol/schemas/runtime.schema.json", Resource::from_contents(runtime)?)
        .with_resource("https://casabre.github.io/agent-definition-protocol/schemas/evaluation.schema.json", Resource::from_contents(evaluation)?)
        .build(&schema)?;
    let errors: Vec<String> = validator.iter_errors(&instance).map(|e| e.to_string()).collect();
    if !errors.is_empty() {
        return Err(errors.join("; ").into());
    }
    Ok(())
}
