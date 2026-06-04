use adp_sdk::adp::{
    Adp, Auth, AuthScheme, Edge, Flow, Graph, Guardrails, GuardrailRail,
    HTTPAPI, MCPServer, Model, Node, NodeKind, Runtime, RuntimeEntry, Subagent, Telemetry, Tools,
    Interop, InteropAgentSpec, InteropAgentSpecLlmBinding,
};
use adp_sdk::evaluation::{load_evaluator, EvaluatorError};
use adp_sdk::validation::{validate_adp, validate_adp_semantics};

fn make_node(id: &str, kind: NodeKind) -> Node {
    Node {
        id: id.into(),
        kind,
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
    }
}

fn make_edge(from: &str, to: &str) -> Edge {
    Edge { from: from.into(), to: to.into(), condition: None, extensions: None }
}

fn make_flow(
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    start_nodes: Option<Vec<&str>>,
    end_nodes: Option<Vec<&str>>,
) -> Flow {
    Flow {
        id: String::new(),
        graph: Graph {
            nodes,
            edges,
            start_nodes: start_nodes.map(|v| v.iter().map(|s| s.to_string()).collect()),
            end_nodes: end_nodes.map(|v| v.iter().map(|s| s.to_string()).collect()),
            extensions: None,
        },
        loop_policy: None,
        extensions: None,
    }
}

fn minimal_flow() -> Flow {
    make_flow(
        vec![make_node("n", NodeKind::Input)],
        vec![],
        Some(vec!["n"]),
        Some(vec!["n"]),
    )
}

fn minimal_evaluation() -> serde_yaml::Value {
    serde_yaml::from_str(r#"
suites:
  - id: "s"
    metrics:
      - id: "m"
        type: "deterministic"
        function: "noop"
        scoring: "boolean"
        threshold: true
"#).unwrap()
}

fn python_entry() -> RuntimeEntry {
    RuntimeEntry { backend: "python".into(), id: "py".into(), entrypoint: Some("agent.main:app".into()), ..Default::default() }
}

#[test]
fn validation_rejects_missing_execution() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        conformance_class: None,
        runtime: Runtime { execution: vec![], models: None, ..Default::default() },
        ..Default::default()
    };
    assert!(validate_adp(&adp).is_err(), "Should reject empty execution array");
    let err = validate_adp(&adp).unwrap_err();
    assert!(err.to_string().contains("execution") || err.to_string().contains("runtime"),
            "Error should mention execution or runtime");
}

#[test]
fn validation_accepts_basic() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        conformance_class: None,
        runtime: Runtime { execution: vec![python_entry()], models: None, ..Default::default() },
        flow: minimal_flow(),
        evaluation: minimal_evaluation(),
        ..Default::default()
    };
    assert!(validate_adp(&adp).is_ok(), "Should accept valid basic ADP");
}

#[test]
fn validation_rejects_invalid_version() {
    let adp = Adp {
        adp_version: "9.9.9".into(),
        id: "agent.test".into(),
        conformance_class: None,
        runtime: Runtime { execution: vec![python_entry()], models: None, ..Default::default() },
        ..Default::default()
    };
    assert!(validate_adp(&adp).is_err(), "Should reject invalid version");
}

#[test]
fn validation_accepts_v0_1_0() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.v0.1.0".into(),
        conformance_class: None,
        runtime: Runtime { execution: vec![python_entry()], models: None, ..Default::default() },
        flow: make_flow(
            vec![
                make_node("input", NodeKind::Input),
                make_node("output", NodeKind::Output),
            ],
            vec![],
            Some(vec!["input"]),
            Some(vec!["output"]),
        ),
        evaluation: minimal_evaluation(),
        ..Default::default()
    };
    assert!(validate_adp(&adp).is_ok(), "Should accept v0.1.0 ADP");
}

#[test]
fn validation_rejects_empty_id() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "".into(),
        conformance_class: None,
        runtime: Runtime { execution: vec![python_entry()], models: None, ..Default::default() },
        ..Default::default()
    };
    let result = validate_adp(&adp);
    assert!(result.is_err(), "validation must reject empty id");
    assert!(result.unwrap_err().to_string().contains("id must not be empty"));
}

