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
        if let Some(ref inline) = self.inline {
            return Ok(inline.clone());
        }
        if let Some(ref script_ref) = self.script_ref {
            if script_ref.starts_with("git+") {
                return Err(EvaluatorError::UnsupportedType(
                    "git-pinned script_ref is not supported in the Rust SDK".to_string(),
                ));
            }
            return std::fs::read_to_string(script_ref)
                .map_err(|e| EvaluatorError::RuntimeError(e.to_string()));
        }
        unreachable!()
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
            if let Err(e) = stdin.write_all(input_str.as_bytes()) {
                // BrokenPipe means the script exited without reading stdin — that's fine.
                if e.kind() != std::io::ErrorKind::BrokenPipe {
                    return Err(EvaluatorError::RuntimeError(e.to_string()));
                }
            }
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
