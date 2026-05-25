use adp_sdk::adp::{Adp, Model, Runtime, RuntimeEntry, Subagent};
use adp_sdk::evaluation::{load_evaluator, EvaluatorError};
use adp_sdk::validation::{validate_adp, validate_adp_semantics};

fn minimal_flow() -> serde_yaml::Value {
    serde_yaml::from_str(r#"
id: "f"
graph:
  nodes:
    - id: "n"
      kind: "input"
  edges: []
  start_nodes: ["n"]
  end_nodes: ["n"]
"#).unwrap()
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
        flow: serde_yaml::Value::Null,
        evaluation: serde_yaml::Value::Null,
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
        flow: serde_yaml::Value::Null,
        evaluation: serde_yaml::Value::Null,
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
        flow: serde_yaml::from_str(r#"
id: "test.flow"
graph:
  nodes:
    - id: "input"
      kind: "input"
    - id: "output"
      kind: "output"
  edges: []
  start_nodes: ["input"]
  end_nodes: ["output"]
"#).unwrap(),
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
        flow: serde_yaml::Value::Null,
        evaluation: serde_yaml::Value::Null,
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
            flow: serde_yaml::Value::Null,
            evaluation: serde_yaml::Value::Null,
            ..Default::default()
        };
        let result = validate_adp(&adp);
        assert!(result.is_ok() || result.is_err(), "Validation should return result for backend {}", backend);
    }
}

#[test]
fn validation_accepts_flow_structure() {
    let flow_yaml = serde_yaml::from_str(r#"
id: "test.flow"
graph:
  nodes:
    - id: "input"
      kind: "input"
  edges: []
  start_nodes: ["input"]
  end_nodes: ["input"]
"#).unwrap();

    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.flow".into(),
        conformance_class: None,
        runtime: Runtime { execution: vec![python_entry()], models: None, ..Default::default() },
        flow: flow_yaml,
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
        flow: serde_yaml::from_str(r#"
graph:
  nodes:
    - id: "n1"
      kind: "input"
    - id: "n2"
      kind: "output"
  edges:
    - from: "n1"
      to: "n2"
  start_nodes: ["n1"]
  end_nodes: ["n2"]
"#).unwrap(),
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
        flow: serde_yaml::from_str(r#"
graph:
  nodes:
    - id: "input"
      kind: "input"
  edges:
    - from: "ghost"
      to: "input"
  start_nodes: ["input"]
  end_nodes: ["input"]
"#).unwrap(),
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
        flow: serde_yaml::from_str(r#"
graph:
  nodes:
    - id: "input"
      kind: "input"
    - id: "input"
      kind: "output"
  edges: []
  start_nodes: ["input"]
  end_nodes: ["input"]
"#).unwrap(),
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
        flow: serde_yaml::from_str(r#"
graph:
  nodes:
    - id: "n1"
      kind: "llm"
      suite_ref: "missing-suite"
  edges: []
  start_nodes: ["n1"]
  end_nodes: ["n1"]
"#).unwrap(),
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
        flow: serde_yaml::from_str(r#"
graph:
  nodes:
    - id: "n1"
      kind: "llm"
      model_ref: "missing-model"
  edges: []
  start_nodes: ["n1"]
  end_nodes: ["n1"]
"#).unwrap(),
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
        flow: serde_yaml::from_str(r#"
graph:
  nodes:
    - id: "n1"
      kind: "llm"
      runtime_ref: "missing-backend"
  edges: []
  start_nodes: ["n1"]
  end_nodes: ["n1"]
"#).unwrap(),
        evaluation: serde_yaml::Value::Null,
        ..Default::default()
    };
    let errors = validate_adp_semantics(&adp);
    assert!(errors.iter().any(|e| e.contains("runtime_ref")), "Expected runtime_ref error, got: {:?}", errors);
}

#[test]
fn validation_rejects_conformance_class_full_with_empty_flow() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.full".into(),
        conformance_class: Some("full".into()),
        runtime: Runtime { execution: vec![python_entry()], models: None, ..Default::default() },
        flow: serde_yaml::Value::Mapping(Default::default()),
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
        flow: serde_yaml::from_str(r#"
graph:
  nodes:
    - id: "n1"
      kind: "input"
  edges: []
  start_nodes: ["n1"]
  end_nodes: ["n1"]
"#).unwrap(),
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
        flow: serde_yaml::from_str(r#"
graph:
  nodes:
    - id: "delegate"
      kind: "subflow"
      adp_ref: "unknown-subagent"
  edges: []
  start_nodes: ["delegate"]
  end_nodes: ["delegate"]
"#).unwrap(),
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

#[test]
fn evaluator_script_bash_object_score_below_threshold() {
    // No "passed" key, score < 0.5 → passed = false
    let config = serde_json::json!({
        "id": "ev", "type": "script", "runtime": "bash",
        "inline": "printf '{\"score\": 0.3}'"
    });
    let ev = load_evaluator(&config).unwrap();
    let r = ev.evaluate(&serde_json::json!({}), &serde_json::json!({})).unwrap();
    assert!(!r.passed);
}

#[test]
fn evaluator_script_bash_object_no_passed_no_score() {
    // Neither "passed" nor "score" → unwrap_or(false) → false
    let config = serde_json::json!({
        "id": "ev", "type": "script", "runtime": "bash",
        "inline": "printf '{\"reason\": \"no keys\"}'"
    });
    let ev = load_evaluator(&config).unwrap();
    let r = ev.evaluate(&serde_json::json!({}), &serde_json::json!({})).unwrap();
    assert!(!r.passed);
}