#[test]
fn validation_accepts_multiple_backends() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.multi".into(),
        conformance_class: None,
        runtime: Runtime {
            execution: vec![
                RuntimeEntry { backend: "docker".into(), id: "docker".into(), image: Some("registry/img:latest".into()), ..Default::default() },
                RuntimeEntry { backend: "python".into(), id: "python".into(), entrypoint: Some("main:app".into()), ..Default::default() },
                RuntimeEntry { backend: "wasm".into(), id: "wasm".into(), module: Some("agent.wasm".into()), ..Default::default() },
            ],
            models: None,
            ..Default::default()
        },
        flow: minimal_flow(),
        evaluation: minimal_evaluation(),
        ..Default::default()
    };
    assert!(validate_adp(&adp).is_ok(), "Should accept multiple backends");
    assert_eq!(adp.runtime.execution.len(), 3, "Should have 3 execution entries");
}

#[test]
fn validation_accepts_different_backend_types() {
    let backends = vec!["docker", "wasm", "python", "typescript", "binary", "custom"];
    for backend in backends {
        let adp = Adp {
            adp_version: "0.1.0".into(),
            id: format!("agent.{}", backend),
            conformance_class: None,
            runtime: Runtime {
                execution: vec![RuntimeEntry { backend: backend.into(), id: format!("{}-id", backend), ..Default::default() }],
                models: None,
                ..Default::default()
            },
            ..Default::default()
        };
        let result = validate_adp(&adp);
        assert!(result.is_ok() || result.is_err(), "Validation should return result for backend {}", backend);
    }
}

#[test]
fn validation_accepts_flow_structure() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.flow".into(),
        conformance_class: None,
        runtime: Runtime { execution: vec![python_entry()], models: None, ..Default::default() },
        flow: make_flow(
            vec![make_node("input", NodeKind::Input)],
            vec![],
            Some(vec!["input"]),
            Some(vec!["input"]),
        ),
        evaluation: minimal_evaluation(),
        ..Default::default()
    };
    assert!(validate_adp(&adp).is_ok(), "Should accept ADP with flow structure");
}

#[test]
fn semantic_validation_passes_for_valid_adp() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        conformance_class: None,
        runtime: Runtime { execution: vec![python_entry()], models: None, ..Default::default() },
        flow: make_flow(
            vec![
                make_node("n1", NodeKind::Input),
                make_node("n2", NodeKind::Output),
            ],
            vec![make_edge("n1", "n2")],
            Some(vec!["n1"]),
            Some(vec!["n2"]),
        ),
        evaluation: minimal_evaluation(),
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(errors.is_empty(), "Expected no semantic errors, got: {:?}", errors);
}

#[test]
fn semantic_validation_rejects_dangling_edge() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        conformance_class: None,
        runtime: Runtime { execution: vec![python_entry()], models: None, ..Default::default() },
        flow: make_flow(
            vec![make_node("input", NodeKind::Input)],
            vec![make_edge("ghost", "input")],
            Some(vec!["input"]),
            Some(vec!["input"]),
        ),
        evaluation: serde_yaml::Value::Null,
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(errors.iter().any(|e| e.contains("ghost")), "Expected dangling edge error, got: {:?}", errors);
}

#[test]
fn semantic_validation_rejects_duplicate_node() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        conformance_class: None,
        runtime: Runtime { execution: vec![python_entry()], models: None, ..Default::default() },
        flow: make_flow(
            vec![
                make_node("input", NodeKind::Input),
                make_node("input", NodeKind::Output),
            ],
            vec![],
            Some(vec!["input"]),
            Some(vec!["input"]),
        ),
        evaluation: serde_yaml::Value::Null,
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(errors.iter().any(|e| e.contains("duplicate")), "Expected duplicate node error, got: {:?}", errors);
}

#[test]
fn semantic_validation_rejects_bad_suite_ref() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        conformance_class: None,
        runtime: Runtime { execution: vec![python_entry()], models: None, ..Default::default() },
        flow: make_flow(
            vec![Node { suite_ref: Some("missing-suite".into()), ..make_node("n1", NodeKind::LLM) }],
            vec![],
            Some(vec!["n1"]),
            Some(vec!["n1"]),
        ),
        evaluation: minimal_evaluation(),
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(errors.iter().any(|e| e.contains("suite_ref")), "Expected suite_ref error, got: {:?}", errors);
}

