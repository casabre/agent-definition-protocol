use adp_sdk::adp::Adp;
use adp_sdk::validation::validate_adp;

#[test]
fn validation_rejects_missing_execution() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        runtime: adp_sdk::adp::Runtime { execution: vec![] },
        flow: serde_yaml::Value::Null,
        evaluation: serde_yaml::Value::Null,
    };
    assert!(validate_adp(&adp).is_err());
}

#[test]
fn validation_accepts_basic() {
    let adp = Adp {
        adp_version: "0.1.0".into(),
        id: "agent.test".into(),
        runtime: adp_sdk::adp::Runtime {
            execution: vec![adp_sdk::adp::RuntimeEntry {
                backend: "python".into(),
                id: "py".into(),
                entrypoint: Some("agent.main:app".into()),
            }],
        },
        flow: serde_yaml::Value::Null,
        evaluation: serde_yaml::Value::Null,
    };
    assert!(validate_adp(&adp).is_ok());
}
