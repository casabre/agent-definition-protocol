use std::collections::HashMap;
use std::fmt;
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct EvaluationResult {
    pub passed: bool,
    pub score: Option<f64>,
    pub reason: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub evaluator_id: String,
    pub evaluator_type: String,
}

#[derive(Debug)]
pub enum EvaluatorError {
    UnsupportedType(String),
    MissingField(String),
    RuntimeError(String),
}

impl fmt::Display for EvaluatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvaluatorError::UnsupportedType(t) => write!(f, "Unsupported evaluator type: '{}'", t),
            EvaluatorError::MissingField(field) => write!(f, "Missing required field: '{}'", field),
            EvaluatorError::RuntimeError(msg) => write!(f, "Runtime error: {}", msg),
        }
    }
}

impl std::error::Error for EvaluatorError {}

pub trait Evaluator {
    fn evaluate(
        &self,
        output: &serde_json::Value,
        context: &serde_json::Value,
    ) -> Result<EvaluationResult, EvaluatorError>;
}

pub fn load_evaluator(
    config: &serde_json::Value,
) -> Result<Box<dyn Evaluator>, EvaluatorError> {
    let eval_type = config
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("(missing)");

    match eval_type {
        "script" => Ok(Box::new(ScriptEvaluator::new(config)?)),
        "deterministic" => Err(EvaluatorError::UnsupportedType(
            "deterministic: function_ref loading requires the Python or TypeScript SDK".to_string(),
        )),
        "llm_judge" => Err(EvaluatorError::UnsupportedType(
            "llm_judge: requires an LLM client; use the Python or TypeScript SDK".to_string(),
        )),
        "container" => Err(EvaluatorError::UnsupportedType(
            "container: deferred in the Rust SDK; use the Python or TypeScript SDK".to_string(),
        )),
        _ => Err(EvaluatorError::UnsupportedType(eval_type.to_string())),
    }
}

struct ScriptEvaluator {
    id: String,
    runtime: String,
    inline: Option<String>,
    script_ref: Option<String>,
}

impl ScriptEvaluator {
    fn new(config: &serde_json::Value) -> Result<Self, EvaluatorError> {
        let id = config
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let runtime = config
            .get("runtime")
            .and_then(|v| v.as_str())
            .ok_or_else(|| EvaluatorError::MissingField("runtime".to_string()))?
            .to_string();
        if runtime != "bash" {
            return Err(EvaluatorError::UnsupportedType(format!(
                "script runtime '{}': only 'bash' is supported in the Rust SDK",
                runtime
            )));
        }
        let inline = config
            .get("inline")
            .and_then(|v| v.as_str())
            .map(String::from);
        let script_ref = config
            .get("script_ref")
            .and_then(|v| v.as_str())
            .map(String::from);
        if inline.is_none() && script_ref.is_none() {
            return Err(EvaluatorError::MissingField(
                "inline or script_ref".to_string(),
            ));
        }
        Ok(ScriptEvaluator { id, runtime, inline, script_ref })
    }

    fn resolve_script(&self) -> Result<String, EvaluatorError> {
        match (&self.inline, &self.script_ref) {
            (Some(inline), _) => Ok(inline.clone()),
            (_, Some(r)) if r.starts_with("git+") => Err(EvaluatorError::UnsupportedType(
                "git-pinned script_ref is not supported in the Rust SDK".to_string(),
            )),
            (_, Some(r)) => std::fs::read_to_string(r)
                .map_err(|e| EvaluatorError::RuntimeError(e.to_string())),
            (None, None) => Err(EvaluatorError::MissingField("inline or script_ref".to_string())),
        }
    }
}