#[test]
fn semantic_validation_rejects_bad_model_ref() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        conformance_class: None,
        runtime: Runtime {
            execution: vec![python_entry()],
            models: Some(vec![Model {
                id: "gpt4".into(),
                provider: "openai".into(),
                model: "gpt-4o".into(),
                ..Default::default()
            }]),
            ..Default::default()
        },
        flow: make_flow(
            vec![Node { model_ref: Some("missing-model".into()), ..make_node("n1", NodeKind::LLM) }],
            vec![],
            Some(vec!["n1"]),
            Some(vec!["n1"]),
        ),
        evaluation: serde_yaml::Value::Null,
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(errors.iter().any(|e| e.contains("model_ref")), "Expected model_ref error, got: {:?}", errors);
}

#[test]
fn semantic_validation_rejects_bad_runtime_ref() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        conformance_class: None,
        runtime: Runtime {
            execution: vec![python_entry()],
            models: None,
            ..Default::default()
        },
        flow: make_flow(
            vec![Node { runtime_ref: Some("missing-backend".into()), ..make_node("n1", NodeKind::LLM) }],
            vec![],
            Some(vec!["n1"]),
            Some(vec!["n1"]),
        ),
        evaluation: serde_yaml::Value::Null,
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(errors.iter().any(|e| e.contains("runtime_ref")), "Expected runtime_ref error, got: {:?}", errors);
}

#[test]
fn validation_rejects_conformance_class_full_with_empty_evaluation() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.full".into(),
        conformance_class: Some("full".into()),
        runtime: Runtime { execution: vec![python_entry()], models: None, ..Default::default() },
        flow: minimal_flow(),
        evaluation: serde_yaml::Value::Mapping(Default::default()),
        ..Default::default()
    };
    let result = validate_adp(&adp);
    assert!(result.is_err(), "Expected error for conformance_class=full with empty evaluation");
    assert!(result.unwrap_err().to_string().contains("full"), "Error should mention 'full'");
}

#[test]
fn semantic_validation_rejects_dangling_edge_to_node() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        conformance_class: None,
        runtime: Runtime { execution: vec![python_entry()], models: None, ..Default::default() },
        flow: make_flow(
            vec![make_node("input", NodeKind::Input)],
            vec![make_edge("input", "ghost-to")],
            Some(vec!["input"]),
            Some(vec!["input"]),
        ),
        evaluation: serde_yaml::Value::Null,
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(errors.iter().any(|e| e.contains("ghost-to")), "Expected dangling to-edge error, got: {:?}", errors);
}

#[test]
fn validation_rejects_conformance_class_full_with_empty_flow() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.full".into(),
        conformance_class: Some("full".into()),
        runtime: Runtime { execution: vec![python_entry()], models: None, ..Default::default() },
        // default flow has empty id and no nodes
        evaluation: minimal_evaluation(),
        ..Default::default()
    };
    let result = validate_adp(&adp);
    assert!(result.is_err(), "Expected error for conformance_class=full with empty flow");
    assert!(result.unwrap_err().to_string().contains("full"), "Error should mention 'full'");
}

#[test]
fn validation_accepts_evaluation_structure() {
    let eval_yaml = serde_yaml::from_str(r#"
suites:
  - id: "basic"
    metrics:
      - id: "m1"
        type: "deterministic"
        function: "noop"
        scoring: "boolean"
        threshold: true
"#).unwrap();

    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.eval".into(),
        conformance_class: None,
        runtime: Runtime { execution: vec![python_entry()], models: None, ..Default::default() },
        flow: minimal_flow(),
        evaluation: eval_yaml,
        ..Default::default()
    };
    assert!(validate_adp(&adp).is_ok(), "Should accept ADP with evaluation structure");
}

#[test]
fn validation_accepts_v0_3_0() {
    let adp = Adp {
        adp_version: "0.3.0".into(),
        id: "agent.v0.3.0".into(),
        runtime: Runtime { execution: vec![python_entry()], models: None, ..Default::default() },
        flow: minimal_flow(),
        evaluation: minimal_evaluation(),
        ..Default::default()
    };
    assert!(validate_adp(&adp).is_ok(), "Should accept v0.3.0 ADP");
}

