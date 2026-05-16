use crate::adp::Adp;
use jsonschema::Resource;
use std::collections::HashSet;

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

    // Conformance class enforcement
    if let Some(cc) = adp.flow.get("conformance_class").and_then(|v| v.as_str()) {
        // conformance_class lives at the top-level ADP, check via serialized form
        let _ = cc; // handled below via serde_json
    }
    let adp_json = serde_json::to_value(adp)?;
    if let Some(cc) = adp_json.get("conformance_class").and_then(|v| v.as_str()) {
        let flow_empty = adp_json.get("flow")
            .and_then(|f| f.as_object())
            .map(|o| o.is_empty())
            .unwrap_or(true);
        let eval_empty = adp_json.get("evaluation")
            .and_then(|f| f.as_object())
            .map(|o| o.is_empty())
            .unwrap_or(true);
        if cc == "full" && flow_empty {
            return Err("conformance_class 'full' declared but flow is empty".into());
        }
        if cc == "full" && eval_empty {
            return Err("conformance_class 'full' declared but evaluation is empty".into());
        }
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

pub fn validate_adp_semantics(adp: &Adp) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();

    let graph = adp.flow.get("graph");
    let nodes = graph
        .and_then(|g| g.get("nodes"))
        .and_then(|n| n.as_sequence())
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    let edges = graph
        .and_then(|g| g.get("edges"))
        .and_then(|e| e.as_sequence())
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    let mut node_ids: HashSet<String> = HashSet::new();
    for node in nodes {
        if let Some(id) = node.get("id").and_then(|v| v.as_str()) {
            if !node_ids.insert(id.to_string()) {
                errors.push(format!("duplicate node id '{}' in graph.nodes", id));
            }
        }
    }

    for edge in edges {
        let from = edge.get("from").and_then(|v| v.as_str()).unwrap_or("");
        let to = edge.get("to").and_then(|v| v.as_str()).unwrap_or("");
        if !node_ids.contains(from) {
            errors.push(format!("edge from '{}' to '{}': node '{}' not found in graph.nodes", from, to, from));
        }
        if !node_ids.contains(to) {
            errors.push(format!("edge from '{}' to '{}': node '{}' not found in graph.nodes", from, to, to));
        }
    }

    let check_list = |key: &str, label: &str, ids: &HashSet<String>| -> Vec<String> {
        let mut errs = Vec::new();
        if let Some(arr) = graph.and_then(|g| g.get(key)).and_then(|n| n.as_sequence()) {
            for item in arr {
                if let Some(id) = item.as_str() {
                    if !ids.contains(id) {
                        errs.push(format!("{} '{}' not found in graph.nodes", label, id));
                    }
                }
            }
        }
        errs
    };
    errors.extend(check_list("start_nodes", "start_node", &node_ids));
    errors.extend(check_list("end_nodes", "end_node", &node_ids));

    let suites = adp.evaluation.get("suites")
        .and_then(|s| s.as_sequence())
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    let suite_ids: HashSet<String> = suites.iter()
        .filter_map(|s| s.get("id").and_then(|v| v.as_str()))
        .map(|s| s.to_string())
        .collect();

    let model_ids: Option<HashSet<String>> = adp.runtime.models.as_ref().map(|ms| {
        ms.iter().map(|m| m.id.clone()).collect()
    });

    let execution_ids: HashSet<String> = adp.runtime.execution.iter()
        .map(|e| e.id.clone())
        .collect();

    for node in nodes {
        let node_id = node.get("id").and_then(|v| v.as_str()).unwrap_or("");

        if let Some(suite_ref) = node.get("suite_ref").and_then(|v| v.as_str()) {
            if !suite_ids.contains(suite_ref) {
                errors.push(format!("node '{}' suite_ref '{}' not found in evaluation.suites", node_id, suite_ref));
            }
        }

        if let Some(model_ref) = node.get("model_ref").and_then(|v| v.as_str()) {
            if let Some(ref ids) = model_ids {
                if !ids.contains(model_ref) {
                    errors.push(format!("node '{}' model_ref '{}' not found in runtime.models", node_id, model_ref));
                }
            }
        }

        if let Some(runtime_ref) = node.get("runtime_ref").and_then(|v| v.as_str()) {
            if !execution_ids.contains(runtime_ref) {
                errors.push(format!("node '{}' runtime_ref '{}' not found in runtime.execution", node_id, runtime_ref));
            }
        }
    }

    errors
}