impl Evaluator for ScriptEvaluator {
    fn evaluate(
        &self,
        output: &serde_json::Value,
        context: &serde_json::Value,
    ) -> Result<EvaluationResult, EvaluatorError> {
        let script = self.resolve_script()?;
        let input = serde_json::json!({"output": output, "context": context});
        let input_str = input.to_string();

        let mut child = Command::new("/bin/bash")
            .arg("-c")
            .arg(&script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| EvaluatorError::RuntimeError(e.to_string()))?;

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input_str.as_bytes()); // ignore EPIPE if script exits early
        }

        let out = child
            .wait_with_output()
            .map_err(|e| EvaluatorError::RuntimeError(e.to_string()))?;

        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            return Ok(EvaluationResult {
                passed: false,
                score: None,
                reason: format!("bash error: {}", stderr),
                metadata: HashMap::new(),
                evaluator_id: self.id.clone(),
                evaluator_type: "script".to_string(),
            });
        }

        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let raw: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|e| EvaluatorError::RuntimeError(format!("failed to parse output: {}", e)))?;

        Ok(normalize_result(&raw, &self.id, "script"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_script_none_none_returns_missing_field() {
        let ev = ScriptEvaluator {
            id: "t".into(),
            runtime: "bash".into(),
            inline: None,
            script_ref: None,
        };
        assert!(matches!(ev.resolve_script(), Err(EvaluatorError::MissingField(_))));
    }

    // ===== EvaluatorError Display tests =====

    #[test]
    fn evaluator_error_unsupported_type_display() {
        let e = EvaluatorError::UnsupportedType("foo".to_string());
        assert_eq!(format!("{}", e), "Unsupported evaluator type: 'foo'");
    }

    #[test]
    fn evaluator_error_missing_field_display() {
        let e = EvaluatorError::MissingField("runtime".to_string());
        assert_eq!(format!("{}", e), "Missing required field: 'runtime'");
    }

    #[test]
    fn evaluator_error_runtime_error_display() {
        let e = EvaluatorError::RuntimeError("crash".to_string());
        assert_eq!(format!("{}", e), "Runtime error: crash");
    }

    #[test]
    fn evaluator_error_is_std_error() {
        let e = EvaluatorError::RuntimeError("test".to_string());
        let _: &dyn std::error::Error = &e;
    }

    // Helper to extract the error from a load_evaluator result
    fn unwrap_load_err(result: Result<Box<dyn Evaluator>, EvaluatorError>) -> EvaluatorError {
        match result {
            Err(e) => e,
            Ok(_) => panic!("Expected Err but got Ok"),
        }
    }

    // ===== load_evaluator tests =====

    #[test]
    fn load_evaluator_missing_type_returns_unsupported() {
        let config = serde_json::json!({});
        let result = load_evaluator(&config);
        assert!(result.is_err());
        let err = unwrap_load_err(result);
        assert!(matches!(err, EvaluatorError::UnsupportedType(_)));
        assert!(format!("{}", err).contains("(missing)"));
    }

    #[test]
    fn load_evaluator_deterministic_returns_unsupported() {
        let config = serde_json::json!({"type": "deterministic"});
        let result = load_evaluator(&config);
        assert!(result.is_err());
        assert!(matches!(unwrap_load_err(result), EvaluatorError::UnsupportedType(_)));
    }

    #[test]
    fn load_evaluator_llm_judge_returns_unsupported() {
        let config = serde_json::json!({"type": "llm_judge"});
        let result = load_evaluator(&config);
        assert!(result.is_err());
        assert!(matches!(unwrap_load_err(result), EvaluatorError::UnsupportedType(_)));
    }

    #[test]
    fn load_evaluator_container_returns_unsupported() {
        let config = serde_json::json!({"type": "container"});
        let result = load_evaluator(&config);
        assert!(result.is_err());
        assert!(matches!(unwrap_load_err(result), EvaluatorError::UnsupportedType(_)));
    }

    #[test]
    fn load_evaluator_unknown_type_returns_unsupported() {
        let config = serde_json::json!({"type": "mystery_type"});
        let result = load_evaluator(&config);
        assert!(result.is_err());
        assert!(matches!(unwrap_load_err(result), EvaluatorError::UnsupportedType(_)));
    }

    // ===== ScriptEvaluator::new tests =====

    #[test]
    fn script_evaluator_new_missing_runtime() {
        let config = serde_json::json!({"type": "script", "inline": "echo hi"});
        let result = load_evaluator(&config);
        assert!(result.is_err());
        assert!(matches!(unwrap_load_err(result), EvaluatorError::MissingField(_)));
    }

    #[test]
    fn script_evaluator_new_unsupported_runtime() {
        let config = serde_json::json!({"type": "script", "runtime": "python", "inline": "print()"});
        let result = load_evaluator(&config);
        assert!(result.is_err());
        assert!(matches!(unwrap_load_err(result), EvaluatorError::UnsupportedType(_)));
    }

    #[test]
    fn script_evaluator_new_missing_inline_and_script_ref() {
        let config = serde_json::json!({"type": "script", "runtime": "bash"});
        let result = load_evaluator(&config);
        assert!(result.is_err());
        assert!(matches!(unwrap_load_err(result), EvaluatorError::MissingField(_)));
    }

    #[test]
    fn script_evaluator_new_with_inline() {
        let config = serde_json::json!({"type": "script", "runtime": "bash", "inline": "echo true"});
        let result = load_evaluator(&config);
        assert!(result.is_ok(), "Expected Ok from load_evaluator with inline script");
    }

    #[test]
    fn script_evaluator_new_with_script_ref() {
        let config = serde_json::json!({"type": "script", "runtime": "bash", "script_ref": "/tmp/myscript.sh"});
        let result = load_evaluator(&config);
        assert!(result.is_ok(), "Expected Ok from load_evaluator with script_ref");
    }

    // ===== resolve_script tests =====

    #[test]
    fn resolve_script_inline_wins() {
        let ev = ScriptEvaluator {
            id: "t".into(),
            runtime: "bash".into(),
            inline: Some("echo inline".into()),
            script_ref: Some("/tmp/other.sh".into()),
        };
        let script = ev.resolve_script().unwrap();
        assert_eq!(script, "echo inline");
    }

    #[test]
    fn resolve_script_git_pinned_ref_returns_error() {
        let ev = ScriptEvaluator {
            id: "t".into(),
            runtime: "bash".into(),
            inline: None,
            script_ref: Some("git+https://github.com/org/repo.git".into()),
        };
        let result = ev.resolve_script();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), EvaluatorError::UnsupportedType(_)));
    }

    #[test]
    fn resolve_script_file_not_found_returns_runtime_error() {
        let ev = ScriptEvaluator {
            id: "t".into(),
            runtime: "bash".into(),
            inline: None,
            script_ref: Some("/this/path/does/not/exist.sh".into()),
        };
        let result = ev.resolve_script();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), EvaluatorError::RuntimeError(_)));
    }

    // ===== Evaluator::evaluate tests (ScriptEvaluator) =====

    #[test]
    fn evaluate_inline_bool_true_output() {
        let config = serde_json::json!({"type": "script", "runtime": "bash", "inline": "echo true"});
        let evaluator = load_evaluator(&config).unwrap();
        let output = serde_json::json!("test output");
        let context = serde_json::json!({});
        let result = evaluator.evaluate(&output, &context);
        assert!(result.is_ok());
        let eval_result = result.unwrap();
        assert!(eval_result.passed);
        assert_eq!(eval_result.evaluator_type, "script");
    }

    #[test]
    fn evaluate_inline_json_object_output() {
        let config = serde_json::json!({
            "type": "script",
            "runtime": "bash",
            "id": "my-evaluator",
            "inline": r#"echo '{"passed": true, "score": 0.9, "reason": "good"}'"#
        });
        let evaluator = load_evaluator(&config).unwrap();
        let result = evaluator.evaluate(&serde_json::json!("output"), &serde_json::json!({})).unwrap();
        assert!(result.passed);
        assert_eq!(result.score, Some(0.9));
        assert_eq!(result.reason, "good");
        assert_eq!(result.evaluator_id, "my-evaluator");
    }

    #[test]
    fn evaluate_inline_json_object_score_based_passed() {
        // When no "passed" key but score >= 0.5
        let config = serde_json::json!({
            "type": "script",
            "runtime": "bash",
            "inline": r#"echo '{"score": 0.7}'"#
        });
        let evaluator = load_evaluator(&config).unwrap();
        let result = evaluator.evaluate(&serde_json::json!("out"), &serde_json::json!({})).unwrap();
        assert!(result.passed);
    }

    #[test]
    fn evaluate_inline_json_object_score_based_failed() {
        // score < 0.5 → failed
        let config = serde_json::json!({
            "type": "script",
            "runtime": "bash",
            "inline": r#"echo '{"score": 0.3}'"#
        });
        let evaluator = load_evaluator(&config).unwrap();
        let result = evaluator.evaluate(&serde_json::json!("out"), &serde_json::json!({})).unwrap();
        assert!(!result.passed);
    }

    #[test]
    fn evaluate_inline_bash_failure_returns_ok_with_passed_false() {
        let config = serde_json::json!({
            "type": "script",
            "runtime": "bash",
            "inline": "exit 1"
        });
        let evaluator = load_evaluator(&config).unwrap();
        let result = evaluator.evaluate(&serde_json::json!("out"), &serde_json::json!({})).unwrap();
        assert!(!result.passed);
        assert!(result.reason.contains("bash error"));
    }

    #[test]
    fn evaluate_inline_invalid_json_output_returns_runtime_error() {
        let config = serde_json::json!({
            "type": "script",
            "runtime": "bash",
            "inline": "echo 'not valid json at all {{{'}"
        });
        let evaluator = load_evaluator(&config).unwrap();
        let result = evaluator.evaluate(&serde_json::json!("out"), &serde_json::json!({}));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), EvaluatorError::RuntimeError(_)));
    }

    #[test]
    fn evaluate_inline_json_null_output() {
        // null output falls through to the _ arm of normalize_result
        let config = serde_json::json!({
            "type": "script",
            "runtime": "bash",
            "inline": "echo null"
        });
        let evaluator = load_evaluator(&config).unwrap();
        let result = evaluator.evaluate(&serde_json::json!("out"), &serde_json::json!({})).unwrap();
        assert!(!result.passed);
        assert_eq!(result.score, None);
    }

    #[test]
    fn evaluate_inline_json_false_output() {
        let config = serde_json::json!({
            "type": "script",
            "runtime": "bash",
            "inline": "echo false"
        });
        let evaluator = load_evaluator(&config).unwrap();
        let result = evaluator.evaluate(&serde_json::json!("out"), &serde_json::json!({})).unwrap();
        assert!(!result.passed);
        assert_eq!(result.score, Some(0.0));
    }

    // ===== normalize_result tests (via evaluate) =====

    #[test]
    fn normalize_result_bool_true() {
        let r = normalize_result(&serde_json::json!(true), "ev1", "script");
        assert!(r.passed);
        assert_eq!(r.score, Some(1.0));
        assert_eq!(r.evaluator_id, "ev1");
        assert_eq!(r.evaluator_type, "script");
    }

    #[test]
    fn normalize_result_bool_false() {
        let r = normalize_result(&serde_json::json!(false), "ev2", "test");
        assert!(!r.passed);
        assert_eq!(r.score, Some(0.0));
    }

    #[test]
    fn normalize_result_object_with_passed_key() {
        let r = normalize_result(
            &serde_json::json!({"passed": false, "score": 0.8, "reason": "partial"}),
            "ev3", "script"
        );
        assert!(!r.passed);
        assert_eq!(r.score, Some(0.8));
        assert_eq!(r.reason, "partial");
    }

    #[test]
    fn normalize_result_object_no_passed_no_score() {
        // Neither passed nor score → defaults to false
        let r = normalize_result(
            &serde_json::json!({"reason": "something"}),
            "ev4", "script"
        );
        assert!(!r.passed);
        assert_eq!(r.score, None);
    }

    #[test]
    fn normalize_result_number_falls_through_to_default() {
        // Number is neither Bool nor Object → _ arm: passed = as_bool().unwrap_or(false) = false
        let r = normalize_result(&serde_json::json!(42), "ev5", "script");
        assert!(!r.passed);
        assert_eq!(r.score, None);
    }
}