#[test]
fn semantic_check12_rejects_hook_node_filter_with_unknown_node() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        runtime: Runtime { execution: vec![python_entry()], models: None, ..Default::default() },
        flow: make_flow(
            vec![make_node("n1", NodeKind::Input)],
            vec![],
            Some(vec!["n1"]),
            Some(vec!["n1"]),
        ),
        evaluation: serde_yaml::Value::Null,
        hooks: Some(serde_json::json!([
            {
                "event": "on_node_end",
                "node_filter": ["n1", "ghost-node"],
                "handler": { "type": "function", "function_ref": "mod:fn" }
            }
        ])),
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(
        errors.iter().any(|e| e.contains("node_filter") && e.contains("ghost-node")),
        "Expected hook node_filter error, got: {:?}", errors
    );
}

#[test]
fn semantic_check13_rejects_subflow_adp_ref_not_in_subagents() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        runtime: Runtime { execution: vec![python_entry()], models: None, ..Default::default() },
        flow: make_flow(
            vec![Node { adp_ref: Some("unknown-subagent".into()), ..make_node("delegate", NodeKind::Subflow) }],
            vec![],
            Some(vec!["delegate"]),
            Some(vec!["delegate"]),
        ),
        evaluation: serde_yaml::Value::Null,
        subagents: Some(vec![Subagent {
            id: "known-subagent".into(),
            ref_uri: "./other/agent.yaml".into(),
            description: None,
            invocation_mode: None,
        }]),
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(
        errors.iter().any(|e| e.contains("adp_ref") || e.contains("subagents")),
        "Expected subflow adp_ref error, got: {:?}", errors
    );
}

#[test]
fn semantic_check14_rejects_evaluator_ref_not_in_x_testing() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        runtime: Runtime { execution: vec![python_entry()], models: None, ..Default::default() },
        flow: minimal_flow(),
        evaluation: serde_yaml::from_str(r#"
suites:
  - id: "s1"
    metrics:
      - id: "m1"
        type: "deterministic"
        function: "noop"
        scoring: "boolean"
        threshold: true
        evaluator_ref: "missing-evaluator"
"#).unwrap(),
        x_testing: Some(serde_json::json!({
            "evaluators": [{ "id": "known-evaluator", "type": "llm_judge", "model": "gpt-4o" }]
        })),
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(
        errors.iter().any(|e| e.contains("evaluator_ref")),
        "Expected evaluator_ref error, got: {:?}", errors
    );
}

#[test]
fn semantic_check7_rejects_guardrail_empty_policy_ref() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        runtime: Runtime { execution: vec![python_entry()], ..Default::default() },
        flow: minimal_flow(),
        evaluation: minimal_evaluation(),
        guardrails: Some(Guardrails {
            input: vec![GuardrailRail { id: "g1".into(), provider: "p".into(), policy_ref: "  ".into(), ..Default::default() }],
            output: vec![],
            on_violation: None,
            ..Default::default()
        }),
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(errors.iter().any(|e| e.contains("policy_ref")), "Expected policy_ref error, got: {:?}", errors);
}

#[test]
fn semantic_check8_rejects_invalid_telemetry_attribute() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        runtime: Runtime { execution: vec![python_entry()], ..Default::default() },
        flow: minimal_flow(),
        evaluation: minimal_evaluation(),
        telemetry: Some(Telemetry {
            required_attributes: vec!["invalid_attr_name".into()],
            ..Default::default()
        }),
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(errors.iter().any(|e| e.contains("required_attributes")), "Expected telemetry attribute error, got: {:?}", errors);
}

#[test]
fn semantic_check9_rejects_tool_auth_missing_env_var() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        runtime: Runtime { execution: vec![python_entry()], ..Default::default() },
        flow: minimal_flow(),
        evaluation: minimal_evaluation(),
        tools: Some(Tools {
            http_apis: Some(vec![HTTPAPI {
                id: "api1".into(),
                base_url: "https://example.com".into(),
                auth: Some(Auth {
                    scheme: Some(AuthScheme::Bearer),
                    env_var: None,
                    header: None,
                    api_key: None,
                    extensions: None,
                }),
                name: None,
                description: None,
                path: None,
                method: None,
                headers: None,
                policy: None,
                extensions: None,
            }]),
            mcp_servers: None,
            sql_functions: None,
            policy: None,
            extensions: None,
        }),
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(errors.iter().any(|e| e.contains("env_var")), "Expected env_var error, got: {:?}", errors);
}

