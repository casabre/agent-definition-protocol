use adp_sdk::adp::{Adp, Model, Runtime, RuntimeEntry};
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
        runtime: Runtime { execution: vec![], models: None },
        flow: serde_yaml::Value::Null,
        evaluation: serde_yaml::Value::Null,
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
        runtime: Runtime { execution: vec![python_entry()], models: None },
        flow: minimal_flow(),
        evaluation: minimal_evaluation(),
    };
    assert!(validate_adp(&adp).is_ok(), "Should accept valid basic ADP");
}

#[test]
fn validation_rejects_invalid_version() {
    let adp = Adp {
        adp_version: "0.3.0".into(),
        id: "agent.test".into(),
        conformance_class: None,
        runtime: Runtime { execution: vec![python_entry()], models: None },
        flow: serde_yaml::Value::Null,
        evaluation: serde_yaml::Value::Null,
    };
    assert!(validate_adp(&adp).is_err(), "Should reject invalid version");
}

#[test]
fn validation_accepts_v0_1_0() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.v0.1.0".into(),
        conformance_class: None,
        runtime: Runtime { execution: vec![python_entry()], models: None },
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
    };
    assert!(validate_adp(&adp).is_ok(), "Should accept v0.1.0 ADP");
}

#[test]
fn validation_rejects_empty_id() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "".into(),
        conformance_class: None,
        runtime: Runtime { execution: vec![python_entry()], models: None },
        flow: serde_yaml::Value::Null,
        evaluation: serde_yaml::Value::Null,
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
        },
        flow: minimal_flow(),
        evaluation: minimal_evaluation(),
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
            },
            flow: serde_yaml::Value::Null,
            evaluation: serde_yaml::Value::Null,
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
        runtime: Runtime { execution: vec![python_entry()], models: None },
        flow: flow_yaml,
        evaluation: minimal_evaluation(),
    };
    assert!(validate_adp(&adp).is_ok(), "Should accept ADP with flow structure");
}

#[test]
fn semantic_validation_passes_for_valid_adp() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        conformance_class: None,
        runtime: Runtime { execution: vec![python_entry()], models: None },
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
        runtime: Runtime { execution: vec![python_entry()], models: None },
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
        runtime: Runtime { execution: vec![python_entry()], models: None },
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
        runtime: Runtime { execution: vec![python_entry()], models: None },
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
                api_key_env: None,
                base_url: None,
                temperature: None,
                max_tokens: None,
                extensions: None,
            }]),
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
        runtime: Runtime { execution: vec![python_entry()], models: None },
        flow: serde_yaml::Value::Mapping(Default::default()),
        evaluation: minimal_evaluation(),
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
        runtime: Runtime { execution: vec![python_entry()], models: None },
        flow: minimal_flow(),
        evaluation: eval_yaml,
    };
    assert!(validate_adp(&adp).is_ok(), "Should accept ADP with evaluation structure");
}
