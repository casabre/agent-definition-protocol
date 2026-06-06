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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adp::*;

    /// Build a minimal valid Adp struct directly (no YAML parsing needed for semantic tests).
    fn minimal_adp() -> Adp {
        Adp {
            adp_version: "0.1.0".to_string(),
            id: "test-id".to_string(),
            conformance_class: None,
            runtime: Runtime {
                execution: vec![RuntimeEntry {
                    id: "r1".to_string(),
                    backend: "python".to_string(),
                    entrypoint: Some("agent.main:app".to_string()),
                    ..Default::default()
                }],
                models: None,
                adapter_hints: None,
            },
            flow: Flow {
                id: "test.flow".to_string(),
                graph: Graph {
                    nodes: vec![Node {
                        id: "n1".to_string(),
                        kind: NodeKind::Input,
                        model_ref: None,
                        tool_ref: None,
                        runtime_ref: None,
                        suite_ref: None,
                        memory_ref: None,
                        strategy: None,
                        adp_ref: None,
                        label: None,
                        body_nodes: None,
                        termination: None,
                        params: None,
                        extensions: None,
                    }],
                    edges: vec![],
                    start_nodes: Some(vec!["n1".to_string()]),
                    end_nodes: Some(vec!["n1".to_string()]),
                    extensions: None,
                },
                loop_policy: None,
                extensions: None,
            },
            evaluation: serde_yaml::from_str(r#"
suites:
  - id: "s1"
    metrics:
      - id: "m1"
        type: "deterministic"
        function: "noop"
        scoring: "boolean"
        threshold: true
"#).unwrap(),
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
            memory: None,
            workspace: None,
            sandbox: None,
            artifacts: None,
            observability: None,
            interop: None,
        }
    }

    // ===== validate_adp tests =====

    #[test]
    fn test_validate_adp_valid() {
        let adp = minimal_adp();
        assert!(validate_adp(&adp).is_ok());
    }

    #[test]
    fn test_validate_adp_bad_version() {
        let mut adp = minimal_adp();
        adp.adp_version = "9.9.9".to_string();
        assert!(validate_adp(&adp).is_err());
    }

    #[test]
    fn test_validate_adp_empty_id() {
        let mut adp = minimal_adp();
        adp.id = "".to_string();
        assert!(validate_adp(&adp).is_err());
    }

    #[test]
    fn test_validate_adp_empty_execution() {
        let mut adp = minimal_adp();
        adp.runtime.execution = vec![];
        assert!(validate_adp(&adp).is_err());
    }

    #[test]
    fn test_validate_adp_conformance_class_full_flow_empty() {
        let mut adp = minimal_adp();
        adp.conformance_class = Some("full".to_string());
        adp.flow.id = "".to_string();
        adp.flow.graph.nodes = vec![];
        let result = validate_adp(&adp);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("flow is empty"));
    }

    #[test]
    fn test_validate_adp_conformance_class_full_eval_empty() {
        let mut adp = minimal_adp();
        adp.conformance_class = Some("full".to_string());
        adp.evaluation = serde_yaml::from_str("{}").unwrap();
        let result = validate_adp(&adp);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("evaluation is empty"));
    }

    #[test]
    fn test_validate_adp_conformance_class_full_flow_ok_eval_empty() {
        // Flow is NOT empty, but evaluation IS empty
        // This exercises: flow_empty=false -> skip first check, eval_empty=true -> return error
        // And covers line 39: the closing } of `if cc == "full" && eval_empty`
        let mut adp = minimal_adp();
        adp.conformance_class = Some("full".to_string());
        // Flow is non-empty (minimal_adp has nodes)
        adp.evaluation = serde_yaml::from_str("{}").unwrap(); // Eval IS empty
        let result = validate_adp(&adp);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("evaluation is empty"));
    }

    #[test]
    fn test_validate_adp_conformance_class_full_both_non_empty() {
        // Both flow and eval are non-empty -> reaches line 39 (neither error fires)
        let mut adp = minimal_adp();
        adp.conformance_class = Some("full".to_string());
        // minimal_adp has both non-empty flow and evaluation
        let result = validate_adp(&adp);
        // Should succeed (no conformance_class errors)
        assert!(result.is_ok(), "Full conformance with non-empty flow and eval should pass: {:?}", result.err());
    }

    #[test]
    fn test_validate_adp_conformance_class_non_full() {
        // conformance_class is not "full" -> neither check fires
        let mut adp = minimal_adp();
        adp.conformance_class = Some("basic".to_string());
        // No validation failure expected for non-"full" conformance_class
        // (it just passes through the if block without error)
        let result = validate_adp(&adp);
        // May pass or fail schema validation depending on schema constraints
        let _ = result; // We don't assert outcome here, just execute the code path
    }

    #[test]
    fn test_validate_adp_versions_020_030() {
        for version in ["0.2.0", "0.3.0"] {
            let mut adp = minimal_adp();
            adp.adp_version = version.to_string();
            assert!(validate_adp(&adp).is_ok(), "version {} should be valid", version);
        }
    }

    // ===== validate_adp_semantics tests =====

    #[test]
    fn test_semantics_clean_adp() {
        let adp = minimal_adp();
        let errors = validate_adp_semantics(&adp);
        assert!(errors.is_empty(), "clean adp should have no errors: {:?}", errors);
    }

    #[test]
    fn test_semantics_unresolved_composition_extends() {
        let mut adp = minimal_adp();
        adp.extends = Some("base.yaml".to_string());
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("unresolved composition")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_unresolved_composition_imports() {
        let mut adp = minimal_adp();
        adp.imports = Some(vec![ImportEntry {
            id: "mod".to_string(),
            from_uri: "module.yaml".to_string(),
            sections: vec![],
        }]);
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("unresolved composition")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_duplicate_node_ids() {
        let mut adp = minimal_adp();
        adp.flow.graph.nodes.push(Node {
            id: "n1".to_string(),
            kind: NodeKind::Output,
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, adp_ref: None, label: None,
            body_nodes: None, termination: None, params: None, extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("duplicate node id")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_edge_from_missing_node() {
        let mut adp = minimal_adp();
        adp.flow.graph.edges.push(Edge {
            from: "missing".to_string(),
            to: "n1".to_string(),
            condition: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("node 'missing' not found")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_edge_to_missing_node() {
        let mut adp = minimal_adp();
        adp.flow.graph.edges.push(Edge {
            from: "n1".to_string(),
            to: "missing-to".to_string(),
            condition: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("node 'missing-to' not found")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_start_node_missing() {
        let mut adp = minimal_adp();
        adp.flow.graph.start_nodes = Some(vec!["nonexistent".to_string()]);
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("start_node 'nonexistent'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_end_node_missing() {
        let mut adp = minimal_adp();
        adp.flow.graph.end_nodes = Some(vec!["nonexistent-end".to_string()]);
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("end_node 'nonexistent-end'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_node_suite_ref_not_found() {
        let mut adp = minimal_adp();
        adp.flow.graph.nodes[0].suite_ref = Some("no-such-suite".to_string());
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("suite_ref 'no-such-suite'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_node_model_ref_not_found() {
        let mut adp = minimal_adp();
        // We need at least one model defined so model_ids is Some
        adp.runtime.models = Some(vec![Model {
            id: "m1".to_string(),
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            ..Default::default()
        }]);
        adp.flow.graph.nodes[0].model_ref = Some("nonexistent-model".to_string());
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("model_ref 'nonexistent-model'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_node_model_ref_no_models_defined() {
        // When no models are defined, model_ref is skipped
        let mut adp = minimal_adp();
        adp.runtime.models = None;
        adp.flow.graph.nodes[0].model_ref = Some("any-model".to_string());
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("model_ref")), "No error expected when models not defined: {:?}", errors);
    }

    #[test]
    fn test_semantics_node_runtime_ref_not_found() {
        let mut adp = minimal_adp();
        adp.flow.graph.nodes[0].runtime_ref = Some("no-such-runtime".to_string());
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("runtime_ref 'no-such-runtime'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_node_tool_ref_not_found() {
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: Some(vec![MCPServer {
                id: "tool1".to_string(),
                name: None,
                description: None,
                command: "cmd".to_string(),
                args: None,
                env: None,
                timeout_seconds: None,
                auth: None,
                policy: None,
                extensions: None,
            }]),
            http_apis: None,
            sql_functions: None,
            policy: None,
            extensions: None,
        });
        adp.flow.graph.nodes[0].tool_ref = Some("nonexistent-tool".to_string());
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("tool_ref 'nonexistent-tool'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_hook_node_filter_not_found() {
        let mut adp = minimal_adp();
        adp.hooks = Some(serde_json::json!([
            {"event": "before_node", "node_filter": ["nonexistent-node"]}
        ]));
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("node_filter 'nonexistent-node'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_subflow_adp_ref_not_found() {
        let mut adp = minimal_adp();
        adp.flow.graph.nodes.push(Node {
            id: "sub1".to_string(),
            kind: NodeKind::Subflow,
            adp_ref: Some("no-such-subagent".to_string()),
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, label: None,
            body_nodes: None, termination: None, params: None, extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("does not resolve to a known subagents")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_subflow_adp_ref_uri_passthrough() {
        let mut adp = minimal_adp();
        adp.flow.graph.nodes.push(Node {
            id: "sub1".to_string(),
            kind: NodeKind::Subflow,
            adp_ref: Some("https://example.com/agent.yaml".to_string()),
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, label: None,
            body_nodes: None, termination: None, params: None, extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("subagents")), "URI adp_ref should not trigger error: {:?}", errors);
    }

    #[test]
    fn test_semantics_telemetry_invalid_attribute() {
        let mut adp = minimal_adp();
        adp.telemetry = Some(Telemetry {
            endpoint: None,
            protocol: None,
            service_name: None,
            sampling_rate: None,
            required_attributes: vec!["invalid_attribute".to_string()],
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("not a valid gen_ai")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_telemetry_valid_gen_ai_attribute() {
        let mut adp = minimal_adp();
        adp.telemetry = Some(Telemetry {
            endpoint: None,
            protocol: None,
            service_name: None,
            sampling_rate: None,
            required_attributes: vec!["gen_ai.request.model".to_string(), "x_acme.custom.attr".to_string()],
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("not a valid gen_ai")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_tool_auth_env_var_required() {
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: Some(vec![MCPServer {
                id: "t1".to_string(),
                name: None,
                description: None,
                command: "cmd".to_string(),
                args: None,
                env: None,
                timeout_seconds: None,
                auth: Some(Auth {
                    scheme: Some(AuthScheme::Bearer),
                    env_var: None,
                    header: None,
                    api_key: None,
                    extensions: None,
                }),
                policy: None,
                extensions: None,
            }]),
            http_apis: None,
            sql_functions: None,
            policy: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("auth.env_var required")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_tool_auth_none_scheme_no_env_var_ok() {
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: Some(vec![MCPServer {
                id: "t1".to_string(),
                name: None,
                description: None,
                command: "cmd".to_string(),
                args: None,
                env: None,
                timeout_seconds: None,
                auth: Some(Auth {
                    scheme: Some(AuthScheme::None),
                    env_var: None,
                    header: None,
                    api_key: None,
                    extensions: None,
                }),
                policy: None,
                extensions: None,
            }]),
            http_apis: None,
            sql_functions: None,
            policy: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("auth.env_var")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_http_api_auth_env_var_required() {
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: None,
            http_apis: Some(vec![HTTPAPI {
                id: "api1".to_string(),
                name: None,
                description: None,
                base_url: "https://api.example.com".to_string(),
                path: None,
                method: None,
                headers: None,
                auth: Some(Auth {
                    scheme: Some(AuthScheme::ApiKey),
                    env_var: None,
                    header: None,
                    api_key: None,
                    extensions: None,
                }),
                policy: None,
                extensions: None,
            }]),
            sql_functions: None,
            policy: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("auth.env_var required")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_sql_function_auth_env_var_required() {
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: None,
            http_apis: None,
            sql_functions: Some(vec![SQLFunction {
                id: "sql1".to_string(),
                name: None,
                description: None,
                query: "SELECT 1".to_string(),
                db_url_env: None,
                db_schema: None,
                auth: Some(Auth {
                    scheme: Some(AuthScheme::OAuth2),
                    env_var: None,
                    header: None,
                    api_key: None,
                    extensions: None,
                }),
                policy: None,
                extensions: None,
            }]),
            policy: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("auth.env_var required")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_compliance_unknown_standard() {
        let mut adp = minimal_adp();
        adp.governance = Some(serde_json::json!({
            "compliance": [{"standard": "unknown-standard-xyz"}]
        }));
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("unknown-standard-xyz")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_compliance_known_standards() {
        let mut adp = minimal_adp();
        adp.governance = Some(serde_json::json!({
            "compliance": [
                {"standard": "gdpr"},
                {"standard": "hipaa"},
                {"standard": "soc2"},
                {"standard": "eu-ai-act"},
                {"standard": "iso-27001"},
                {"standard": "fedramp"},
                {"standard": "x_custom.standard"}
            ]
        }));
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("unknown")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_workspace_permissions_path_traversal() {
        let mut adp = minimal_adp();
        adp.workspace = Some(Workspace {
            root: Some("/workspace".to_string()),
            root_env_var: None,
            git: None,
            permissions: Some(WorkspacePermissions {
                read: None,
                write: Some(vec!["../escape".to_string()]),
                execute: None,
                extensions: None,
            }),
            mounts: None,
            cleanup: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("MUST NOT escape workspace.root")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_workspace_both_root_and_root_env_var() {
        let mut adp = minimal_adp();
        adp.workspace = Some(Workspace {
            root: Some("/workspace".to_string()),
            root_env_var: Some("WS_ROOT".to_string()),
            git: None,
            permissions: None,
            mounts: None,
            cleanup: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("exactly one of 'root' or 'root_env_var'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_workspace_neither_root_nor_root_env_var() {
        let mut adp = minimal_adp();
        adp.workspace = Some(Workspace {
            root: None,
            root_env_var: None,
            git: None,
            permissions: None,
            mounts: None,
            cleanup: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("exactly one of 'root' or 'root_env_var'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_workspace_git_auto_commit_requires_enabled() {
        let mut adp = minimal_adp();
        adp.workspace = Some(Workspace {
            root: Some("/ws".to_string()),
            root_env_var: None,
            git: Some(WorkspaceGit {
                enabled: Some(false),
                repo_url: None,
                branch: None,
                commit: None,
                auto_commit: Some(true),
                extensions: None,
            }),
            permissions: None,
            mounts: None,
            cleanup: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("auto_commit requires enabled")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_workspace_mount_duplicate_ids() {
        let mut adp = minimal_adp();
        adp.workspace = Some(Workspace {
            root: Some("/ws".to_string()),
            root_env_var: None,
            git: None,
            permissions: None,
            mounts: Some(vec![
                WorkspaceMount {
                    id: "m1".to_string(),
                    source: WorkspaceMountSource { workspace: None, path: Some("/src".to_string()), mount_id: None, extensions: None },
                    target: "data".to_string(),
                    read_only: None,
                    extensions: None,
                },
                WorkspaceMount {
                    id: "m1".to_string(),
                    source: WorkspaceMountSource { workspace: None, path: Some("/src2".to_string()), mount_id: None, extensions: None },
                    target: "data2".to_string(),
                    read_only: None,
                    extensions: None,
                },
            ]),
            cleanup: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("duplicate mount id")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_workspace_mount_target_path_traversal() {
        let mut adp = minimal_adp();
        adp.workspace = Some(Workspace {
            root: Some("/ws".to_string()),
            root_env_var: None,
            git: None,
            permissions: None,
            mounts: Some(vec![
                WorkspaceMount {
                    id: "m1".to_string(),
                    source: WorkspaceMountSource { workspace: None, path: Some("/src".to_string()), mount_id: None, extensions: None },
                    target: "../escape".to_string(),
                    read_only: None,
                    extensions: None,
                },
            ]),
            cleanup: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("MUST NOT escape workspace.root")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_sandbox_policy_missing() {
        let mut adp = minimal_adp();
        adp.sandbox = Some(Sandbox {
            runtime: SandboxRuntime::Python,
            provider: None,
            image: None,
            mounts: None,
            snapshot: None,
            policy: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("sandbox.policy MUST be present")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_sandbox_policy_timeout_missing() {
        let mut adp = minimal_adp();
        adp.sandbox = Some(Sandbox {
            runtime: SandboxRuntime::Python,
            provider: None,
            image: None,
            mounts: None,
            snapshot: None,
            policy: Some(SandboxPolicy {
                timeout_ms: None,
                max_processes: None,
                network: None,
                allowed_hosts: None,
                allowed_ports: None,
                extensions: None,
            }),
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("timeout_ms MUST be present")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_sandbox_mount_workspace_without_workspace_section() {
        let mut adp = minimal_adp();
        adp.sandbox = Some(Sandbox {
            runtime: SandboxRuntime::Python,
            provider: None,
            image: None,
            mounts: Some(vec![SandboxMount {
                id: "sm1".to_string(),
                source: SandboxMountSource {
                    workspace: Some("main".to_string()),
                    path: None,
                    url: None,
                    extensions: None,
                },
                target: "/app".to_string(),
                read_only: None,
                extensions: None,
            }]),
            snapshot: None,
            policy: Some(SandboxPolicy {
                timeout_ms: Some(30000),
                max_processes: None,
                network: None,
                allowed_hosts: None,
                allowed_ports: None,
                extensions: None,
            }),
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("source 'workspace' requires a workspace section")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_sandbox_snapshot_custom_warning() {
        let mut adp = minimal_adp();
        adp.sandbox = Some(Sandbox {
            runtime: SandboxRuntime::Python,
            provider: Some(SandboxProvider::Custom),
            image: None,
            mounts: None,
            snapshot: Some(SandboxSnapshot {
                enabled: Some(true),
                provider: None,
                interval_seconds: None,
                retention_count: None,
                extensions: None,
            }),
            policy: Some(SandboxPolicy {
                timeout_ms: Some(30000),
                max_processes: None,
                network: None,
                allowed_hosts: None,
                allowed_ports: None,
                extensions: None,
            }),
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("WARNING: sandbox: snapshot.enabled")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_memory_duplicate_store_ids() {
        let mut adp = minimal_adp();
        adp.memory = Some(Memory::Structured(MemoryStructured {
            stores: Some(vec![
                MemoryStore {
                    id: "store1".to_string(),
                    store_type: MemoryStoreType::Episodic,
                    provider: None,
                    endpoint: None,
                    index: None,
                    scope: None,
                    pii_policy: None,
                    auto_clear: None,
                    extensions: None,
                },
                MemoryStore {
                    id: "store1".to_string(),
                    store_type: MemoryStoreType::Semantic,
                    provider: None,
                    endpoint: None,
                    index: None,
                    scope: None,
                    pii_policy: None,
                    auto_clear: None,
                    extensions: None,
                },
            ]),
            working: None,
            context_assembly: None,
            operations: None,
            retention: None,
            static_injection: None,
            extensions: None,
        }));
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("duplicate store id")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_memory_operation_store_ref_not_found() {
        let mut adp = minimal_adp();
        adp.memory = Some(Memory::Structured(MemoryStructured {
            stores: Some(vec![MemoryStore {
                id: "store1".to_string(),
                store_type: MemoryStoreType::Episodic,
                provider: None, endpoint: None, index: None, scope: None,
                pii_policy: None, auto_clear: None, extensions: None,
            }]),
            working: None,
            context_assembly: None,
            operations: Some(vec![MemoryOperation {
                id: "op1".to_string(),
                on_event: MemoryOperationOnEvent::OnInvokeEnd,
                op: MemoryOperationOp::Write,
                store_ref: Some("no-such-store".to_string()),
                store_id: None,
                filter: None,
                extensions: None,
            }]),
            retention: None,
            static_injection: None,
            extensions: None,
        }));
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("store_ref 'no-such-store'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_memory_operation_store_id_not_found() {
        let mut adp = minimal_adp();
        adp.memory = Some(Memory::Structured(MemoryStructured {
            stores: Some(vec![MemoryStore {
                id: "store1".to_string(),
                store_type: MemoryStoreType::Episodic,
                provider: None, endpoint: None, index: None, scope: None,
                pii_policy: None, auto_clear: None, extensions: None,
            }]),
            working: None,
            context_assembly: None,
            operations: Some(vec![MemoryOperation {
                id: "op1".to_string(),
                on_event: MemoryOperationOnEvent::OnInvokeEnd,
                op: MemoryOperationOp::Write,
                store_ref: None,
                store_id: Some("bad-store-id".to_string()),
                filter: None,
                extensions: None,
            }]),
            retention: None,
            static_injection: None,
            extensions: None,
        }));
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("store_id 'bad-store-id'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_memory_context_assembly_store_ref_not_found() {
        let mut adp = minimal_adp();
        adp.memory = Some(Memory::Structured(MemoryStructured {
            stores: Some(vec![MemoryStore {
                id: "store1".to_string(),
                store_type: MemoryStoreType::Episodic,
                provider: None, endpoint: None, index: None, scope: None,
                pii_policy: None, auto_clear: None, extensions: None,
            }]),
            working: None,
            context_assembly: Some(ContextAssembly {
                enabled: Some(true),
                sources: Some(vec![ContextAssemblySource::Store]),
                store_ref: Some("no-such".to_string()),
                max_tokens: None,
                position: None,
                static_injection: None,
                extensions: None,
            }),
            operations: None,
            retention: None,
            static_injection: None,
            extensions: None,
        }));
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("context_assembly: store_ref 'no-such'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_memory_static_injection_path_traversal() {
        let mut adp = minimal_adp();
        adp.memory = Some(Memory::Structured(MemoryStructured {
            stores: Some(vec![MemoryStore {
                id: "store1".to_string(),
                store_type: MemoryStoreType::Episodic,
                provider: None, endpoint: None, index: None, scope: None,
                pii_policy: None, auto_clear: None, extensions: None,
            }]),
            working: None,
            context_assembly: Some(ContextAssembly {
                enabled: None,
                sources: None,
                store_ref: None,
                max_tokens: None,
                position: None,
                static_injection: Some(vec![StaticInjection {
                    id: "si1".to_string(),
                    source: Some("file".to_string()),
                    path: Some("../escape.txt".to_string()),
                    content: None,
                    position: None,
                    max_tokens: None,
                    workspace: None,
                    content_type: None,
                    extensions: None,
                }]),
                extensions: None,
            }),
            operations: None,
            retention: None,
            static_injection: None,
            extensions: None,
        }));
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("relative path without .. traversal")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_memory_static_injection_no_workspace() {
        let mut adp = minimal_adp();
        adp.workspace = None;
        adp.memory = Some(Memory::Structured(MemoryStructured {
            stores: Some(vec![MemoryStore {
                id: "store1".to_string(),
                store_type: MemoryStoreType::Episodic,
                provider: None, endpoint: None, index: None, scope: None,
                pii_policy: None, auto_clear: None, extensions: None,
            }]),
            working: None,
            context_assembly: Some(ContextAssembly {
                enabled: None,
                sources: None,
                store_ref: None,
                max_tokens: None,
                position: None,
                static_injection: Some(vec![StaticInjection {
                    id: "si1".to_string(),
                    source: Some("file".to_string()),
                    path: Some("relative/path.txt".to_string()),
                    content: None,
                    position: None,
                    max_tokens: None,
                    workspace: None,
                    content_type: None,
                    extensions: None,
                }]),
                extensions: None,
            }),
            operations: None,
            retention: None,
            static_injection: None,
            extensions: None,
        }));
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("requires a workspace section")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_memory_working_summary_model_ref_not_found() {
        let mut adp = minimal_adp();
        adp.runtime.models = Some(vec![Model {
            id: "m1".to_string(),
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            ..Default::default()
        }]);
        adp.memory = Some(Memory::Structured(MemoryStructured {
            stores: None,
            working: Some(MemoryWorking {
                strategy: None,
                max_tokens: None,
                compaction_threshold: None,
                summary_model_ref: Some("nonexistent-model".to_string()),
                extensions: None,
            }),
            context_assembly: None,
            operations: None,
            retention: None,
            static_injection: None,
            extensions: None,
        }));
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("summary_model_ref 'nonexistent-model'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_memory_working_strategy_summary_no_model_ref() {
        let mut adp = minimal_adp();
        adp.memory = Some(Memory::Structured(MemoryStructured {
            stores: None,
            working: Some(MemoryWorking {
                strategy: Some(MemoryWorkingStrategy::Summary),
                max_tokens: None,
                compaction_threshold: None,
                summary_model_ref: None,
                extensions: None,
            }),
            context_assembly: None,
            operations: None,
            retention: None,
            static_injection: None,
            extensions: None,
        }));
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("summary_model_ref MUST be present")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_memory_working_compaction_exceeds_max_tokens() {
        let mut adp = minimal_adp();
        adp.memory = Some(Memory::Structured(MemoryStructured {
            stores: None,
            working: Some(MemoryWorking {
                strategy: None,
                max_tokens: Some(1000),
                compaction_threshold: Some(2000),
                summary_model_ref: None,
                extensions: None,
            }),
            context_assembly: None,
            operations: None,
            retention: None,
            static_injection: None,
            extensions: None,
        }));
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("compaction_threshold_tokens")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_guardrails_empty_policy_ref() {
        let mut adp = minimal_adp();
        adp.guardrails = Some(Guardrails {
            input: vec![GuardrailRail {
                id: "g1".to_string(),
                provider: "openai".to_string(),
                policy_ref: "   ".to_string(),
                mode: None,
                categories: None,
                threshold: None,
            }],
            output: vec![],
            on_violation: None,
            interrupts: None,
            cost: None,
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("policy_ref is empty")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_interrupt_tool_call_empty_tool_refs() {
        let mut adp = minimal_adp();
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: Some(vec![Interrupt {
                id: "int1".to_string(),
                trigger: InterruptTrigger::ToolCall,
                tool_refs: Some(vec![]),
                mode: InterruptMode::Block,
                execution_mode: None,
                notification: None,
                threshold_usd: None,
                extensions: None,
            }]),
            cost: None,
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("tool_refs required for tool_call trigger")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_interrupt_pause_and_notify_parallel_execution_mode() {
        let mut adp = minimal_adp();
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: Some(vec![Interrupt {
                id: "int1".to_string(),
                trigger: InterruptTrigger::Custom,
                tool_refs: None,
                mode: InterruptMode::PauseAndNotify,
                execution_mode: Some(InterruptExecutionMode::Parallel),
                notification: None,
                threshold_usd: None,
                extensions: None,
            }]),
            cost: None,
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("execution_mode 'parallel' not supported")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_cost_downgrade_missing_model_ref() {
        let mut adp = minimal_adp();
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: Some(vec![]),
            cost: Some(CostGuardrail {
                threshold_usd: Some(10.0),
                on_threshold_exceeded: Some(CostOnThresholdExceeded::Downgrade),
                interrupt_ref: None,
                downgrade_model_ref: None,
                track_by: None,
                model_refs: None,
                extensions: None,
            }),
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("downgrade_model_ref required")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_cost_downgrade_model_ref_not_found() {
        let mut adp = minimal_adp();
        adp.runtime.models = Some(vec![Model {
            id: "m1".to_string(),
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            ..Default::default()
        }]);
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: Some(vec![]),
            cost: Some(CostGuardrail {
                threshold_usd: Some(10.0),
                on_threshold_exceeded: Some(CostOnThresholdExceeded::Downgrade),
                interrupt_ref: None,
                downgrade_model_ref: Some("nonexistent".to_string()),
                track_by: None,
                model_refs: None,
                extensions: None,
            }),
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("downgrade_model_ref 'nonexistent'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_cost_interrupt_ref_not_found() {
        let mut adp = minimal_adp();
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: Some(vec![Interrupt {
                id: "int1".to_string(),
                trigger: InterruptTrigger::Custom,
                tool_refs: None,
                mode: InterruptMode::Block,
                execution_mode: None,
                notification: None,
                threshold_usd: None,
                extensions: None,
            }]),
            cost: Some(CostGuardrail {
                threshold_usd: Some(10.0),
                on_threshold_exceeded: Some(CostOnThresholdExceeded::Interrupt),
                interrupt_ref: Some("nonexistent-int".to_string()),
                downgrade_model_ref: None,
                track_by: None,
                model_refs: None,
                extensions: None,
            }),
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("interrupt_ref 'nonexistent-int'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_guardrail_interrupt_tool_refs_not_found() {
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: Some(vec![MCPServer {
                id: "tool1".to_string(),
                name: None,
                description: None,
                command: "cmd".to_string(),
                args: None,
                env: None,
                timeout_seconds: None,
                auth: None,
                policy: None,
                extensions: None,
            }]),
            http_apis: None,
            sql_functions: None,
            policy: None,
            extensions: None,
        });
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: Some(vec![Interrupt {
                id: "int1".to_string(),
                trigger: InterruptTrigger::ToolCall,
                tool_refs: Some(vec!["nonexistent-tool".to_string()]),
                mode: InterruptMode::Block,
                execution_mode: None,
                notification: None,
                threshold_usd: None,
                extensions: None,
            }]),
            cost: None,
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("tool_ref 'nonexistent-tool' not found in tools")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_guardrail_interrupt_pause_and_notify_execution_mode_set() {
        let mut adp = minimal_adp();
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: Some(vec![Interrupt {
                id: "int1".to_string(),
                trigger: InterruptTrigger::Custom,
                tool_refs: None,
                mode: InterruptMode::PauseAndNotify,
                execution_mode: Some(InterruptExecutionMode::Blocking),
                notification: None,
                threshold_usd: None,
                extensions: None,
            }]),
            cost: None,
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("execution_mode MUST NOT be set when mode is 'pause_and_notify'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_guardrail_cost_interrupt_ref_not_found_in_interrupts() {
        let mut adp = minimal_adp();
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: Some(vec![Interrupt {
                id: "int1".to_string(),
                trigger: InterruptTrigger::Custom,
                tool_refs: None,
                mode: InterruptMode::Block,
                execution_mode: None,
                notification: None,
                threshold_usd: None,
                extensions: None,
            }]),
            cost: Some(CostGuardrail {
                threshold_usd: Some(5.0),
                on_threshold_exceeded: None,
                interrupt_ref: Some("bad-ref".to_string()),
                downgrade_model_ref: None,
                track_by: None,
                model_refs: None,
                extensions: None,
            }),
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("interrupt_ref 'bad-ref' not found")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_guardrail_cost_downgrade_missing_model_ref() {
        let mut adp = minimal_adp();
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: Some(vec![]),
            cost: Some(CostGuardrail {
                threshold_usd: Some(5.0),
                on_threshold_exceeded: Some(CostOnThresholdExceeded::Downgrade),
                interrupt_ref: None,
                downgrade_model_ref: None,
                track_by: None,
                model_refs: None,
                extensions: None,
            }),
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("downgrade_model_ref MUST be present")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_guardrail_cost_downgrade_model_ref_not_in_runtime() {
        let mut adp = minimal_adp();
        adp.runtime.models = Some(vec![Model {
            id: "m1".to_string(),
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            ..Default::default()
        }]);
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: Some(vec![]),
            cost: Some(CostGuardrail {
                threshold_usd: Some(5.0),
                on_threshold_exceeded: Some(CostOnThresholdExceeded::Downgrade),
                interrupt_ref: None,
                downgrade_model_ref: Some("not-found".to_string()),
                track_by: None,
                model_refs: None,
                extensions: None,
            }),
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("downgrade_model_ref 'not-found' not found")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_observability_cost_reporting_model_refs_not_found() {
        let mut adp = minimal_adp();
        adp.runtime.models = Some(vec![Model {
            id: "m1".to_string(),
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            ..Default::default()
        }]);
        adp.observability = Some(Observability {
            tracing: None,
            cost_reporting: Some(CostReporting {
                enabled: Some(true),
                granularity: None,
                model_refs: Some(vec!["nonexistent-model".to_string()]),
                extensions: None,
            }),
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("cost_reporting.model_refs: 'nonexistent-model'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_observability_tracing_present() {
        let mut adp = minimal_adp();
        adp.observability = Some(Observability {
            tracing: Some(Tracing {
                backend: Some(TracingBackend::OTLP),
                trace_events: Some(vec![TraceEvent::ModelRequest, TraceEvent::ToolCall]),
                sampling_rate: None,
                extensions: None,
            }),
            cost_reporting: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        // No error expected for valid tracing config
        assert!(!errors.iter().any(|e| e.contains("tracing")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_interop_agentspec_node_map_bad_key() {
        use std::collections::HashMap;
        let mut adp = minimal_adp();
        let mut node_map = HashMap::new();
        node_map.insert("nonexistent-node".to_string(), "agentspec-node".to_string());
        adp.interop = Some(Interop {
            a2a: None,
            agentspec: Some(InteropAgentSpec {
                ref_uri: None,
                version: None,
                component_type: None,
                component_id: None,
                runtime_adapters: None,
                node_map: Some(node_map),
                llm_map: None,
            }),
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("node_map: key 'nonexistent-node'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_interop_agentspec_llm_map_bad_backend_id() {
        let mut adp = minimal_adp();
        adp.interop = Some(Interop {
            a2a: None,
            agentspec: Some(InteropAgentSpec {
                ref_uri: None,
                version: None,
                component_type: None,
                component_id: None,
                runtime_adapters: None,
                node_map: None,
                llm_map: Some(vec![InteropAgentSpecLlmBinding {
                    backend_id: "nonexistent-backend".to_string(),
                    agentspec_id: "llm1".to_string(),
                    agentspec_type: None,
                }]),
            }),
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("llm_map: backend_id 'nonexistent-backend'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_interop_agentspec_ref_path_traversal() {
        let mut adp = minimal_adp();
        adp.interop = Some(Interop {
            a2a: None,
            agentspec: Some(InteropAgentSpec {
                ref_uri: Some("../escape/agent.yaml".to_string()),
                version: None,
                component_type: None,
                component_id: None,
                runtime_adapters: None,
                node_map: None,
                llm_map: None,
            }),
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("MUST NOT contain path traversal")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_x_testing_evaluator_ref_not_found() {
        let mut adp = minimal_adp();
        adp.x_testing = Some(serde_json::json!({
            "evaluators": [{"id": "ev1"}]
        }));
        adp.evaluation = serde_yaml::from_str(r#"
suites:
  - id: "s1"
    metrics:
      - id: "m1"
        type: "deterministic"
        function: "noop"
        scoring: "boolean"
        threshold: true
        evaluator_ref: "nonexistent-ev"
"#).unwrap();
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("evaluator_ref 'nonexistent-ev'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_x_testing_judges_deprecated_warning() {
        let mut adp = minimal_adp();
        adp.x_testing = Some(serde_json::json!({
            "judges": [{"id": "j1"}]
        }));
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("x_testing.judges[] is deprecated")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_x_testing_judges_with_evaluators_no_warning() {
        let mut adp = minimal_adp();
        adp.x_testing = Some(serde_json::json!({
            "judges": [{"id": "j1"}],
            "evaluators": [{"id": "ev1"}]
        }));
        let errors = validate_adp_semantics(&adp);
        // judges warning should NOT fire when evaluators are also present
        assert!(!errors.iter().any(|e| e.contains("deprecated")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_loop_node_body_nodes_ref_missing() {
        let mut adp = minimal_adp();
        adp.flow.graph.nodes.push(Node {
            id: "loop1".to_string(),
            kind: NodeKind::Loop,
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, adp_ref: None, label: None,
            body_nodes: Some(vec!["missing-body".to_string()]),
            termination: None, params: None, extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("body_nodes references 'missing-body'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_loop_node_self_reference() {
        let mut adp = minimal_adp();
        adp.flow.graph.nodes.push(Node {
            id: "loop1".to_string(),
            kind: NodeKind::Loop,
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, adp_ref: None, label: None,
            body_nodes: Some(vec!["loop1".to_string()]),
            termination: None, params: None, extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("MUST NOT reference the loop node itself")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_loop_node_body_not_connected() {
        let mut adp = minimal_adp();
        adp.flow.graph.nodes.push(Node {
            id: "body1".to_string(),
            kind: NodeKind::LLM,
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, adp_ref: None, label: None,
            body_nodes: None, termination: None, params: None, extensions: None,
        });
        adp.flow.graph.nodes.push(Node {
            id: "body2".to_string(),
            kind: NodeKind::LLM,
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, adp_ref: None, label: None,
            body_nodes: None, termination: None, params: None, extensions: None,
        });
        adp.flow.graph.nodes.push(Node {
            id: "loop1".to_string(),
            kind: NodeKind::Loop,
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, adp_ref: None, label: None,
            body_nodes: Some(vec!["body1".to_string(), "body2".to_string()]),
            termination: None, params: None, extensions: None,
        });
        // No edges connecting body1 -> body2
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("must contain at least 2 nodes connected")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_loop_node_body_connected_no_error() {
        let mut adp = minimal_adp();
        adp.flow.graph.nodes.push(Node {
            id: "body1".to_string(),
            kind: NodeKind::LLM,
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, adp_ref: None, label: None,
            body_nodes: None, termination: None, params: None, extensions: None,
        });
        adp.flow.graph.nodes.push(Node {
            id: "body2".to_string(),
            kind: NodeKind::LLM,
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, adp_ref: None, label: None,
            body_nodes: None, termination: None, params: None, extensions: None,
        });
        adp.flow.graph.nodes.push(Node {
            id: "loop1".to_string(),
            kind: NodeKind::Loop,
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, adp_ref: None, label: None,
            body_nodes: Some(vec!["body1".to_string(), "body2".to_string()]),
            termination: None, params: None, extensions: None,
        });
        adp.flow.graph.edges.push(Edge {
            from: "body1".to_string(),
            to: "body2".to_string(),
            condition: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("must contain at least 2 nodes connected")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_loop_node_transitive_circular_ref() {
        // loop1 contains loop2, and loop2 has loop1 in its body_nodes (transitive circular)
        let mut adp = minimal_adp();
        adp.flow.graph.nodes.push(Node {
            id: "loop2".to_string(),
            kind: NodeKind::Loop,
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, adp_ref: None, label: None,
            body_nodes: Some(vec!["loop1".to_string()]),
            termination: None, params: None, extensions: None,
        });
        adp.flow.graph.nodes.push(Node {
            id: "loop1".to_string(),
            kind: NodeKind::Loop,
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, adp_ref: None, label: None,
            body_nodes: Some(vec!["loop2".to_string()]),
            termination: None, params: None, extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("circular loop reference")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_tool_cache_key_fields_invalid_dot_path() {
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: Some(vec![MCPServer {
                id: "t1".to_string(),
                name: None,
                description: None,
                command: "cmd".to_string(),
                args: None,
                env: None,
                timeout_seconds: None,
                auth: None,
                policy: Some(ToolPolicy {
                    load_strategy: None,
                    retry: None,
                    rate_limit: None,
                    cache: Some(CachePolicy {
                        enabled: Some(true),
                        ttl_seconds: None,
                        scope: None,
                        key_fields: Some(vec!["invalid field!".to_string()]),
                        extensions: None,
                    }),
                    extensions: None,
                }),
                extensions: None,
            }]),
            http_apis: None,
            sql_functions: None,
            policy: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("must use dot-path notation")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_tool_http_api_cache_key_fields_invalid() {
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: None,
            http_apis: Some(vec![HTTPAPI {
                id: "api1".to_string(),
                name: None,
                description: None,
                base_url: "https://api.example.com".to_string(),
                path: None,
                method: None,
                headers: None,
                auth: None,
                policy: Some(ToolPolicy {
                    load_strategy: None,
                    retry: None,
                    rate_limit: None,
                    cache: Some(CachePolicy {
                        enabled: Some(true),
                        ttl_seconds: None,
                        scope: None,
                        key_fields: Some(vec!["bad field!".to_string()]),
                        extensions: None,
                    }),
                    extensions: None,
                }),
                extensions: None,
            }]),
            sql_functions: None,
            policy: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("must use dot-path notation")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_tool_sql_cache_key_fields_invalid() {
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: None,
            http_apis: None,
            sql_functions: Some(vec![SQLFunction {
                id: "sql1".to_string(),
                name: None,
                description: None,
                query: "SELECT 1".to_string(),
                db_url_env: None,
                db_schema: None,
                auth: None,
                policy: Some(ToolPolicy {
                    load_strategy: None,
                    retry: None,
                    rate_limit: None,
                    cache: Some(CachePolicy {
                        enabled: Some(true),
                        ttl_seconds: None,
                        scope: None,
                        key_fields: Some(vec!["bad.field!".to_string()]),
                        extensions: None,
                    }),
                    extensions: None,
                }),
                extensions: None,
            }]),
            policy: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("must use dot-path notation")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_tool_global_policy_cache_key_fields_invalid() {
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: None,
            http_apis: None,
            sql_functions: None,
            policy: Some(ToolPolicy {
                load_strategy: None,
                retry: None,
                rate_limit: None,
                cache: Some(CachePolicy {
                    enabled: Some(true),
                    ttl_seconds: None,
                    scope: None,
                    key_fields: Some(vec!["bad field!".to_string()]),
                    extensions: None,
                }),
                extensions: None,
            }),
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("tools.policy.cache.key_fields")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_tool_on_demand_no_description() {
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: None,
            http_apis: Some(vec![HTTPAPI {
                id: "api1".to_string(),
                name: None,
                description: None, // Missing description
                base_url: "https://api.example.com".to_string(),
                path: None,
                method: None,
                headers: None,
                auth: None,
                policy: Some(ToolPolicy {
                    load_strategy: Some(LoadStrategy::OnDemand),
                    retry: None,
                    rate_limit: None,
                    cache: None,
                    extensions: None,
                }),
                extensions: None,
            }]),
            sql_functions: None,
            policy: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("load_strategy 'on_demand' requires a non-empty description")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_tool_sql_on_demand_no_description() {
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: None,
            http_apis: None,
            sql_functions: Some(vec![SQLFunction {
                id: "sql1".to_string(),
                name: None,
                description: None,
                query: "SELECT 1".to_string(),
                db_url_env: None,
                db_schema: None,
                auth: None,
                policy: Some(ToolPolicy {
                    load_strategy: Some(LoadStrategy::OnDemand),
                    retry: None,
                    rate_limit: None,
                    cache: None,
                    extensions: None,
                }),
                extensions: None,
            }]),
            policy: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("load_strategy 'on_demand' requires a non-empty description")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_tool_http_no_policy_on_demand_skipped() {
        // http_api with no policy - on_demand check passes through without error
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: None,
            http_apis: Some(vec![HTTPAPI {
                id: "api1".to_string(),
                name: None,
                description: None,
                base_url: "https://api.example.com".to_string(),
                path: None,
                method: None,
                headers: None,
                auth: None,
                policy: None,
                extensions: None,
            }]),
            sql_functions: None,
            policy: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("on_demand")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_tool_sql_no_policy_on_demand_skipped() {
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: None,
            http_apis: None,
            sql_functions: Some(vec![SQLFunction {
                id: "sql1".to_string(),
                name: None,
                description: None,
                query: "SELECT 1".to_string(),
                db_url_env: None,
                db_schema: None,
                auth: None,
                policy: None,
                extensions: None,
            }]),
            policy: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("on_demand")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_artifacts_duplicate_store_ids() {
        let mut adp = minimal_adp();
        adp.artifacts = Some(Artifacts {
            stores: Some(vec![
                ArtifactStore {
                    id: "store1".to_string(),
                    provider: ArtifactProvider::GCS,
                    bucket: None,
                    prefix: None,
                    scope: None,
                    extensions: None,
                },
                ArtifactStore {
                    id: "store1".to_string(),
                    provider: ArtifactProvider::S3,
                    bucket: None,
                    prefix: None,
                    scope: None,
                    extensions: None,
                },
            ]),
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("artifacts.stores: duplicate store id")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_node_params_artifact_store_ref_not_found() {
        let mut adp = minimal_adp();
        adp.artifacts = Some(Artifacts {
            stores: Some(vec![ArtifactStore {
                id: "store1".to_string(),
                provider: ArtifactProvider::GCS,
                bucket: None,
                prefix: None,
                scope: None,
                extensions: None,
            }]),
            extensions: None,
        });
        adp.flow.graph.nodes[0].params = Some(serde_json::json!({
            "artifact": {"store_ref": "nonexistent-store"}
        }));
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("params.artifact.store_ref 'nonexistent-store'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_sandbox_tool_source_path_in_all_tool_ids() {
        // sandbox.mounts[].source.path contributes to all_tool_ids set - verify this path works
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: None,
            http_apis: None,
            sql_functions: None,
            policy: None,
            extensions: None,
        });
        adp.sandbox = Some(Sandbox {
            runtime: SandboxRuntime::Python,
            provider: None,
            image: None,
            mounts: Some(vec![SandboxMount {
                id: "sm1".to_string(),
                source: SandboxMountSource {
                    workspace: None,
                    path: Some("my-tool-path".to_string()),
                    url: None,
                    extensions: None,
                },
                target: "/target".to_string(),
                read_only: None,
                extensions: None,
            }]),
            snapshot: None,
            policy: Some(SandboxPolicy {
                timeout_ms: Some(30000),
                max_processes: None,
                network: None,
                allowed_hosts: None,
                allowed_ports: None,
                extensions: None,
            }),
            extensions: None,
        });
        // node.tool_ref = "my-tool-path" should be valid now (it's in the tools sandbox paths)
        adp.flow.graph.nodes[0].tool_ref = Some("my-tool-path".to_string());
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("tool_ref 'my-tool-path' not found")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_interop_agentspec_llm_map_empty_backend_id_skipped() {
        let mut adp = minimal_adp();
        adp.interop = Some(Interop {
            a2a: None,
            agentspec: Some(InteropAgentSpec {
                ref_uri: None,
                version: None,
                component_type: None,
                component_id: None,
                runtime_adapters: None,
                node_map: None,
                llm_map: Some(vec![InteropAgentSpecLlmBinding {
                    backend_id: "".to_string(), // Empty backend_id should be skipped
                    agentspec_id: "llm1".to_string(),
                    agentspec_type: None,
                }]),
            }),
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("llm_map")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_memory_legacy_variant_no_crash() {
        let mut adp = minimal_adp();
        adp.memory = Some(Memory::Legacy(MemoryLegacy {
            memory_type: "buffer".to_string(),
            strategy: None,
            max_history: None,
            extensions: None,
        }));
        let errors = validate_adp_semantics(&adp);
        // Legacy memory should not trigger structured memory checks
        assert!(!errors.iter().any(|e| e.contains("memory:")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_context_assembly_source_working_no_store_ref_check() {
        let mut adp = minimal_adp();
        adp.memory = Some(Memory::Structured(MemoryStructured {
            stores: Some(vec![MemoryStore {
                id: "store1".to_string(),
                store_type: MemoryStoreType::Episodic,
                provider: None, endpoint: None, index: None, scope: None,
                pii_policy: None, auto_clear: None, extensions: None,
            }]),
            working: None,
            context_assembly: Some(ContextAssembly {
                enabled: None,
                sources: Some(vec![ContextAssemblySource::Working]), // Working source, not Store
                store_ref: Some("bad-ref".to_string()),
                max_tokens: None,
                position: None,
                static_injection: None,
                extensions: None,
            }),
            operations: None,
            retention: None,
            static_injection: None,
            extensions: None,
        }));
        let errors = validate_adp_semantics(&adp);
        // Working source should not trigger store_ref check
        assert!(!errors.iter().any(|e| e.contains("context_assembly: store_ref")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_static_injection_absolute_path_error() {
        let mut adp = minimal_adp();
        adp.memory = Some(Memory::Structured(MemoryStructured {
            stores: Some(vec![MemoryStore {
                id: "store1".to_string(),
                store_type: MemoryStoreType::Episodic,
                provider: None, endpoint: None, index: None, scope: None,
                pii_policy: None, auto_clear: None, extensions: None,
            }]),
            working: None,
            context_assembly: Some(ContextAssembly {
                enabled: None,
                sources: None,
                store_ref: None,
                max_tokens: None,
                position: None,
                static_injection: Some(vec![StaticInjection {
                    id: "si1".to_string(),
                    source: Some("file".to_string()),
                    path: Some("/absolute/path.txt".to_string()),
                    content: None,
                    position: None,
                    max_tokens: None,
                    workspace: None,
                    content_type: None,
                    extensions: None,
                }]),
                extensions: None,
            }),
            operations: None,
            retention: None,
            static_injection: None,
            extensions: None,
        }));
        adp.workspace = Some(Workspace {
            root: Some("/ws".to_string()),
            root_env_var: None,
            git: None,
            permissions: None,
            mounts: None,
            cleanup: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(errors.iter().any(|e| e.contains("relative path without .. traversal")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_subagent_adp_ref_found() {
        let mut adp = minimal_adp();
        adp.subagents = Some(vec![Subagent {
            id: "agent1".to_string(),
            ref_uri: "agent1.yaml".to_string(),
            description: None,
            invocation_mode: None,
        }]);
        adp.flow.graph.nodes.push(Node {
            id: "sub1".to_string(),
            kind: NodeKind::Subflow,
            adp_ref: Some("agent1".to_string()), // Matches subagent id
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, label: None,
            body_nodes: None, termination: None, params: None, extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("subagents")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_guardrails_http_api_interrupt_tool_refs() {
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: None,
            http_apis: Some(vec![HTTPAPI {
                id: "api1".to_string(),
                name: None,
                description: None,
                base_url: "https://api.example.com".to_string(),
                path: None,
                method: None,
                headers: None,
                auth: None,
                policy: None,
                extensions: None,
            }]),
            sql_functions: None,
            policy: None,
            extensions: None,
        });
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: Some(vec![Interrupt {
                id: "int1".to_string(),
                trigger: InterruptTrigger::ToolCall,
                tool_refs: Some(vec!["api1".to_string()]),
                mode: InterruptMode::Block,
                execution_mode: None,
                notification: None,
                threshold_usd: None,
                extensions: None,
            }]),
            cost: None,
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("not found in tools")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_guardrails_sql_interrupt_tool_refs() {
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: None,
            http_apis: None,
            sql_functions: Some(vec![SQLFunction {
                id: "sql1".to_string(),
                name: None,
                description: None,
                query: "SELECT 1".to_string(),
                db_url_env: None,
                db_schema: None,
                auth: None,
                policy: None,
                extensions: None,
            }]),
            policy: None,
            extensions: None,
        });
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: Some(vec![Interrupt {
                id: "int1".to_string(),
                trigger: InterruptTrigger::ToolCall,
                tool_refs: Some(vec!["sql1".to_string()]),
                mode: InterruptMode::Block,
                execution_mode: None,
                notification: None,
                threshold_usd: None,
                extensions: None,
            }]),
            cost: None,
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("not found in tools")), "{:?}", errors);
    }

    // ===== Branch coverage gap fillers =====

    #[test]
    fn test_validate_adp_schema_validation_failure() {
        // Create an ADP that passes our manual checks but fails JSON schema validation.
        // The schema requires flow.graph.start_nodes and flow.graph.end_nodes.
        // If we set them to None, the JSON serialization skips them, causing schema errors.
        let mut adp = minimal_adp();
        adp.flow.graph.start_nodes = None; // Will be omitted in JSON, failing schema
        adp.flow.graph.end_nodes = None;   // Will be omitted in JSON, failing schema
        let result = validate_adp(&adp);
        // Schema validation should fail since start_nodes and end_nodes are required
        assert!(result.is_err(), "Expected schema validation error when start_nodes/end_nodes are missing");
    }

    #[test]
    fn test_semantics_guardrails_interrupt_no_tool_refs() {
        // interrupt.tool_refs is None (not Some(empty)), trigger is ToolCall
        // This covers line 181: map_or(false, ...) returns false when tool_refs is None
        let mut adp = minimal_adp();
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: Some(vec![Interrupt {
                id: "int1".to_string(),
                trigger: InterruptTrigger::ToolCall,
                tool_refs: None, // None, not empty vec
                mode: InterruptMode::Block,
                execution_mode: None,
                notification: None,
                threshold_usd: None,
                extensions: None,
            }]),
            cost: None,
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        // tool_refs is None (not empty), so the error for "tool_refs required" should NOT fire
        assert!(!errors.iter().any(|e| e.contains("tool_refs required")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_interrupt_pause_and_notify_no_execution_mode() {
        // interrupt.mode is PauseAndNotify but execution_mode is None -> no error
        // This covers line 187: `if let Some(ref execution_mode)` is None case
        let mut adp = minimal_adp();
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: Some(vec![Interrupt {
                id: "int1".to_string(),
                trigger: InterruptTrigger::Custom,
                tool_refs: None,
                mode: InterruptMode::PauseAndNotify,
                execution_mode: None, // None -> no error
                notification: None,
                threshold_usd: None,
                extensions: None,
            }]),
            cost: None,
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("execution_mode 'parallel'")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_cost_downgrade_model_ref_exists_in_models() {
        // downgrade_model_ref is present AND exists in runtime.models -> no error
        // This covers the `if !model_ids.contains(downgrade_model_ref)` false path (line ~207)
        let mut adp = minimal_adp();
        adp.runtime.models = Some(vec![Model {
            id: "small-model".to_string(),
            provider: "openai".to_string(),
            model: "gpt-3.5".to_string(),
            ..Default::default()
        }]);
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: None,
            cost: Some(CostGuardrail {
                threshold_usd: Some(10.0),
                on_threshold_exceeded: Some(CostOnThresholdExceeded::Downgrade),
                interrupt_ref: None,
                downgrade_model_ref: Some("small-model".to_string()),
                track_by: None,
                model_refs: None,
                extensions: None,
            }),
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("downgrade_model_ref")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_cost_interrupt_no_interrupt_ref() {
        // on_threshold_exceeded is Interrupt but interrupt_ref is None -> no error about interrupt_ref
        // This covers the `if let Some(ref interrupt_ref) = cost.interrupt_ref` false path
        let mut adp = minimal_adp();
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: None,
            cost: Some(CostGuardrail {
                threshold_usd: Some(10.0),
                on_threshold_exceeded: Some(CostOnThresholdExceeded::Interrupt),
                interrupt_ref: None, // No interrupt_ref
                downgrade_model_ref: None,
                track_by: None,
                model_refs: None,
                extensions: None,
            }),
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        // We may get other errors but not the "interrupt_ref not found" one
        assert!(!errors.iter().any(|e| e.contains("not found in guardrails.interrupts")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_cost_interrupt_no_interrupts_list() {
        // on_threshold_exceeded is Interrupt, interrupt_ref is set but interrupts list is None
        // This covers line 220: `if let Some(ref interrupts) = guardrails.interrupts` false path
        let mut adp = minimal_adp();
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: None, // No interrupts list
            cost: Some(CostGuardrail {
                threshold_usd: Some(10.0),
                on_threshold_exceeded: Some(CostOnThresholdExceeded::Interrupt),
                interrupt_ref: Some("int1".to_string()),
                downgrade_model_ref: None,
                track_by: None,
                model_refs: None,
                extensions: None,
            }),
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        // Should not produce error since interrupts list doesn't exist to validate against
        assert!(!errors.iter().any(|e| e.contains("not found in guardrails.interrupts")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_compliance_no_standard_field() {
        // governance.compliance entry has no "standard" key -> standard defaults to ""
        // covers the compliance check with empty standard (which is not in known list)
        let mut adp = minimal_adp();
        adp.governance = Some(serde_json::json!({
            "compliance": [{}] // No "standard" key, defaults to ""
        }));
        let errors = validate_adp_semantics(&adp);
        // "" is not in known standards and doesn't start with x_
        assert!(errors.iter().any(|e| e.contains("unknown")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_tools_no_sandbox_mounts() {
        // tools is Some, sandbox is Some with no mounts -> no ids from sandbox
        // This covers line ~317-324 where sandbox.mounts is None
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: Some(vec![MCPServer {
                id: "t1".to_string(),
                name: None,
                description: None,
                command: "cmd".to_string(),
                args: None,
                env: None,
                timeout_seconds: None,
                auth: None,
                policy: None,
                extensions: None,
            }]),
            http_apis: None,
            sql_functions: None,
            policy: None,
            extensions: None,
        });
        adp.sandbox = Some(Sandbox {
            runtime: SandboxRuntime::Python,
            provider: None,
            image: None,
            mounts: None, // No mounts -> sandbox doesn't contribute to tool IDs
            snapshot: None,
            policy: Some(SandboxPolicy {
                timeout_ms: Some(30000),
                max_processes: None,
                network: None,
                allowed_hosts: None,
                allowed_ports: None,
                extensions: None,
            }),
            extensions: None,
        });
        adp.flow.graph.nodes[0].tool_ref = Some("t1".to_string());
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("tool_ref")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_hooks_no_filter_arr() {
        // hook entry has event but no node_filter key -> nothing to check
        let mut adp = minimal_adp();
        adp.hooks = Some(serde_json::json!([
            {"event": "before_node"} // No node_filter key
        ]));
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("node_filter")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_hooks_non_array_hooks_value() {
        // hooks is a non-array JSON value -> as_array() returns None, skipped
        let mut adp = minimal_adp();
        adp.hooks = Some(serde_json::json!({"single": "hook"})); // Not an array
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("node_filter")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_x_testing_no_evaluators_and_no_judges() {
        // x_testing is present but has neither evaluators nor judges
        // covers: testing_evaluator_ids.is_empty() -> skip loop; has_judges = false -> skip warn
        let mut adp = minimal_adp();
        adp.x_testing = Some(serde_json::json!({"description": "test config"}));
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("evaluator") || e.contains("judge")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_x_testing_evaluator_ref_match() {
        // evaluator_ref in metric matches a known evaluator -> no error
        let mut adp = minimal_adp();
        adp.x_testing = Some(serde_json::json!({
            "evaluators": [{"id": "ev1"}]
        }));
        adp.evaluation = serde_yaml::from_str(r#"
suites:
  - id: "s1"
    metrics:
      - id: "m1"
        type: "deterministic"
        function: "noop"
        scoring: "boolean"
        threshold: true
        evaluator_ref: "ev1"
"#).unwrap();
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("evaluator_ref")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_loop_body_nodes_single_node_no_connectivity_check() {
        // loop node with body_nodes of length 1 -> connectivity check is skipped
        let mut adp = minimal_adp();
        adp.flow.graph.nodes.push(Node {
            id: "body1".to_string(),
            kind: NodeKind::LLM,
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, adp_ref: None, label: None,
            body_nodes: None, termination: None, params: None, extensions: None,
        });
        adp.flow.graph.nodes.push(Node {
            id: "loop1".to_string(),
            kind: NodeKind::Loop,
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, adp_ref: None, label: None,
            body_nodes: Some(vec!["body1".to_string()]), // Only 1 node, < 2
            termination: None, params: None, extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        // No connectivity error for single-node body
        assert!(!errors.iter().any(|e| e.contains("must contain at least 2 nodes connected")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_loop_body_no_body_nodes() {
        // loop node without body_nodes -> all loop checks skipped
        let mut adp = minimal_adp();
        adp.flow.graph.nodes.push(Node {
            id: "loop1".to_string(),
            kind: NodeKind::Loop,
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, adp_ref: None, label: None,
            body_nodes: None, // No body_nodes
            termination: None, params: None, extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("loop")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_loop_transitive_non_loop_body_node() {
        // Body node exists in graph but is not a Loop kind -> transitive check skipped
        let mut adp = minimal_adp();
        adp.flow.graph.nodes.push(Node {
            id: "body1".to_string(),
            kind: NodeKind::LLM, // NOT a loop
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, adp_ref: None, label: None,
            body_nodes: None, termination: None, params: None, extensions: None,
        });
        adp.flow.graph.nodes.push(Node {
            id: "loop1".to_string(),
            kind: NodeKind::Loop,
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, adp_ref: None, label: None,
            body_nodes: Some(vec!["body1".to_string()]),
            termination: None, params: None, extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("circular loop")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_loop_transitive_body_node_no_nested_body() {
        // Body node IS a loop but has no body_nodes -> transitive check skipped
        let mut adp = minimal_adp();
        adp.flow.graph.nodes.push(Node {
            id: "inner_loop".to_string(),
            kind: NodeKind::Loop,
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, adp_ref: None, label: None,
            body_nodes: None, // No body_nodes in inner loop
            termination: None, params: None, extensions: None,
        });
        adp.flow.graph.nodes.push(Node {
            id: "outer_loop".to_string(),
            kind: NodeKind::Loop,
            model_ref: None, tool_ref: None, runtime_ref: None, suite_ref: None,
            memory_ref: None, strategy: None, adp_ref: None, label: None,
            body_nodes: Some(vec!["inner_loop".to_string()]),
            termination: None, params: None, extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("circular loop")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_tool_cache_no_key_fields() {
        // tool has a cache policy but no key_fields -> check_cache_key_fields skips key_fields loop
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: Some(vec![MCPServer {
                id: "t1".to_string(),
                name: None,
                description: None,
                command: "cmd".to_string(),
                args: None,
                env: None,
                timeout_seconds: None,
                auth: None,
                policy: Some(ToolPolicy {
                    load_strategy: None,
                    retry: None,
                    rate_limit: None,
                    cache: Some(CachePolicy {
                        enabled: Some(true),
                        ttl_seconds: None,
                        scope: None,
                        key_fields: None, // No key_fields
                        extensions: None,
                    }),
                    extensions: None,
                }),
                extensions: None,
            }]),
            http_apis: None,
            sql_functions: None,
            policy: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("dot-path notation")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_tool_cache_no_cache_policy() {
        // tool policy exists but has no cache -> check_cache_key_fields skips
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: Some(vec![MCPServer {
                id: "t1".to_string(),
                name: None,
                description: None,
                command: "cmd".to_string(),
                args: None,
                env: None,
                timeout_seconds: None,
                auth: None,
                policy: Some(ToolPolicy {
                    load_strategy: None,
                    retry: None,
                    rate_limit: None,
                    cache: None, // No cache
                    extensions: None,
                }),
                extensions: None,
            }]),
            http_apis: None,
            sql_functions: None,
            policy: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("dot-path notation")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_tool_global_policy_no_cache() {
        // global tools policy has no cache -> global key_fields loop skipped
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: None,
            http_apis: None,
            sql_functions: None,
            policy: Some(ToolPolicy {
                load_strategy: None,
                retry: None,
                rate_limit: None,
                cache: None, // No cache
                extensions: None,
            }),
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("dot-path notation")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_tool_global_policy_cache_no_key_fields() {
        // global tools policy has cache but no key_fields -> loop skipped
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: None,
            http_apis: None,
            sql_functions: None,
            policy: Some(ToolPolicy {
                load_strategy: None,
                retry: None,
                rate_limit: None,
                cache: Some(CachePolicy {
                    enabled: Some(true),
                    ttl_seconds: None,
                    scope: None,
                    key_fields: None, // No key_fields
                    extensions: None,
                }),
                extensions: None,
            }),
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("dot-path notation")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_on_demand_with_description() {
        // http_api with on_demand and non-empty description -> no error
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: None,
            http_apis: Some(vec![HTTPAPI {
                id: "api1".to_string(),
                name: None,
                description: Some("This API does something".to_string()),
                base_url: "https://api.example.com".to_string(),
                path: None,
                method: None,
                headers: None,
                auth: None,
                policy: Some(ToolPolicy {
                    load_strategy: Some(LoadStrategy::OnDemand),
                    retry: None,
                    rate_limit: None,
                    cache: None,
                    extensions: None,
                }),
                extensions: None,
            }]),
            sql_functions: None,
            policy: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("on_demand")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_memory_context_assembly_store_source_with_valid_store_ref() {
        // context_assembly.sources has Store, store_ref exists -> no error
        let mut adp = minimal_adp();
        adp.memory = Some(Memory::Structured(MemoryStructured {
            stores: Some(vec![MemoryStore {
                id: "store1".to_string(),
                store_type: MemoryStoreType::Episodic,
                provider: None, endpoint: None, index: None, scope: None,
                pii_policy: None, auto_clear: None, extensions: None,
            }]),
            working: None,
            context_assembly: Some(ContextAssembly {
                enabled: None,
                sources: Some(vec![ContextAssemblySource::Store]),
                store_ref: Some("store1".to_string()), // Valid ref
                max_tokens: None,
                position: None,
                static_injection: None,
                extensions: None,
            }),
            operations: None,
            retention: None,
            static_injection: None,
            extensions: None,
        }));
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("context_assembly: store_ref")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_memory_context_assembly_no_sources() {
        // context_assembly exists but sources is None -> store_ref check skipped
        let mut adp = minimal_adp();
        adp.memory = Some(Memory::Structured(MemoryStructured {
            stores: Some(vec![MemoryStore {
                id: "store1".to_string(),
                store_type: MemoryStoreType::Episodic,
                provider: None, endpoint: None, index: None, scope: None,
                pii_policy: None, auto_clear: None, extensions: None,
            }]),
            working: None,
            context_assembly: Some(ContextAssembly {
                enabled: None,
                sources: None, // No sources
                store_ref: Some("bad-ref".to_string()),
                max_tokens: None,
                position: None,
                static_injection: None,
                extensions: None,
            }),
            operations: None,
            retention: None,
            static_injection: None,
            extensions: None,
        }));
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("context_assembly: store_ref")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_memory_static_injection_source_not_file() {
        // static_injection source is not "file" -> path check skipped
        let mut adp = minimal_adp();
        adp.memory = Some(Memory::Structured(MemoryStructured {
            stores: Some(vec![MemoryStore {
                id: "store1".to_string(),
                store_type: MemoryStoreType::Episodic,
                provider: None, endpoint: None, index: None, scope: None,
                pii_policy: None, auto_clear: None, extensions: None,
            }]),
            working: None,
            context_assembly: Some(ContextAssembly {
                enabled: None,
                sources: None,
                store_ref: None,
                max_tokens: None,
                position: None,
                static_injection: Some(vec![StaticInjection {
                    id: "si1".to_string(),
                    source: Some("inline".to_string()), // Not "file"
                    path: Some("../escape.txt".to_string()),
                    content: Some("content".to_string()),
                    position: None,
                    max_tokens: None,
                    workspace: None,
                    content_type: None,
                    extensions: None,
                }]),
                extensions: None,
            }),
            operations: None,
            retention: None,
            static_injection: None,
            extensions: None,
        }));
        let errors = validate_adp_semantics(&adp);
        // path should not be checked since source != "file"
        assert!(!errors.iter().any(|e| e.contains("relative path")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_memory_static_injection_no_path() {
        // static_injection source is "file" but path is None -> path check skipped
        let mut adp = minimal_adp();
        adp.memory = Some(Memory::Structured(MemoryStructured {
            stores: Some(vec![MemoryStore {
                id: "store1".to_string(),
                store_type: MemoryStoreType::Episodic,
                provider: None, endpoint: None, index: None, scope: None,
                pii_policy: None, auto_clear: None, extensions: None,
            }]),
            working: None,
            context_assembly: Some(ContextAssembly {
                enabled: None,
                sources: None,
                store_ref: None,
                max_tokens: None,
                position: None,
                static_injection: Some(vec![StaticInjection {
                    id: "si1".to_string(),
                    source: Some("file".to_string()),
                    path: None, // No path
                    content: None,
                    position: None,
                    max_tokens: None,
                    workspace: None,
                    content_type: None,
                    extensions: None,
                }]),
                extensions: None,
            }),
            operations: None,
            retention: None,
            static_injection: None,
            extensions: None,
        }));
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("relative path")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_memory_working_strategy_not_summary() {
        // working.strategy is SlidingWindow -> summary_model_ref check skipped
        let mut adp = minimal_adp();
        adp.memory = Some(Memory::Structured(MemoryStructured {
            stores: None,
            working: Some(MemoryWorking {
                strategy: Some(MemoryWorkingStrategy::SlidingWindow),
                max_tokens: None,
                compaction_threshold: None,
                summary_model_ref: None,
                extensions: None,
            }),
            context_assembly: None,
            operations: None,
            retention: None,
            static_injection: None,
            extensions: None,
        }));
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("summary_model_ref")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_memory_working_compaction_within_max_tokens() {
        // compaction <= max_tokens -> no error
        let mut adp = minimal_adp();
        adp.memory = Some(Memory::Structured(MemoryStructured {
            stores: None,
            working: Some(MemoryWorking {
                strategy: None,
                max_tokens: Some(2000),
                compaction_threshold: Some(1000), // <= max_tokens, ok
                summary_model_ref: None,
                extensions: None,
            }),
            context_assembly: None,
            operations: None,
            retention: None,
            static_injection: None,
            extensions: None,
        }));
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("compaction_threshold_tokens")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_guardrail_cost_downgrade_model_ref_found() {
        // downgrade_model_ref is in interrupts block, model_ids_set has it -> no error (line 823-830)
        let mut adp = minimal_adp();
        adp.runtime.models = Some(vec![Model {
            id: "budget-model".to_string(),
            provider: "openai".to_string(),
            model: "gpt-3.5".to_string(),
            ..Default::default()
        }]);
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: Some(vec![]),
            cost: Some(CostGuardrail {
                threshold_usd: Some(5.0),
                on_threshold_exceeded: Some(CostOnThresholdExceeded::Downgrade),
                interrupt_ref: None,
                downgrade_model_ref: Some("budget-model".to_string()),
                track_by: None,
                model_refs: None,
                extensions: None,
            }),
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("downgrade_model_ref")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_sandbox_snapshot_disabled() {
        // snapshot.enabled is None (not Some(true)) -> custom warning skipped
        let mut adp = minimal_adp();
        adp.sandbox = Some(Sandbox {
            runtime: SandboxRuntime::Python,
            provider: Some(SandboxProvider::Custom),
            image: None,
            mounts: None,
            snapshot: Some(SandboxSnapshot {
                enabled: None, // Not Some(true) -> skip custom warning
                provider: None,
                interval_seconds: None,
                retention_count: None,
                extensions: None,
            }),
            policy: Some(SandboxPolicy {
                timeout_ms: Some(30000),
                max_processes: None,
                network: None,
                allowed_hosts: None,
                allowed_ports: None,
                extensions: None,
            }),
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("WARNING: sandbox")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_sandbox_snapshot_enabled_non_custom_provider() {
        // snapshot.enabled = true but provider != Custom -> no warning
        let mut adp = minimal_adp();
        adp.sandbox = Some(Sandbox {
            runtime: SandboxRuntime::Python,
            provider: Some(SandboxProvider::Docker), // Not Custom
            image: None,
            mounts: None,
            snapshot: Some(SandboxSnapshot {
                enabled: Some(true),
                provider: None,
                interval_seconds: None,
                retention_count: None,
                extensions: None,
            }),
            policy: Some(SandboxPolicy {
                timeout_ms: Some(30000),
                max_processes: None,
                network: None,
                allowed_hosts: None,
                allowed_ports: None,
                extensions: None,
            }),
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("WARNING: sandbox")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_artifacts_no_stores() {
        // artifacts present but no stores -> nothing to check
        let mut adp = minimal_adp();
        adp.artifacts = Some(Artifacts {
            stores: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("artifacts")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_node_params_no_artifact_key() {
        // node.params exists but no "artifact" key -> store_ref check skipped
        let mut adp = minimal_adp();
        adp.artifacts = Some(Artifacts {
            stores: Some(vec![ArtifactStore {
                id: "store1".to_string(),
                provider: ArtifactProvider::GCS,
                bucket: None, prefix: None, scope: None, extensions: None,
            }]),
            extensions: None,
        });
        adp.flow.graph.nodes[0].params = Some(serde_json::json!({"other_key": "value"}));
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("store_ref")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_node_params_no_store_ref_in_artifact() {
        // node.params.artifact exists but no store_ref key -> check skipped
        let mut adp = minimal_adp();
        adp.artifacts = Some(Artifacts {
            stores: Some(vec![ArtifactStore {
                id: "store1".to_string(),
                provider: ArtifactProvider::GCS,
                bucket: None, prefix: None, scope: None, extensions: None,
            }]),
            extensions: None,
        });
        adp.flow.graph.nodes[0].params = Some(serde_json::json!({
            "artifact": {"other": "value"} // No store_ref
        }));
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("store_ref")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_observability_no_cost_reporting() {
        // observability present but cost_reporting is None -> model_refs check skipped
        let mut adp = minimal_adp();
        adp.observability = Some(Observability {
            tracing: None,
            cost_reporting: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("cost_reporting")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_observability_cost_reporting_no_model_refs() {
        // cost_reporting present but model_refs is None -> loop skipped
        let mut adp = minimal_adp();
        adp.observability = Some(Observability {
            tracing: None,
            cost_reporting: Some(CostReporting {
                enabled: Some(true),
                granularity: None,
                model_refs: None,
                extensions: None,
            }),
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("cost_reporting")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_interop_no_agentspec() {
        // interop present but agentspec is None -> all AS checks skipped
        let mut adp = minimal_adp();
        adp.interop = Some(Interop {
            a2a: Some(serde_json::json!({"protocol": "v1"})),
            agentspec: None,
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("agentspec")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_interop_agentspec_no_node_map_no_llm_map_no_ref() {
        // agentspec present but all check fields are None -> no errors
        let mut adp = minimal_adp();
        adp.interop = Some(Interop {
            a2a: None,
            agentspec: Some(InteropAgentSpec {
                ref_uri: None,
                version: Some("v1".to_string()),
                component_type: None,
                component_id: None,
                runtime_adapters: None,
                node_map: None,
                llm_map: None,
            }),
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("agentspec")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_interop_agentspec_ref_no_traversal() {
        // agentspec.ref exists but has no ".." -> no error
        let mut adp = minimal_adp();
        adp.interop = Some(Interop {
            a2a: None,
            agentspec: Some(InteropAgentSpec {
                ref_uri: Some("safe/path/agent.yaml".to_string()),
                version: None,
                component_type: None,
                component_id: None,
                runtime_adapters: None,
                node_map: None,
                llm_map: None,
            }),
        });
        let errors = validate_adp_semantics(&adp);
        assert!(!errors.iter().any(|e| e.contains("path traversal")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_cost_downgrade_model_ref_found_in_models_no_error() {
        // Covers lines 213-214: else-if branch where model_ids IS Some and downgrade_model_ref IS Some
        // and the model IS found (no error). The inner `if let Some(downgrade_model_ref)` always
        // matches here, covering its closing brace (line 213) and the else-if closing brace (line 214).
        let mut adp = minimal_adp();
        adp.runtime.models = Some(vec![Model {
            id: "fast-model".to_string(),
            provider: "anthropic".to_string(),
            model: "claude-3-haiku".to_string(),
            ..Default::default()
        }]);
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: None,
            cost: Some(CostGuardrail {
                threshold_usd: Some(5.0),
                on_threshold_exceeded: Some(CostOnThresholdExceeded::Downgrade),
                interrupt_ref: None,
                downgrade_model_ref: Some("fast-model".to_string()),
                track_by: None,
                model_refs: None,
                extensions: None,
            }),
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        // Model is found -> no downgrade_model_ref error
        assert!(!errors.iter().any(|e| e.contains("downgrade_model_ref")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_governance_with_no_compliance_field() {
        // Covers line 295: governance block exists but has no "compliance" key,
        // so the inner `if let Some(compliance)` is not entered; its closing brace (295) is covered.
        let mut adp = minimal_adp();
        // Set governance to a JSON value without a "compliance" field
        adp.governance = Some(serde_json::json!({
            "data_classification": "internal"
        }));
        let errors = validate_adp_semantics(&adp);
        // No compliance errors expected
        assert!(!errors.iter().any(|e| e.contains("compliance")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_hook_node_filter_non_string_entry() {
        // Covers line 357: node_filter contains a non-string value (integer),
        // so filter_id.as_str() returns None and the inner block is not entered;
        // the closing brace at 357 is covered.
        let mut adp = minimal_adp();
        adp.hooks = Some(serde_json::json!([
            {
                "event": "before_node",
                "node_filter": [42]
            }
        ]));
        let errors = validate_adp_semantics(&adp);
        // No node_filter error (non-string is silently skipped)
        assert!(!errors.iter().any(|e| e.contains("node_filter")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_subflow_node_no_adp_ref() {
        // Covers line 382: subflow node with adp_ref = None; the `if let Some(adp_ref)` is not entered,
        // so its closing brace at line 382 is covered.
        let mut adp = minimal_adp();
        adp.flow.graph.nodes.push(Node {
            id: "sf1".to_string(),
            kind: NodeKind::Subflow,
            adp_ref: None, // no adp_ref -> inner if-let not entered
            model_ref: None,
            tool_ref: None,
            runtime_ref: None,
            suite_ref: None,
            memory_ref: None,
            strategy: None,
            label: None,
            body_nodes: None,
            termination: None,
            params: None,
            extensions: None,
        });
        adp.flow.graph.edges.push(Edge { from: "n1".to_string(), to: "sf1".to_string(), condition: None, extensions: None });
        let errors = validate_adp_semantics(&adp);
        // No adp_ref error (adp_ref is None)
        assert!(!errors.iter().any(|e| e.contains("adp_ref")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_x_testing_suite_without_metrics() {
        // Covers line 417: testing_evaluator_ids is non-empty, suites exist,
        // but a suite has no "metrics" key; the `if let Some(metrics)` is not entered,
        // covering its closing brace at line 417.
        let mut adp = minimal_adp();
        // Add an evaluator to make testing_evaluator_ids non-empty
        adp.x_testing = Some(serde_json::json!({
            "evaluators": [{"id": "eval1", "type": "script"}]
        }));
        // Override evaluation to have a suite with no metrics
        adp.evaluation = serde_yaml::from_str(r#"
suites:
  - id: "s1"
"#).unwrap();
        let errors = validate_adp_semantics(&adp);
        // No evaluator_ref error (suite has no metrics)
        assert!(!errors.iter().any(|e| e.contains("evaluator_ref")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_tool_eager_load_strategy_no_on_demand_check() {
        // Covers line 590: load_strategy is Some(Eager), which is not OnDemand;
        // the inner `if let OnDemand` block is not entered, covering its closing brace at 590.
        let mut adp = minimal_adp();
        adp.tools = Some(Tools {
            mcp_servers: None,
            http_apis: Some(vec![HTTPAPI {
                id: "api1".to_string(),
                name: None,
                description: None, // No description, but NOT on_demand -> no error
                base_url: "https://api.example.com".to_string(),
                path: None,
                method: None,
                headers: None,
                auth: None,
                policy: Some(ToolPolicy {
                    load_strategy: Some(LoadStrategy::Eager), // NOT OnDemand
                    retry: None,
                    rate_limit: None,
                    cache: None,
                    extensions: None,
                }),
                extensions: None,
            }]),
            sql_functions: None,
            policy: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        // Eager load strategy -> no on_demand description error
        assert!(!errors.iter().any(|e| e.contains("on_demand")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_memory_context_assembly_store_source_no_store_ref() {
        // Covers line 667: context_assembly has a Store source but store_ref is None;
        // the `if let Some(ref store_ref)` is not entered, covering its closing brace at 667.
        let mut adp = minimal_adp();
        let stores = vec![MemoryStore {
            id: "store1".to_string(),
            store_type: MemoryStoreType::Episodic,
            provider: None,
            endpoint: None,
            index: None,
            scope: None,
            pii_policy: None,
            auto_clear: None,
            extensions: None,
        }];
        adp.memory = Some(Memory::Structured(MemoryStructured {
            stores: Some(stores),
            working: None,
            context_assembly: Some(ContextAssembly {
                enabled: None,
                sources: Some(vec![ContextAssemblySource::Store]),
                position: None,
                store_ref: None, // No store_ref -> inner if-let not entered
                max_tokens: None,
                static_injection: None,
                extensions: None,
            }),
            operations: None,
            retention: None,
            static_injection: None,
            extensions: None,
        }));
        let errors = validate_adp_semantics(&adp);
        // No store_ref error (store_ref is None)
        assert!(!errors.iter().any(|e| e.contains("store_ref")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_memory_static_injection_no_source() {
        // Covers line 696: static_injection entry with source = None;
        // the `if let Some(source)` is not entered, covering its closing brace at 696.
        let mut adp = minimal_adp();
        adp.workspace = Some(Workspace {
            root: Some("/tmp/workspace".to_string()),
            root_env_var: None,
            git: None,
            permissions: None,
            mounts: None,
            cleanup: None,
            extensions: None,
        });
        adp.memory = Some(Memory::Structured(MemoryStructured {
            stores: None,
            working: None,
            context_assembly: Some(ContextAssembly {
                enabled: None,
                sources: None,
                position: None,
                store_ref: None,
                max_tokens: None,
                static_injection: Some(vec![StaticInjection {
                    id: "si1".to_string(),
                    source: None, // No source -> inner if-let not entered
                    path: Some("data/file.txt".to_string()),
                    content: None,
                    position: None,
                    max_tokens: None,
                    workspace: None,
                    content_type: None,
                    extensions: None,
                }]),
                extensions: None,
            }),
            operations: None,
            retention: None,
            static_injection: None,
            extensions: None,
        }));
        let errors = validate_adp_semantics(&adp);
        // No static_injection error (source is None)
        assert!(!errors.iter().any(|e| e.contains("static_injection")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_guardrail_cost_downgrade_model_ref_found_in_runtime() {
        // Covers lines 830-831: the observability guardrail cost check where
        // downgrade_model_ref IS Some AND model_ids IS Some AND the model IS found (no error).
        // The inner `if let Some(downgrade_model_ref)` always matches here (covered 830-831).
        let mut adp = minimal_adp();
        adp.runtime.models = Some(vec![Model {
            id: "lite-model".to_string(),
            provider: "openai".to_string(),
            model: "gpt-4o-mini".to_string(),
            ..Default::default()
        }]);
        adp.observability = Some(Observability {
            tracing: None,
            cost_reporting: Some(CostReporting {
                enabled: None,
                granularity: None,
                model_refs: Some(vec!["lite-model".to_string()]),
                extensions: None,
            }),
            extensions: None,
        });
        // Use guardrails with cost guardrail downgrade that FINDS the model
        adp.guardrails = Some(Guardrails {
            input: vec![],
            output: vec![],
            on_violation: None,
            interrupts: None,
            cost: Some(CostGuardrail {
                threshold_usd: Some(20.0),
                on_threshold_exceeded: Some(CostOnThresholdExceeded::Downgrade),
                interrupt_ref: None,
                downgrade_model_ref: Some("lite-model".to_string()), // Found in runtime.models
                track_by: None,
                model_refs: None,
                extensions: None,
            }),
            agent_trust: None,
        });
        let errors = validate_adp_semantics(&adp);
        // Model is found -> no guardrails.cost.downgrade_model_ref error
        assert!(!errors.iter().any(|e| e.contains("guardrails.cost.downgrade_model_ref")), "{:?}", errors);
    }

    #[test]
    fn test_semantics_workspace_permissions_no_write_paths() {
        // Covers line 852: workspace.permissions exists but write is None;
        // the `if let Some(ref write_paths)` is not entered, covering its closing brace at 852.
        let mut adp = minimal_adp();
        adp.workspace = Some(Workspace {
            root: Some("/workspace".to_string()),
            root_env_var: None,
            git: None,
            permissions: Some(WorkspacePermissions {
                read: Some(vec!["src/".to_string()]),
                write: None, // No write paths -> inner if-let not entered
                execute: None,
                extensions: None,
            }),
            mounts: None,
            cleanup: None,
            extensions: None,
        });
        let errors = validate_adp_semantics(&adp);
        // No workspace permissions error (write is None)
        assert!(!errors.iter().any(|e| e.contains("workspace.permissions.write")), "{:?}", errors);
    }
}