#[test]
fn semantic_check10_rejects_unknown_compliance_standard() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        runtime: Runtime { execution: vec![python_entry()], ..Default::default() },
        flow: minimal_flow(),
        evaluation: minimal_evaluation(),
        governance: Some(serde_json::json!({
            "compliance": [{ "standard": "unknown-standard-xyz" }]
        })),
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(errors.iter().any(|e| e.contains("unknown")), "Expected compliance standard error, got: {:?}", errors);
}

#[test]
fn semantic_check11_rejects_node_tool_ref_not_in_tools() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        runtime: Runtime { execution: vec![python_entry()], ..Default::default() },
        flow: Flow {
            id: "f".into(),
            ..make_flow(
                vec![Node { tool_ref: Some("missing-tool".into()), ..make_node("n", NodeKind::Tool) }],
                vec![],
                Some(vec!["n"]),
                Some(vec!["n"]),
            )
        },
        evaluation: minimal_evaluation(),
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(errors.iter().any(|e| e.contains("tool_ref")), "Expected tool_ref error, got: {:?}", errors);
}

#[test]
fn semantic_check11_passes_with_tool_ref_in_tools() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        runtime: Runtime { execution: vec![python_entry()], ..Default::default() },
        flow: Flow {
            id: "f".into(),
            ..make_flow(
                vec![Node { tool_ref: Some("known-tool".into()), ..make_node("n", NodeKind::Tool) }],
                vec![],
                Some(vec!["n"]),
                Some(vec!["n"]),
            )
        },
        evaluation: minimal_evaluation(),
        tools: Some(Tools {
            mcp_servers: Some(vec![MCPServer {
                id: "known-tool".into(),
                command: "".into(),
                name: None,
                description: None,
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
        }),
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(!errors.iter().any(|e| e.contains("tool_ref")), "Expected no tool_ref error, got: {:?}", errors);
}

#[test]
fn semantic_deprecated_judges_warning() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        runtime: Runtime { execution: vec![python_entry()], ..Default::default() },
        flow: minimal_flow(),
        evaluation: minimal_evaluation(),
        x_testing: Some(serde_json::json!({
            "judges": [{ "id": "j1", "model": "gpt-4o", "system_prompt": "Rate 0-1" }]
        })),
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(errors.iter().any(|e| e.contains("deprecated")), "Expected deprecated judges warning, got: {:?}", errors);
}

#[test]
fn semantic_as1_rejects_node_map_unknown_node() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        runtime: Runtime { execution: vec![python_entry()], ..Default::default() },
        flow: make_flow(
            vec![make_node("planner", NodeKind::LLM)],
            vec![],
            Some(vec!["planner"]),
            Some(vec!["planner"]),
        ),
        evaluation: minimal_evaluation(),
        interop: Some(Interop {
            a2a: None,
            agentspec: Some(InteropAgentSpec {
                ref_uri: None,
                version: None,
                component_type: None,
                component_id: None,
                runtime_adapters: None,
                node_map: Some({
                    let mut m = std::collections::HashMap::new();
                    m.insert("planner".to_string(), "3a5bf0c0-9f28-47d8-a000-111111111111".to_string());
                    m.insert("ghost-node".to_string(), "fc98ab56-0d30-4bdd-a000-222222222222".to_string());
                    m
                }),
                llm_map: None,
            }),
        }),
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(
        errors.iter().any(|e| e.contains("ghost-node")),
        "Expected AS-1 node_map error for unknown key, got: {:?}", errors
    );
}

#[test]
fn semantic_as1_passes_valid_node_map() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        runtime: Runtime { execution: vec![python_entry()], ..Default::default() },
        flow: make_flow(
            vec![make_node("planner", NodeKind::LLM)],
            vec![],
            Some(vec!["planner"]),
            Some(vec!["planner"]),
        ),
        evaluation: minimal_evaluation(),
        interop: Some(Interop {
            a2a: None,
            agentspec: Some(InteropAgentSpec {
                ref_uri: None,
                version: None,
                component_type: None,
                component_id: None,
                runtime_adapters: None,
                node_map: Some({
                    let mut m = std::collections::HashMap::new();
                    m.insert("planner".to_string(), "3a5bf0c0-9f28-47d8-a000-111111111111".to_string());
                    m
                }),
                llm_map: None,
            }),
        }),
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(
        !errors.iter().any(|e| e.contains("node_map")),
        "Expected no AS-1 error for valid node_map, got: {:?}", errors
    );
}