fn normalize_result(raw: &serde_json::Value, id: &str, eval_type: &str) -> EvaluationResult {
    match raw {
        serde_json::Value::Bool(b) => EvaluationResult {
            passed: *b,
            score: Some(if *b { 1.0 } else { 0.0 }),
            reason: String::new(),
            metadata: HashMap::new(),
            evaluator_id: id.to_string(),
            evaluator_type: eval_type.to_string(),
        },
        serde_json::Value::Object(obj) => {
            let passed = obj
                .get("passed")
                .and_then(|v| v.as_bool())
                .unwrap_or_else(|| {
                    obj.get("score")
                        .and_then(|v| v.as_f64())
                        .map(|s| s >= 0.5)
                        .unwrap_or(false)
                });
            let score = obj.get("score").and_then(|v| v.as_f64());
            let reason = obj
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            EvaluationResult {
                passed,
                score,
                reason,
                metadata: HashMap::new(),
                evaluator_id: id.to_string(),
                evaluator_type: eval_type.to_string(),
            }
        }
        _ => EvaluationResult {
            passed: raw.as_bool().unwrap_or(false),
            score: None,
            reason: String::new(),
            metadata: HashMap::new(),
            evaluator_id: id.to_string(),
            evaluator_type: eval_type.to_string(),
        },
    }
}
