use crate::adp::Adp;
use jsonschema::Resource;
use regex::Regex;
use std::collections::HashSet;

const ADP_SCHEMA: &str = include_str!("../../../schemas/adp.schema.json");
const FLOW_SCHEMA: &str = include_str!("../../../schemas/flow.schema.json");
const RUNTIME_SCHEMA: &str = include_str!("../../../schemas/runtime.schema.json");
const EVALUATION_SCHEMA: &str = include_str!("../../../schemas/evaluation.schema.json");

pub fn validate_adp(adp: &Adp) -> Result<(), Box<dyn std::error::Error>> {
    if adp.adp_version != "0.1.0" && adp.adp_version != "0.2.0" && adp.adp_version != "0.3.0" {
        return Err(format!("adp_version must be 0.1.0, 0.2.0, or 0.3.0, got {}", adp.adp_version).into());
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

const KNOWN_COMPLIANCE_STANDARDS: &[&str] = &[
    "gdpr",
    "hipaa",
    "soc2",
    "eu-ai-act",
    "iso-27001",
    "fedramp",
];

pub fn validate_adp_semantics(adp: &Adp) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();

    // Pre-composition guard
    if adp.extends.is_some() || adp.imports.is_some() {
        errors.push(
            "WARNING: manifest has unresolved composition fields (extends/import); \
             semantic validation may be incomplete — call resolve_adp() first"
                .to_string(),
        );
    }

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

    // Check 7: guardrail policy_ref must be non-empty
    if let Some(guardrails) = &adp.guardrails {
        for rail in guardrails.input.iter().chain(guardrails.output.iter()) {
            if rail.policy_ref.trim().is_empty() {
                errors.push(format!("guardrail '{}': policy_ref is empty", rail.id));
            }
        }
    }

    // Check 8: telemetry.required_attributes must match gen_ai.* or x_<vendor>.*
    if let Some(telemetry) = &adp.telemetry {
        let attr_re = Regex::new(r"^gen_ai\.[a-z0-9_.]+$|^x_[a-z0-9]+\.[a-z0-9_.]+$").unwrap();
        for attr in &telemetry.required_attributes {
            if !attr_re.is_match(attr) {
                errors.push(format!(
                    "telemetry.required_attributes: '{}' is not a valid gen_ai.* or x_<vendor>.* attribute",
                    attr
                ));
            }
        }
    }

    // Check 9: tool auth.env_var required when scheme != "none"
    let adp_json = serde_json::to_value(adp).unwrap_or_default();
    let tools = adp_json.get("tools");
    if let Some(tools_val) = tools {
        for list_key in &["mcp_servers", "http_apis", "sql_functions"] {
            if let Some(tool_list) = tools_val.get(list_key).and_then(|v| v.as_array()) {
                for tool in tool_list {
                    if let Some(auth) = tool.get("auth") {
                        let scheme = auth.get("scheme").and_then(|v| v.as_str()).unwrap_or("none");
                        if scheme != "none" {
                            let env_var = auth.get("env_var").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                            if env_var.is_empty() {
                                let tool_id = tool.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                                errors.push(format!(
                                    "tool '{}': auth.env_var required when scheme is not 'none'",
                                    tool_id
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // Check 10: compliance standard must be known or start with x_
    if let Some(governance) = adp_json.get("governance") {
        if let Some(compliance) = governance.get("compliance").and_then(|v| v.as_array()) {
            for entry in compliance {
                let standard = entry.get("standard").and_then(|v| v.as_str()).unwrap_or("");
                if !KNOWN_COMPLIANCE_STANDARDS.contains(&standard) && !standard.starts_with("x_") {
                    errors.push(format!(
                        "compliance standard '{}' is unknown; use x_<vendor>.<name> for custom standards",
                        standard
                    ));
                }
            }
        }
    }

    // Check 11: node tool_ref must match a tool ID in tools.*
    let all_tool_ids: HashSet<String> = if let Some(tools_val) = tools {
        let mut ids = HashSet::new();
        for list_key in &["mcp_servers", "http_apis", "sql_functions"] {
            if let Some(tool_list) = tools_val.get(list_key).and_then(|v| v.as_array()) {
                for tool in tool_list {
                    if let Some(id) = tool.get("id").and_then(|v| v.as_str()) {
                        ids.insert(id.to_string());
                    }
                }
            }
        }
        ids
    } else {
        HashSet::new()
    };

    for node in nodes {
        let node_id = node.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if let Some(tool_ref) = node.get("tool_ref").and_then(|v| v.as_str()) {
            if !all_tool_ids.contains(tool_ref) {
                errors.push(format!(
                    "node '{}' tool_ref '{}' not found in tools",
                    node_id, tool_ref
                ));
            }
        }
    }

    // Check 12: hooks[].node_filter entries must reference known flow node IDs
    if let Some(hooks_val) = &adp.hooks {
        if let Some(hooks_arr) = hooks_val.as_array() {
            for hook in hooks_arr {
                let event = hook.get("event").and_then(|v| v.as_str()).unwrap_or("?");
                if let Some(filter_arr) = hook.get("node_filter").and_then(|v| v.as_array()) {
                    for filter_id in filter_arr {
                        if let Some(fid) = filter_id.as_str() {
                            if !node_ids.contains(fid) {
                                errors.push(format!(
                                    "hook event '{}' node_filter '{}' does not reference a known flow node",
                                    event, fid
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // Check 13: subflow node adp_ref (non-URI/path) must resolve to subagents[].id
    let subagent_ids: HashSet<String> = adp.subagents.as_ref()
        .map(|subs| subs.iter().map(|s| s.id.clone()).collect())
        .unwrap_or_default();
    for node in nodes {
        let node_id = node.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if node.get("kind").and_then(|v| v.as_str()) == Some("subflow") {
            if let Some(adp_ref) = node.get("adp_ref").and_then(|v| v.as_str()) {
                let is_uri_or_path = adp_ref.contains("://")
                    || adp_ref.contains('/')
                    || adp_ref.ends_with(".yaml")
                    || adp_ref.ends_with(".json");
                if !is_uri_or_path && !subagent_ids.contains(adp_ref) {
                    errors.push(format!(
                        "subflow node '{}' adp_ref '{}' does not resolve to a known subagents[] entry",
                        node_id, adp_ref
                    ));
                }
            }
        }
    }

    // Check 14: evaluator_ref must resolve to known x_testing evaluator/judge ID
    if let Some(x_testing) = &adp.x_testing {
        let mut testing_evaluator_ids: HashSet<String> = HashSet::new();
        if let Some(evs) = x_testing.get("evaluators").and_then(|v| v.as_array()) {
            for ev in evs {
                if let Some(id) = ev.get("id").and_then(|v| v.as_str()) {
                    testing_evaluator_ids.insert(id.to_string());
                }
            }
        }
        if let Some(judges) = x_testing.get("judges").and_then(|v| v.as_array()) {
            for j in judges {
                if let Some(id) = j.get("id").and_then(|v| v.as_str()) {
                    testing_evaluator_ids.insert(id.to_string());
                }
            }
        }
        if !testing_evaluator_ids.is_empty() {
            for suite in suites {
                if let Some(metrics) = suite.get("metrics").and_then(|v| v.as_sequence()) {
                    for metric in metrics {
                        if let Some(eval_ref) = metric.get("evaluator_ref").and_then(|v| v.as_str()) {
                            if !testing_evaluator_ids.contains(eval_ref) {
                                let metric_id = metric.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                                errors.push(format!(
                                    "evaluator '{}' evaluator_ref '{}' does not resolve to a known x_testing evaluator",
                                    metric_id, eval_ref
                                ));
                            }
                        }
                    }
                }
            }
        }
        let has_judges = x_testing.get("judges")
            .and_then(|v| v.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false);
        let has_evaluators = x_testing.get("evaluators")
            .and_then(|v| v.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false);
        if has_judges && !has_evaluators {
            errors.push(
                "WARNING: x_testing.judges[] is deprecated; migrate to x_testing.evaluators[]"
                    .to_string(),
            );
        }
    }

    errors
}