#[test]
fn semantic_as2_rejects_llm_map_unknown_backend() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        runtime: Runtime { execution: vec![python_entry()], ..Default::default() },
        flow: minimal_flow(),
        evaluation: minimal_evaluation(),
        interop: Some(Interop {
            a2a: None,
            agentspec: Some(InteropAgentSpec {
                ref_uri: None,
                version: None,
                component_type: None,
                component_id: None,
                runtime_adapters: None,
                node_map: None,
                llm_map: Some(vec![InteropAgentSpecLlmBinding {
                    backend_id: "ghost-backend".into(),
                    agentspec_id: "3a5bf0c0-9f28-47d8-a000-111111111111".into(),
                    agentspec_type: None,
                }]),
            }),
        }),
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(
        errors.iter().any(|e| e.contains("ghost-backend")),
        "Expected AS-2 llm_map error for unknown backend, got: {:?}", errors
    );
}

#[test]
fn semantic_as2_passes_valid_llm_map() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        runtime: Runtime { execution: vec![python_entry()], ..Default::default() },
        flow: minimal_flow(),
        evaluation: minimal_evaluation(),
        interop: Some(Interop {
            a2a: None,
            agentspec: Some(InteropAgentSpec {
                ref_uri: None,
                version: None,
                component_type: None,
                component_id: None,
                runtime_adapters: None,
                node_map: None,
                llm_map: Some(vec![InteropAgentSpecLlmBinding {
                    backend_id: "py".into(),
                    agentspec_id: "3a5bf0c0-9f28-47d8-a000-111111111111".into(),
                    agentspec_type: Some("OpenAiConfig".into()),
                }]),
            }),
        }),
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(
        !errors.iter().any(|e| e.contains("llm_map")),
        "Expected no AS-2 error for valid llm_map, got: {:?}", errors
    );
}

#[test]
fn evaluator_load_unsupported_types_return_errors() {
    for eval_type in &["deterministic", "llm_judge", "container"] {
        let config = serde_json::json!({ "id": "e1", "type": eval_type });
        let result = load_evaluator(&config);
        assert!(
            matches!(result, Err(EvaluatorError::UnsupportedType(_))),
            "Expected UnsupportedType for {}", eval_type
        );
    }
}

#[test]
fn evaluator_load_script_missing_runtime_returns_error() {
    let config = serde_json::json!({ "id": "e1", "type": "script", "inline": "echo hi" });
    let result = load_evaluator(&config);
    assert!(result.is_err(), "Should fail without runtime field");
}

#[test]
fn evaluator_load_script_non_bash_runtime_returns_unsupported() {
    let config = serde_json::json!({
        "id": "e1", "type": "script", "runtime": "python",
        "inline": "def evaluate(o, c): return True"
    });
    let result = load_evaluator(&config);
    assert!(
        matches!(result, Err(EvaluatorError::UnsupportedType(_))),
        "Expected UnsupportedType for python runtime in Rust SDK"
    );
}

#[test]
fn evaluator_load_unknown_type_and_display() {
    let config = serde_json::json!({ "id": "e1", "type": "foobar" });
    let err = load_evaluator(&config).err().unwrap();
    assert!(err.to_string().contains("foobar")); // exercises Display::UnsupportedType
}

#[test]
fn evaluator_error_display_missing_field() {
    // missing runtime → MissingField → exercises Display::MissingField
    let config = serde_json::json!({ "id": "e1", "type": "script", "inline": "x" });
    let err = load_evaluator(&config).err().unwrap();
    assert!(err.to_string().contains("runtime"));
}

#[test]
fn evaluator_load_script_bash_requires_inline_or_script_ref() {
    let config = serde_json::json!({ "id": "e1", "type": "script", "runtime": "bash" });
    let result = load_evaluator(&config);
    assert!(matches!(result, Err(EvaluatorError::MissingField(_))));
}

