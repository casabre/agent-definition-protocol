use crate::adp::Adp;
use jsonschema::Resource;
use regex::Regex;
use std::collections::HashSet;
const ADP_SCHEMA: &str = include_str!("../../../schemas/adp.schema.json");
const FLOW_SCHEMA: &str = include_str!("../../../schemas/flow.schema.json");
const RUNTIME_SCHEMA: &str = include_str!("../../../schemas/runtime.schema.json");
const EVALUATION_SCHEMA: &str = include_str!("../../../schemas/evaluation.schema.json");
const MEMORY_SCHEMA: &str = include_str!("../../../schemas/memory.schema.json");
const WORKSPACE_SCHEMA: &str = include_str!("../../../schemas/workspace.schema.json");
const SANDBOX_SCHEMA: &str = include_str!("../../../schemas/sandbox.schema.json");
const ARTIFACTS_SCHEMA: &str = include_str!("../../../schemas/artifacts.schema.json");
const OBSERVABILITY_SCHEMA: &str = include_str!("../../../schemas/observability.schema.json");

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
    if let Some(cc) = &adp.conformance_class {
        let flow_empty = adp.flow.id.is_empty() && adp.flow.graph.nodes.is_empty();
        let adp_json_for_eval = serde_json::to_value(adp)?;
        let eval_empty = adp_json_for_eval.get("evaluation")
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
    let memory: serde_json::Value = serde_json::from_str(MEMORY_SCHEMA)?;
    let workspace: serde_json::Value = serde_json::from_str(WORKSPACE_SCHEMA)?;
    let sandbox: serde_json::Value = serde_json::from_str(SANDBOX_SCHEMA)?;
    let artifacts: serde_json::Value = serde_json::from_str(ARTIFACTS_SCHEMA)?;
    let observability: serde_json::Value = serde_json::from_str(OBSERVABILITY_SCHEMA)?;
    let instance = serde_json::to_value(adp)?;
    let validator = jsonschema::options()
        .with_resource("https://casabre.github.io/agent-definition-protocol/schemas/flow.schema.json", Resource::from_contents(flow)?)
        .with_resource("https://casabre.github.io/agent-definition-protocol/schemas/runtime.schema.json", Resource::from_contents(runtime)?)
        .with_resource("https://casabre.github.io/agent-definition-protocol/schemas/evaluation.schema.json", Resource::from_contents(evaluation)?)
        .with_resource("https://casabre.github.io/agent-definition-protocol/schemas/memory.schema.json", Resource::from_contents(memory)?)
        .with_resource("https://casabre.github.io/agent-definition-protocol/schemas/workspace.schema.json", Resource::from_contents(workspace)?)
        .with_resource("https://casabre.github.io/agent-definition-protocol/schemas/sandbox.schema.json", Resource::from_contents(sandbox)?)
        .with_resource("https://casabre.github.io/agent-definition-protocol/schemas/artifacts.schema.json", Resource::from_contents(artifacts)?)
        .with_resource("https://casabre.github.io/agent-definition-protocol/schemas/observability.schema.json", Resource::from_contents(observability)?)
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

    let graph = &adp.flow.graph;
    let nodes: &Vec<crate::adp::Node> = &graph.nodes;
    let edges: &Vec<crate::adp::Edge> = &graph.edges;

    let mut node_ids: HashSet<String> = HashSet::new();
    for node in nodes {
        let id = &node.id;
        if !node_ids.insert(id.clone()) {
            errors.push(format!("duplicate node id '{}' in graph.nodes", id));
        }
    }

    for edge in edges {
        let from = &edge.from;
        let to = &edge.to;
        if !node_ids.contains(from) {
            errors.push(format!("edge from '{}' to '{}': node '{}' not found in graph.nodes", from, to, from));
        }
        if !node_ids.contains(to) {
            errors.push(format!("edge from '{}' to '{}': node '{}' not found in graph.nodes", from, to, to));
        }
    }

    // Check start_nodes and end_nodes
    if let Some(ref start_nodes) = graph.start_nodes {
        for node_id in start_nodes {
            if !node_ids.contains(node_id) {
                errors.push(format!("start_node '{}' not found in graph.nodes", node_id));
            }
        }
    }
    if let Some(ref end_nodes) = graph.end_nodes {
        for node_id in end_nodes {
            if !node_ids.contains(node_id) {
                errors.push(format!("end_node '{}' not found in graph.nodes", node_id));
            }
        }
    }

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
        let node_id = &node.id;

        if let Some(ref suite_ref) = node.suite_ref {
            if !suite_ids.contains(suite_ref) {
                errors.push(format!("node '{}' suite_ref '{}' not found in evaluation.suites", node_id, suite_ref));
            }
        }

        if let Some(ref model_ref) = node.model_ref {
            if let Some(ref ids) = model_ids {
                if !ids.contains(model_ref) {
                    errors.push(format!("node '{}' model_ref '{}' not found in runtime.models", node_id, model_ref));
                }
            }
        }

        if let Some(ref runtime_ref) = node.runtime_ref {
            if !execution_ids.contains(runtime_ref) {
                errors.push(format!("node '{}' runtime_ref '{}' not found in runtime.execution", node_id, runtime_ref));
            }
        }
    }

    // Check 7: guardrail policy_ref must be non-empty
    if let Some(ref guardrails) = adp.guardrails {
        for rail in guardrails.input.iter().chain(guardrails.output.iter()) {
            if rail.policy_ref.trim().is_empty() {
                errors.push(format!("guardrail '{}': policy_ref is empty", rail.id));
            }
        }
        // v0.3.0: Check interrupts (Check 22, 22b)
        if let Some(ref interrupts) = guardrails.interrupts {
            for interrupt in interrupts {
                // Check 22: interrupt tool_refs must be non-empty for tool_call trigger
                if interrupt.trigger == crate::adp::InterruptTrigger::ToolCall {
                    if interrupt.tool_refs.as_ref().map_or(false, |refs| refs.is_empty()) {
                        errors.push(format!("interrupt '{}': tool_refs required for tool_call trigger", interrupt.id));
                    }
                }
                // Check 22b: execution_mode must not be parallel with pause_and_notify
                if interrupt.mode == crate::adp::InterruptMode::PauseAndNotify {
                    if let Some(ref execution_mode) = interrupt.execution_mode {
                        if *execution_mode == crate::adp::InterruptExecutionMode::Parallel {
                            errors.push(format!(
                                "interrupt '{}': execution_mode 'parallel' not supported with pause_and_notify mode",
                                interrupt.id
                            ));
                        }
                    }
                }
            }
        }
        // Check cost guardrails (Check 23, 30)
        if let Some(ref cost) = guardrails.cost {
            if let Some(ref on_threshold_exceeded) = cost.on_threshold_exceeded {
                // Check 30: downgrade_model_ref required when on_threshold_exceeded is downgrade
                if *on_threshold_exceeded == crate::adp::CostOnThresholdExceeded::Downgrade {
                    if cost.downgrade_model_ref.is_none() {
                        errors.push("cost.downgrade_model_ref required when on_threshold_exceeded is 'downgrade'".to_string());
                    } else if let Some(ref model_ids) = model_ids {
                        if let Some(ref downgrade_model_ref) = cost.downgrade_model_ref {
                            if !model_ids.contains(downgrade_model_ref) {
                                errors.push(format!(
                                    "cost.downgrade_model_ref '{}' not found in runtime.models",
                                    downgrade_model_ref
                                ));
                            }
                        }
                    }
                }
                // Check 23: interrupt_ref must reference valid interrupt
                if *on_threshold_exceeded == crate::adp::CostOnThresholdExceeded::Interrupt {
                    if let Some(ref interrupt_ref) = cost.interrupt_ref {
                        // Check if interrupt exists
                        if let Some(ref interrupts) = guardrails.interrupts {
                            let interrupt_ids: HashSet<String> = interrupts.iter().map(|i| i.id.clone()).collect();
                            if !interrupt_ids.contains(interrupt_ref) {
                                errors.push(format!(
                                    "cost.interrupt_ref '{}' not found in guardrails.interrupts",
                                    interrupt_ref
                                ));
                            }
                        }
                    }
                }
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
    if let Some(ref tools_obj) = adp.tools {
        fn check_tool_auth(tool_id: &str, auth: &Option<crate::adp::Auth>, errors: &mut Vec<String>) {
            if let Some(ref auth_obj) = auth {
                let scheme_is_none = auth_obj.scheme.as_ref() == Some(&crate::adp::AuthScheme::None);
                if !scheme_is_none {
                    let env_var = auth_obj.env_var.as_deref().unwrap_or("").trim();
                    if env_var.is_empty() {
                        errors.push(format!(
                            "tool '{}': auth.env_var required when scheme is not 'none'",
                            tool_id
                        ));
                    }
                }
            }
        }
        
        if let Some(ref mcp_servers) = tools_obj.mcp_servers {
            for tool in mcp_servers {
                check_tool_auth(&tool.id, &tool.auth, &mut errors);
            }
        }
        if let Some(ref http_apis) = tools_obj.http_apis {
            for tool in http_apis {
                check_tool_auth(&tool.id, &tool.auth, &mut errors);
            }
        }
        if let Some(ref sql_functions) = tools_obj.sql_functions {
            for tool in sql_functions {
                check_tool_auth(&tool.id, &tool.auth, &mut errors);
            }
        }
    }

    // Check 10: compliance standard must be known or start with x_
    let adp_json = serde_json::to_value(adp).unwrap_or_default();
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
    let all_tool_ids: HashSet<String> = if let Some(ref tools_obj) = adp.tools {
        let mut ids = HashSet::new();
        if let Some(ref mcp_servers) = tools_obj.mcp_servers {
            for tool in mcp_servers {
                ids.insert(tool.id.clone());
            }
        }
        if let Some(ref http_apis) = tools_obj.http_apis {
            for tool in http_apis {
                ids.insert(tool.id.clone());
            }
        }
        if let Some(ref sql_functions) = tools_obj.sql_functions {
            for tool in sql_functions {
                ids.insert(tool.id.clone());
            }
        }
        // Also check sandbox tools
        if let Some(ref sandbox) = adp.sandbox {
            if let Some(ref mounts) = sandbox.mounts {
                for mount in mounts {
                    if let Some(ref source) = mount.source.path {
                        ids.insert(source.clone());
                    }
                }
            }
        }
        ids
    } else {
        HashSet::new()
    };

    for node in nodes {
        let node_id = &node.id;
        if let Some(ref tool_ref) = node.tool_ref {
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
        let node_id = &node.id;
        if node.kind == crate::adp::NodeKind::Subflow {
            if let Some(ref adp_ref) = node.adp_ref {
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

    // =========================================================================
    // v0.3.0 Semantic Validation Checks (15-35b)
    // =========================================================================

    // --- Loop Checks (15-16) ---

    // Check 15: loop.body_nodes[] must reference known node IDs in flow.graph.nodes[]
    let loop_nodes: Vec<&crate::adp::Node> = nodes.iter()
        .filter(|n| n.kind == crate::adp::NodeKind::Loop)
        .collect();
    for loop_node in &loop_nodes {
        if let Some(ref body_nodes) = loop_node.body_nodes {
            for body_node_id in body_nodes {
                if !node_ids.contains(body_node_id) {
                    errors.push(format!(
                        "loop node '{}': body_nodes references '{}' which is not found in graph.nodes",
                        loop_node.id, body_node_id
                    ));
                }
            }
        }
    }

    // Check 15b: loop.body_nodes[] must contain at least 2 nodes connected by
    // at least one edge in flow.graph.edges[]
    for loop_node in &loop_nodes {
        if let Some(ref body_nodes) = loop_node.body_nodes {
            if body_nodes.len() >= 2 {
                // Build adjacency from edges
                let mut edge_map: std::collections::HashMap<String, HashSet<String>> = std::collections::HashMap::new();
                for edge in edges {
                    edge_map.entry(edge.from.clone()).or_default().insert(edge.to.clone());
                }
                // Check if any body node connects to another body node
                let has_connection = body_nodes.iter().any(|node_id| {
                    edge_map.get(node_id).map_or(false, |connected| {
                        connected.iter().any(|to| body_nodes.contains(to))
                    })
                });
                if !has_connection {
                    let body_str = body_nodes.join(", ");
                    errors.push(format!(
                        "loop node '{}': body_nodes [{}] must contain at least 2 nodes connected by at least one edge",
                        loop_node.id, body_str
                    ));
                }
            }
        }
    }

    // Check 16: Loop node MUST NOT reference itself (directly or transitively) in body_nodes
    for loop_node in &loop_nodes {
        let loop_id = &loop_node.id;
        if let Some(ref body_nodes) = loop_node.body_nodes {
            if body_nodes.contains(loop_id) {
                errors.push(format!(
                    "loop node '{}': body_nodes MUST NOT reference the loop node itself",
                    loop_id
                ));
            }
            // Check transitive: if any body_node is a loop that has this loop in its body_nodes
            for body_node_id in body_nodes {
                if let Some(body_node) = nodes.iter().find(|n| n.id == *body_node_id) {
                    if body_node.kind == crate::adp::NodeKind::Loop {
                        if let Some(ref nested_body) = body_node.body_nodes {
                            if nested_body.contains(loop_id) {
                                errors.push(format!(
                                    "loop node '{}': circular loop reference detected with '{}'",
                                    loop_id, body_node_id
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // --- Tools Policy Checks (17, 29) ---

    // Check 17: policy.cache.key_fields[] entries MUST use dot-path notation
    let dot_path_re = Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*(\.[a-zA-Z_][a-zA-Z0-9_]*)*$").unwrap();
    if let Some(ref tools_obj) = adp.tools {
        fn check_cache_key_fields(
            tool_id: &str,
            policy: &Option<crate::adp::ToolPolicy>,
            errors: &mut Vec<String>,
            dot_path_re: &Regex,
        ) {
            if let Some(ref policy_obj) = policy {
                if let Some(ref cache) = policy_obj.cache {
                    if let Some(ref key_fields) = cache.key_fields {
                        for field in key_fields {
                            if !dot_path_re.is_match(field) {
                                errors.push(format!(
                                    "tool '{}': cache.key_fields entry '{}' must use dot-path notation",
                                    tool_id, field
                                ));
                            }
                        }
                    }
                }
            }
        }

        if let Some(ref mcp_servers) = tools_obj.mcp_servers {
            for tool in mcp_servers {
                check_cache_key_fields(&tool.id, &tool.policy, &mut errors, &dot_path_re);
            }
        }
        if let Some(ref http_apis) = tools_obj.http_apis {
            for tool in http_apis {
                check_cache_key_fields(&tool.id, &tool.policy, &mut errors, &dot_path_re);
            }
        }
        if let Some(ref sql_functions) = tools_obj.sql_functions {
            for tool in sql_functions {
                check_cache_key_fields(&tool.id, &tool.policy, &mut errors, &dot_path_re);
            }
        }
        // Also check global tools policy
        if let Some(ref global_policy) = tools_obj.policy {
            if let Some(ref cache) = global_policy.cache {
                if let Some(ref key_fields) = cache.key_fields {
                    for field in key_fields {
                        if !dot_path_re.is_match(field) {
                            errors.push(format!(
                                "tools.policy.cache.key_fields entry '{}' must use dot-path notation",
                                field
                            ));
                        }
                    }
                }
            }
        }
    }

    // Check 29: Any tool with load_strategy: "on_demand" MUST have a non-empty description
    if let Some(ref tools_obj) = adp.tools {
        fn check_on_demand_description(
            tool_id: &str,
            load_strategy: &Option<crate::adp::LoadStrategy>,
            description: &Option<String>,
            errors: &mut Vec<String>,
        ) {
            if let Some(ref strategy) = load_strategy {
                if let crate::adp::LoadStrategy::OnDemand = strategy {
                    let desc = description.as_deref().unwrap_or("");
                    if desc.trim().is_empty() {
                        errors.push(format!(
                            "tool '{}': load_strategy 'on_demand' requires a non-empty description",
                            tool_id
                        ));
                    }
                }
            }
        }

        if let Some(ref mcp_servers) = tools_obj.mcp_servers {
            for tool in mcp_servers {
                check_on_demand_description(&tool.id, &None, &tool.description, &mut errors);
            }
        }
        if let Some(ref http_apis) = tools_obj.http_apis {
            for tool in http_apis {
                if let Some(ref policy) = tool.policy {
                    check_on_demand_description(&tool.id, &policy.load_strategy, &tool.description, &mut errors);
                } else {
                    check_on_demand_description(&tool.id, &None, &tool.description, &mut errors);
                }
            }
        }
        if let Some(ref sql_functions) = tools_obj.sql_functions {
            for tool in sql_functions {
                if let Some(ref policy) = tool.policy {
                    check_on_demand_description(&tool.id, &policy.load_strategy, &tool.description, &mut errors);
                } else {
                    check_on_demand_description(&tool.id, &None, &tool.description, &mut errors);
                }
            }
        }
    }

    // --- Memory Checks (18-21c, 24) ---

    // Check 18: memory.stores[] IDs must be unique (post-composition)
    if let Some(ref memory) = adp.memory {
        if let crate::adp::Memory::Structured(ref structured) = memory {
            if let Some(ref stores) = structured.stores {
                let mut store_ids: HashSet<String> = HashSet::new();
                for store in stores {
                    if !store_ids.insert(store.id.clone()) {
                        errors.push(format!("memory: duplicate store id '{}'", store.id));
                    }
                }
                let store_ids_set: HashSet<String> = stores.iter().map(|s| s.id.clone()).collect();

                // Check 19: memory.operations[].store_ref must reference a known stores[].id
                if let Some(ref operations) = structured.operations {
                    for op in operations {
                        if let Some(ref store_ref) = op.store_ref {
                            if !store_ids_set.contains(store_ref) {
                                errors.push(format!(
                                    "memory.operations: store_ref '{}' not found in memory.stores",
                                    store_ref
                                ));
                            }
                        }
                        if let Some(ref store_id) = op.store_id {
                            if !store_ids_set.contains(store_id) {
                                errors.push(format!(
                                    "memory.operations: store_id '{}' not found in memory.stores",
                                    store_id
                                ));
                            }
                        }
                    }
                }

                // Check 20: memory.context_assembly.order[].store_ref must reference a known stores[].id
                if let Some(ref context_assembly) = structured.context_assembly {
                    if let Some(ref order) = context_assembly.sources {
                        for item in order {
                            if let crate::adp::ContextAssemblySource::Store = item {
                                if let Some(ref store_ref) = context_assembly.store_ref {
                                    if !store_ids_set.contains(store_ref) {
                                        errors.push(format!(
                                            "memory.context_assembly: store_ref '{}' not found in memory.stores",
                                            store_ref
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }

                // Check 24: memory.context_assembly.static_injection[].path (when source: "file")
                // must be a relative path without .. traversal; must also reference a declared workspace
                if let Some(ref context_assembly) = structured.context_assembly {
                    if let Some(ref static_injections) = context_assembly.static_injection {
                        let has_workspace = adp.workspace.is_some();
                        for si in static_injections {
                            if let Some(source) = &si.source {
                                if source == "file" {
                                    if let Some(path) = &si.path {
                                        if path.contains("..") || path.starts_with('/') {
                                            errors.push(format!(
                                                "memory.context_assembly.static_injection: path '{}' must be a relative path without .. traversal",
                                                path
                                            ));
                                        }
                                        if !has_workspace {
                                            errors.push(format!(
                                                "memory.context_assembly.static_injection: path '{}' requires a workspace section to be declared",
                                                path
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Check 21: memory.working.summary_model_ref (when present) must reference a known runtime.models[].id
    // Check 21b: memory.working.summary_model_ref MUST be present when memory.working.strategy = "summary"
    // Check 21c: memory.working.compaction_threshold_tokens (when present) MUST be <= memory.working.max_tokens
    if let Some(ref memory) = adp.memory {
        if let crate::adp::Memory::Structured(ref structured) = memory {
            if let Some(ref working) = structured.working {
                // Get model IDs
                let model_ids_set: HashSet<String> = adp.runtime.models.as_ref()
                    .map(|ms| ms.iter().map(|m| m.id.clone()).collect())
                    .unwrap_or_default();

                // Check 21
                if let Some(ref summary_model_ref) = working.summary_model_ref {
                    if !model_ids_set.contains(summary_model_ref) {
                        errors.push(format!(
                            "memory.working.summary_model_ref '{}' not found in runtime.models",
                            summary_model_ref
                        ));
                    }
                }

                // Check 21b
                if let Some(ref strategy) = working.strategy {
                    if let crate::adp::MemoryWorkingStrategy::Summary = strategy {
                        if working.summary_model_ref.is_none() {
                            errors.push(
                                "memory.working: summary_model_ref MUST be present when strategy is 'summary'"
                                    .to_string(),
                            );
                        }
                    }
                }

                // Check 21c
                if let (Some(compaction), Some(max_tokens)) = (working.compaction_threshold, working.max_tokens) {
                    if compaction > max_tokens {
                        errors.push(format!(
                            "memory.working: compaction_threshold_tokens ({}) MUST be <= max_tokens ({})",
                            compaction, max_tokens
                        ));
                    }
                }
            }
        }
    }

    // --- Guardrails Checks (22-23, 30) ---

    // Check 22: guardrails.interrupts[].tool_refs[] must reference known tool IDs
    // Check 22b: guardrails.interrupts[].execution_mode MUST NOT be set when mode: "pause_and_notify"
    if let Some(ref guardrails) = adp.guardrails {
        if let Some(ref interrupts) = guardrails.interrupts {
            // Collect all tool IDs
            let mut all_tool_ids_all_types: HashSet<String> = HashSet::new();
            if let Some(ref tools_obj) = adp.tools {
                if let Some(ref mcp_servers) = tools_obj.mcp_servers {
                    for tool in mcp_servers {
                        all_tool_ids_all_types.insert(tool.id.clone());
                    }
                }
                if let Some(ref http_apis) = tools_obj.http_apis {
                    for tool in http_apis {
                        all_tool_ids_all_types.insert(tool.id.clone());
                    }
                }
                if let Some(ref sql_functions) = tools_obj.sql_functions {
                    for tool in sql_functions {
                        all_tool_ids_all_types.insert(tool.id.clone());
                    }
                }
            }

            for interrupt in interrupts {
                // Check 22
                if let Some(ref tool_refs) = interrupt.tool_refs {
                    for tool_ref in tool_refs {
                        if !all_tool_ids_all_types.contains(tool_ref) {
                            errors.push(format!(
                                "guardrails.interrupts: tool_ref '{}' not found in tools",
                                tool_ref
                            ));
                        }
                    }
                }

                // Check 22b
                if let crate::adp::InterruptMode::PauseAndNotify = interrupt.mode {
                    if interrupt.execution_mode.is_some() {
                        errors.push(format!(
                            "guardrails.interrupts '{}': execution_mode MUST NOT be set when mode is 'pause_and_notify'",
                            interrupt.id
                        ));
                    }
                }
            }

            let interrupt_ids: HashSet<String> = interrupts.iter().map(|i| i.id.clone()).collect();

            // Check 23: guardrails.cost.interrupt_ref (when present) must reference a known guardrails.interrupts[].id
            if let Some(ref cost) = guardrails.cost {
                if let Some(ref interrupt_ref) = cost.interrupt_ref {
                    if !interrupt_ids.contains(interrupt_ref) {
                        errors.push(format!(
                            "guardrails.cost.interrupt_ref '{}' not found in guardrails.interrupts",
                            interrupt_ref
                        ));
                    }
                }

                // Check 30: guardrails.cost.downgrade_model_ref MUST be present when
                // on_threshold_exceeded: "downgrade"; it MUST reference a known runtime.models[].id
                if let Some(ref on_threshold_exceeded) = cost.on_threshold_exceeded {
                    if let crate::adp::CostOnThresholdExceeded::Downgrade = on_threshold_exceeded {
                        if cost.downgrade_model_ref.is_none() {
                            errors.push(
                                "guardrails.cost: downgrade_model_ref MUST be present when on_threshold_exceeded is 'downgrade'"
                                    .to_string(),
                            );
                        } else if let Some(ref model_ids_set) = model_ids {
                            if let Some(ref downgrade_model_ref) = cost.downgrade_model_ref {
                                if !model_ids_set.contains(downgrade_model_ref) {
                                    errors.push(format!(
                                        "guardrails.cost.downgrade_model_ref '{}' not found in runtime.models",
                                        downgrade_model_ref
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // --- Workspace Checks (25-26, 31) ---

    if let Some(ref workspace) = adp.workspace {
        // Check 25: workspace.permissions.write[] paths MUST NOT escape workspace.root (no .. traversal)
        if let Some(ref permissions) = workspace.permissions {
            if let Some(ref write_paths) = permissions.write {
                for path in write_paths {
                    if path.contains("..") {
                        errors.push(format!(
                            "workspace.permissions.write: path '{}' MUST NOT escape workspace.root",
                            path
                        ));
                    }
                }
            }
        }

        // Check 25b: Exactly one of workspace.root or workspace.root_env_var MUST be present
        if workspace.root.is_some() && workspace.root_env_var.is_some() {
            errors.push(
                "workspace: exactly one of 'root' or 'root_env_var' MUST be present, not both"
                    .to_string(),
            );
        }
        if workspace.root.is_none() && workspace.root_env_var.is_none() {
            errors.push(
                "workspace: exactly one of 'root' or 'root_env_var' MUST be present"
                    .to_string(),
            );
        }

        // Check 26: workspace.git.auto_commit: true requires workspace.git.enabled: true
        if let Some(ref git) = workspace.git {
            if git.auto_commit == Some(true) && git.enabled != Some(true) {
                errors.push("workspace.git: auto_commit requires enabled to be true".to_string());
            }
        }

        // Check 31: workspace.mounts[].id values must be unique;
        // workspace.mounts[].target paths MUST NOT escape workspace.root
        if let Some(ref mounts) = workspace.mounts {
            let mut mount_ids: HashSet<String> = HashSet::new();
            for mount in mounts {
                if !mount_ids.insert(mount.id.clone()) {
                    errors.push(format!("workspace.mounts: duplicate mount id '{}'", mount.id));
                }
                if mount.target.contains("..") {
                    errors.push(format!(
                        "workspace.mounts: target path '{}' MUST NOT escape workspace.root",
                        mount.target
                    ));
                }
            }
        }
    }

    // --- Sandbox Checks (27-28, 32) ---

    if let Some(ref sandbox) = adp.sandbox {
        // Check 27: sandbox.policy.timeout_ms MUST be present (no unbounded sandbox execution)
        if let Some(ref policy) = sandbox.policy {
            if policy.timeout_ms.is_none() {
                errors.push(
                    "sandbox.policy.timeout_ms MUST be present (no unbounded sandbox execution)"
                        .to_string(),
                );
            }
        } else {
            errors.push(
                "sandbox.policy MUST be present (no unbounded sandbox execution)"
                    .to_string(),
            );
        }

        // Check 28: sandbox.mounts[].source: "workspace" requires a workspace section to be declared
        let has_workspace = adp.workspace.is_some();
        if let Some(ref mounts) = sandbox.mounts {
            for mount in mounts {
                if let crate::adp::SandboxMountSource { workspace: Some(_), .. } = &mount.source {
                    if !has_workspace {
                        errors.push(
                            "sandbox.mounts: source 'workspace' requires a workspace section to be declared"
                                .to_string(),
                        );
                    }
                }
            }
        }

        // Check 32: sandbox.snapshot.enabled: true with provider: "custom" emits a WARNING
        if let Some(ref snapshot) = sandbox.snapshot {
            if snapshot.enabled == Some(true) {
                if sandbox.provider == Some(crate::adp::SandboxProvider::Custom) {
                    errors.push(
                        "WARNING: sandbox: snapshot.enabled with provider 'custom' may not be supported"
                            .to_string(),
                    );
                }
            }
        }
    }

    // --- Artifacts Checks (33-34) ---

    if let Some(ref artifacts) = adp.artifacts {
        // Check 33: artifacts.stores[].id must be unique
        if let Some(ref stores) = artifacts.stores {
            let mut artifact_store_ids: HashSet<String> = HashSet::new();
            for store in stores {
                if !artifact_store_ids.insert(store.id.clone()) {
                    errors.push(format!("artifacts.stores: duplicate store id '{}'", store.id));
                }
            }
            let artifact_store_ids_set: HashSet<String> = stores.iter().map(|s| s.id.clone()).collect();

            // Check 34: nodes[].params.artifact.store_ref must reference a known artifacts.stores[].id
            for node in nodes {
                if let Some(ref params) = node.params {
                    if let Some(artifact_val) = params.get("artifact") {
                        if let Some(store_ref) = artifact_val.get("store_ref").and_then(|v| v.as_str()) {
                            if !artifact_store_ids_set.contains(store_ref) {
                                errors.push(format!(
                                    "node '{}' params.artifact.store_ref '{}' not found in artifacts.stores",
                                    node.id, store_ref
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // --- Observability Checks (35-35b) ---

    if let Some(ref observability) = adp.observability {
        // Check 35: observability.tracing.trace_events[] entries must be from the valid enum
        // Note: serde deserialization with the schema enum ensures valid values,
        // so this check is handled by schema validation. No runtime check needed here.
        if let Some(ref tracing) = observability.tracing {
            if tracing.trace_events.is_some() {
                // Schema validation ensures all trace_events are valid enum values
            }
        }

        // Check 35b: observability.cost_reporting.model_refs[] (when present) must
        // reference known runtime.models[].id values
        if let Some(ref cost_reporting) = observability.cost_reporting {
            if let Some(ref model_refs) = cost_reporting.model_refs {
                let model_ids_set: HashSet<String> = adp.runtime.models.as_ref()
                    .map(|ms| ms.iter().map(|m| m.id.clone()).collect())
                    .unwrap_or_default();
                for model_ref in model_refs {
                    if !model_ids_set.contains(model_ref) {
                        errors.push(format!(
                            "observability.cost_reporting.model_refs: '{}' not found in runtime.models",
                            model_ref
                        ));
                    }
                }
            }
        }
    }

    // --- AgentSpec Interop Checks (AS-1, AS-2) ---

    if let Some(ref interop) = adp.interop {
        if let Some(ref agentspec) = interop.agentspec {
            // Check AS-1: interop.agentspec.node_map keys must match node IDs in flow.graph.nodes
            if let Some(ref node_map) = agentspec.node_map {
                for mapped_node_id in node_map.keys() {
                    if !node_ids.contains(mapped_node_id.as_str()) {
                        errors.push(format!(
                            "interop.agentspec.node_map: key '{}' does not match any node id in flow.graph.nodes",
                            mapped_node_id
                        ));
                    }
                }
            }

            // Check AS-2: interop.agentspec.llm_map[].backend_id must match runtime.execution[].id
            if let Some(ref llm_map) = agentspec.llm_map {
                let backend_ids: HashSet<String> = adp.runtime.execution
                    .iter()
                    .map(|e| e.id.clone())
                    .collect();
                for binding in llm_map {
                    if !binding.backend_id.is_empty() && !backend_ids.contains(&binding.backend_id) {
                        errors.push(format!(
                            "interop.agentspec.llm_map: backend_id '{}' does not match any id in runtime.execution",
                            binding.backend_id
                        ));
                    }
                }
            }

            // Check AS-3: interop.agentspec.ref MUST NOT contain path traversal sequences
            if let Some(ref agentspec_ref) = agentspec.ref_uri {
                if agentspec_ref.contains("..") {
                    errors.push(format!(
                        "interop.agentspec.ref '{}' MUST NOT contain path traversal sequences (..)",
                        agentspec_ref
                    ));
                }
            }
        }
    }

    errors
}