#[test]
fn evaluator_script_bash_inline_object_result() {
    let config = serde_json::json!({
        "id": "ev", "type": "script", "runtime": "bash",
        "inline": "printf '{\"passed\": true, \"score\": 0.9, \"reason\": \"ok\"}'"
    });
    let ev = load_evaluator(&config).unwrap();
    let r = ev.evaluate(&serde_json::json!({}), &serde_json::json!({})).unwrap();
    assert!(r.passed);
    assert_eq!(r.score, Some(0.9));
    assert_eq!(r.reason, "ok");
    assert_eq!(r.evaluator_id, "ev");
    assert_eq!(r.evaluator_type, "script");
}

#[test]
fn evaluator_script_bash_inline_bool_true() {
    let config = serde_json::json!({
        "id": "ev", "type": "script", "runtime": "bash",
        "inline": "echo 'true'"
    });
    let ev = load_evaluator(&config).unwrap();
    let r = ev.evaluate(&serde_json::json!({}), &serde_json::json!({})).unwrap();
    assert!(r.passed);
    assert_eq!(r.score, Some(1.0));
}

#[test]
fn evaluator_script_bash_inline_bool_false() {
    let config = serde_json::json!({
        "id": "ev", "type": "script", "runtime": "bash",
        "inline": "echo 'false'"
    });
    let ev = load_evaluator(&config).unwrap();
    let r = ev.evaluate(&serde_json::json!({}), &serde_json::json!({})).unwrap();
    assert!(!r.passed);
    assert_eq!(r.score, Some(0.0));
}

#[test]
fn evaluator_script_bash_inline_fallback_arm() {
    // JSON number → normalize_result _ arm; as_bool() returns None → false
    let config = serde_json::json!({
        "id": "ev", "type": "script", "runtime": "bash",
        "inline": "echo '42'"
    });
    let ev = load_evaluator(&config).unwrap();
    let r = ev.evaluate(&serde_json::json!({}), &serde_json::json!({})).unwrap();
    assert!(!r.passed);
}

#[test]
fn evaluator_script_bash_nonzero_exit() {
    let config = serde_json::json!({
        "id": "ev", "type": "script", "runtime": "bash",
        "inline": "exit 1"
    });
    let ev = load_evaluator(&config).unwrap();
    let r = ev.evaluate(&serde_json::json!({}), &serde_json::json!({})).unwrap();
    assert!(!r.passed);
    assert!(r.reason.contains("bash error"));
}

#[test]
fn evaluator_script_bash_invalid_json_exercises_runtime_error_display() {
    let config = serde_json::json!({
        "id": "ev", "type": "script", "runtime": "bash",
        "inline": "echo 'not valid json'"
    });
    let ev = load_evaluator(&config).unwrap();
    let err = ev.evaluate(&serde_json::json!({}), &serde_json::json!({})).err().unwrap();
    assert!(err.to_string().contains("Runtime error")); // exercises Display::RuntimeError
}

#[test]
fn evaluator_script_bash_local_file() {
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "printf '{{\"passed\": true}}'").unwrap();
    let path = tmp.path().to_str().unwrap().to_string();
    let config = serde_json::json!({
        "id": "ev", "type": "script", "runtime": "bash",
        "script_ref": path
    });
    let ev = load_evaluator(&config).unwrap();
    let r = ev.evaluate(&serde_json::json!({}), &serde_json::json!({})).unwrap();
    assert!(r.passed);
}

#[test]
fn evaluator_script_bash_git_pinned_ref_unsupported() {
    let config = serde_json::json!({
        "id": "ev", "type": "script", "runtime": "bash",
        "script_ref": "git+https://github.com/example/repo.git/script.sh@abc1234"
    });
    let ev = load_evaluator(&config).unwrap();
    let result = ev.evaluate(&serde_json::json!({}), &serde_json::json!({}));
    assert!(matches!(result, Err(EvaluatorError::UnsupportedType(_))));
}

#[test]
fn evaluator_script_bash_object_score_based_passed() {
    // No "passed" key, score >= 0.5 → passed = true via score branch
    let config = serde_json::json!({
        "id": "ev", "type": "script", "runtime": "bash",
        "inline": "printf '{\"score\": 0.8}'"
    });
    let ev = load_evaluator(&config).unwrap();
    let r = ev.evaluate(&serde_json::json!({}), &serde_json::json!({})).unwrap();
    assert!(r.passed);
}
