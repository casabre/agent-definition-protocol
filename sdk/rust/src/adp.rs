use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct RuntimeEntry {
    pub backend: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entrypoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub backend_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_manager: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ModelStructuredOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_ref: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct Model {
    pub id: String,
    pub provider: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_yaml::Value>,
    // v0.3.0 model parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_streaming_api: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_output: Option<ModelStructuredOutput>,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Runtime {
    pub execution: Vec<RuntimeEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<Model>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_hints: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct GuardrailRail {
    pub id: String,
    pub provider: String,
    pub policy_ref: String,
    pub mode: Option<String>,
    pub categories: Option<Vec<String>>,
    pub threshold: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct Guardrails {
    pub input: Vec<GuardrailRail>,
    pub output: Vec<GuardrailRail>,
    pub on_violation: Option<String>,
    // v0.3.0 additions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interrupts: Option<Vec<Interrupt>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostGuardrail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_trust: Option<AgentTrust>,
}

// =============================================================================
// v0.3.0 Types
// =============================================================================

// --- Memory ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryLegacy {
    #[serde(rename = "type")]
    pub memory_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_history: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryStore {
    pub id: String,
    #[serde(rename = "type")]
    pub store_type: MemoryStoreType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<MemoryStoreScope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pii_policy: Option<PiiPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_clear: Option<AutoClearOn>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryStoreType {
    #[serde(rename = "episodic")]
    Episodic,
    #[serde(rename = "semantic")]
    Semantic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryStoreScope {
    #[serde(rename = "agent")]
    Agent,
    #[serde(rename = "session")]
    Session,
    #[serde(rename = "user")]
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PiiPolicy {
    #[serde(rename = "redact")]
    Redact,
    #[serde(rename = "encrypt")]
    Encrypt,
    #[serde(rename = "block")]
    Block,
    #[serde(rename = "log")]
    Log,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AutoClearOn {
    #[serde(rename = "session_end")]
    SessionEnd,
    #[serde(rename = "agent_stop")]
    AgentStop,
    #[serde(rename = "never")]
    Never,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryWorking {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<MemoryWorkingStrategy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compaction_threshold: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_model_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryWorkingStrategy {
    #[serde(rename = "sliding_window")]
    SlidingWindow,
    #[serde(rename = "full")]
    Full,
    #[serde(rename = "summary")]
    Summary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextAssembly {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<ContextAssemblySource>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<ContextAssemblyPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_injection: Option<Vec<StaticInjection>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContextAssemblySource {
    #[serde(rename = "working")]
    Working,
    #[serde(rename = "store")]
    Store,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContextAssemblyPosition {
    #[serde(rename = "prepend")]
    Prepend,
    #[serde(rename = "append")]
    Append,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryOperation {
    pub id: String,
    pub on_event: MemoryOperationOnEvent,
    pub op: MemoryOperationOp,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryOperationOnEvent {
    #[serde(rename = "on_invoke_end")]
    OnInvokeEnd,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemoryOperationOp {
    #[serde(rename = "write")]
    Write,
    #[serde(rename = "clear")]
    Clear,
    #[serde(rename = "summarize")]
    Summarize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryRetention {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_entries: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StaticInjection {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Memory {
    Legacy(MemoryLegacy),
    Structured(MemoryStructured),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryStructured {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stores: Option<Vec<MemoryStore>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working: Option<MemoryWorking>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_assembly: Option<ContextAssembly>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operations: Option<Vec<MemoryOperation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention: Option<MemoryRetention>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_injection: Option<Vec<StaticInjection>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

// --- Guardrails v0.3.0 ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Interrupt {
    pub id: String,
    pub trigger: InterruptTrigger,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_refs: Option<Vec<String>>,
    pub mode: InterruptMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_mode: Option<InterruptExecutionMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification: Option<InterruptNotification>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InterruptTrigger {
    #[serde(rename = "tool_call")]
    ToolCall,
    #[serde(rename = "cost_threshold")]
    CostThreshold,
    #[serde(rename = "loop_max_exceeded")]
    LoopMaxExceeded,
    #[serde(rename = "custom")]
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InterruptMode {
    #[serde(rename = "pause_and_notify")]
    PauseAndNotify,
    #[serde(rename = "block")]
    Block,
    #[serde(rename = "log")]
    Log,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InterruptExecutionMode {
    #[serde(rename = "blocking")]
    Blocking,
    #[serde(rename = "parallel")]
    Parallel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InterruptNotification {
    pub channel: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint_env_var: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_timeout: Option<InterruptOnTimeout>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InterruptOnTimeout {
    #[serde(rename = "fail")]
    Fail,
    #[serde(rename = "approve")]
    Approve,
    #[serde(rename = "deny")]
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CostGuardrail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_threshold_exceeded: Option<CostOnThresholdExceeded>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interrupt_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downgrade_model_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_by: Option<CostTrackBy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_refs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CostOnThresholdExceeded {
    #[serde(rename = "block")]
    Block,
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "interrupt")]
    Interrupt,
    #[serde(rename = "downgrade")]
    Downgrade,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CostTrackBy {
    #[serde(rename = "invocation")]
    Invocation,
    #[serde(rename = "session")]
    Session,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "day")]
    Day,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentTrust {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<AgentTrustLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side_effect_tool_refs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentTrustLevel {
    #[serde(rename = "sandboxed")]
    Sandboxed,
    #[serde(rename = "supervised")]
    Supervised,
    #[serde(rename = "autonomous")]
    Autonomous,
}

// --- Flow / Loop ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoopPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_max_iterations: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_max_exceeded: Option<LoopOnMaxExceeded>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LoopOnMaxExceeded {
    #[serde(rename = "fail")]
    Fail,
    #[serde(rename = "use_last")]
    UseLast,
    #[serde(rename = "escalate")]
    Escalate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suite_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adp_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    // v0.3.0 additions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_nodes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub termination: Option<Termination>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeKind {
    #[serde(rename = "input")]
    Input,
    #[serde(rename = "output")]
    Output,
    #[serde(rename = "llm")]
    LLM,
    #[serde(rename = "tool")]
    Tool,
    #[serde(rename = "router")]
    Router,
    #[serde(rename = "retriever")]
    Retriever,
    #[serde(rename = "evaluator")]
    Evaluator,
    #[serde(rename = "subflow")]
    Subflow,
    // v0.3.0 addition
    #[serde(rename = "loop")]
    Loop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Termination {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_max_exceeded: Option<LoopOnMaxExceeded>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_nodes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_nodes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Edge {
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Flow {
    pub id: String,
    pub graph: Graph,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_policy: Option<LoopPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

// --- Tools / Policy ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Auth {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<AuthScheme>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_var: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthScheme {
    #[serde(rename = "bearer")]
    Bearer,
    #[serde(rename = "api_key")]
    ApiKey,
    #[serde(rename = "oauth2")]
    OAuth2,
    #[serde(rename = "mtls")]
    MTLS,
    #[serde(rename = "none")]
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetryPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_delay_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_delay_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backoff_type: Option<BackoffType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retryable_status_codes: Option<Vec<u16>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackoffType {
    #[serde(rename = "fixed")]
    Fixed,
    #[serde(rename = "linear")]
    Linear,
    #[serde(rename = "exponential")]
    Exponential,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RateLimitPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_limit_exceeded: Option<RateLimitOnLimitExceeded>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RateLimitOnLimitExceeded {
    #[serde(rename = "queue")]
    Queue,
    #[serde(rename = "fail")]
    Fail,
    #[serde(rename = "warn")]
    Warn,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CachePolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<CacheScope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CacheScope {
    #[serde(rename = "agent")]
    Agent,
    #[serde(rename = "session")]
    Session,
    #[serde(rename = "user")]
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_strategy: Option<LoadStrategy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimitPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<CachePolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LoadStrategy {
    #[serde(rename = "eager")]
    Eager,
    #[serde(rename = "lazy")]
    Lazy,
    #[serde(rename = "on_demand")]
    OnDemand,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MCPServer {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<ToolPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HTTPAPI {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<ToolPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SQLFunction {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_url_env: Option<String>,
    #[serde(rename = "db_schema")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_schema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<ToolPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tools {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<Vec<MCPServer>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_apis: Option<Vec<HTTPAPI>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sql_functions: Option<Vec<SQLFunction>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<ToolPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

// --- Workspace ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceGit {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_commit: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspacePermissions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execute: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceMount {
    pub id: String,
    pub source: WorkspaceMountSource,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceMountSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mount_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkspaceCleanup {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_agent_stop: Option<WorkspaceCleanupOn>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_session_end: Option<WorkspaceCleanupOn>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retain_patterns: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WorkspaceCleanupOn {
    #[serde(rename = "delete")]
    Delete,
    #[serde(rename = "archive")]
    Archive,
    #[serde(rename = "preserve")]
    Preserve,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Workspace {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_env_var: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<WorkspaceGit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<WorkspacePermissions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mounts: Option<Vec<WorkspaceMount>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleanup: Option<WorkspaceCleanup>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

// --- Sandbox ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SandboxMount {
    pub id: String,
    pub source: SandboxMountSource,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SandboxMountSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SandboxSnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SandboxPolicy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_processes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<SandboxNetwork>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_hosts: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_ports: Option<Vec<u16>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SandboxNetwork {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "host")]
    Host,
    #[serde(rename = "bridge")]
    Bridge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Sandbox {
    pub runtime: SandboxRuntime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<SandboxProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mounts: Option<Vec<SandboxMount>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<SandboxSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<SandboxPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SandboxRuntime {
    #[serde(rename = "python")]
    Python,
    #[serde(rename = "node")]
    Node,
    #[serde(rename = "bash")]
    Bash,
    #[serde(rename = "browser")]
    Browser,
    #[serde(rename = "custom")]
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SandboxProvider {
    #[serde(rename = "docker")]
    Docker,
    #[serde(rename = "e2b")]
    E2B,
    #[serde(rename = "modal")]
    Modal,
    #[serde(rename = "daytona")]
    Daytona,
    #[serde(rename = "vercel")]
    Vercel,
    #[serde(rename = "cloudflare")]
    Cloudflare,
    #[serde(rename = "runloop")]
    Runloop,
    #[serde(rename = "blaxel")]
    Blaxel,
    #[serde(rename = "custom")]
    Custom,
}

// --- Artifacts ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArtifactStore {
    pub id: String,
    pub provider: ArtifactProvider,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<ArtifactScope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArtifactProvider {
    #[serde(rename = "gcs")]
    GCS,
    #[serde(rename = "s3")]
    S3,
    #[serde(rename = "azure_blob")]
    AzureBlob,
    #[serde(rename = "inmemory")]
    InMemory,
    #[serde(rename = "local")]
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArtifactScope {
    #[serde(rename = "session")]
    Session,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "agent")]
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Artifacts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stores: Option<Vec<ArtifactStore>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

// --- Observability ---

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tracing {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<TracingBackend>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_events: Option<Vec<TraceEvent>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TracingBackend {
    #[serde(rename = "otlp")]
    OTLP,
    #[serde(rename = "langfuse")]
    Langfuse,
    #[serde(rename = "langsmith")]
    Langsmith,
    #[serde(rename = "arize")]
    Arize,
    #[serde(rename = "phoenix")]
    Phoenix,
    #[serde(rename = "stdout")]
    Stdout,
    #[serde(rename = "none")]
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TraceEvent {
    #[serde(rename = "model_request")]
    ModelRequest,
    #[serde(rename = "tool_call")]
    ToolCall,
    #[serde(rename = "flow_node")]
    FlowNode,
    #[serde(rename = "loop_iteration")]
    LoopIteration,
    #[serde(rename = "interrupt")]
    Interrupt,
    #[serde(rename = "cost_check")]
    CostCheck,
    #[serde(rename = "artifact_write")]
    ArtifactWrite,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CostReporting {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granularity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_refs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Observability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracing: Option<Tracing>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_reporting: Option<CostReporting>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct Telemetry {
    pub endpoint: Option<String>,
    pub protocol: Option<String>,
    pub service_name: Option<String>,
    pub sampling_rate: Option<f64>,
    pub required_attributes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportEntry {
    pub id: String,
    #[serde(rename = "from")]
    pub from_uri: String,
    #[serde(default)]
    pub sections: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverrideEntry {
    pub path: String,
    pub value: Option<serde_json::Value>,
    #[serde(default = "default_op")]
    pub op: String,
}

fn default_op() -> String {
    "set".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Subagent {
    pub id: String,
    #[serde(rename = "ref")]
    pub ref_uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invocation_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InteropAgentSpecLlmBinding {
    pub backend_id: String,
    pub agentspec_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agentspec_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InteropAgentSpec {
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub ref_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_adapters: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_map: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_map: Option<Vec<InteropAgentSpecLlmBinding>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Interop {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub a2a: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agentspec: Option<InteropAgentSpec>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Adp {
    pub adp_version: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conformance_class: Option<String>,
    pub runtime: Runtime,
    pub flow: Flow,
    pub evaluation: serde_yaml::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,
    #[serde(rename = "import", skip_serializing_if = "Option::is_none")]
    pub imports: Option<Vec<ImportEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<Vec<OverrideEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guardrails: Option<Guardrails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telemetry: Option<Telemetry>,
    // v0.3.0 fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagents: Option<Vec<Subagent>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_testing: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Tools>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governance: Option<serde_json::Value>,
    // Additional v0.3.0 top-level fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<Memory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<Workspace>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox: Option<Sandbox>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<Artifacts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observability: Option<Observability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interop: Option<Interop>,
}

impl Default for Adp {
    fn default() -> Self {
        Adp {
            adp_version: String::new(),
            id: String::new(),
            conformance_class: None,
            runtime: Runtime {
                execution: Vec::new(),
                models: None,
                adapter_hints: None,
            },
            flow: Flow {
                id: String::new(),
                graph: Graph {
                    nodes: Vec::new(),
                    edges: Vec::new(),
                    start_nodes: None,
                    end_nodes: None,
                    extensions: None,
                },
                loop_policy: None,
                extensions: None,
            },
            evaluation: serde_yaml::Value::Null,
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
}

pub fn load_adp(path: &str) -> Result<Adp, Box<dyn std::error::Error>> {
    let data = fs::read_to_string(Path::new(path))?;
    Ok(serde_yaml::from_str(&data)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_op_via_deserialization() {
        let entry: OverrideEntry = serde_json::from_str(r#"{"path": "/id", "value": "test"}"#).unwrap();
        assert_eq!(entry.op, "set");
    }

    // ===== load_adp tests =====

    #[test]
    fn test_load_adp_happy_path() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, r#"adp_version: "0.1.0"
id: "loaded-agent"
runtime:
  execution:
    - id: "r1"
      backend: "python"
      entrypoint: "agent.main:app"
flow:
  id: "loaded.flow"
  graph:
    nodes:
      - id: "n1"
        kind: "input"
    edges: []
    start_nodes: ["n1"]
    end_nodes: ["n1"]
evaluation:
  suites:
    - id: "s1"
      metrics:
        - id: "m1"
          type: "deterministic"
          function: "noop"
          scoring: "boolean"
          threshold: true
"#).unwrap();
        let adp = load_adp(tmp.path().to_str().unwrap()).unwrap();
        assert_eq!(adp.id, "loaded-agent");
        assert_eq!(adp.adp_version, "0.1.0");
        assert_eq!(adp.runtime.execution[0].id, "r1");
    }

    #[test]
    fn test_load_adp_file_not_found() {
        let result = load_adp("/nonexistent/path/agent.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_adp_invalid_yaml() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, "{{invalid_yaml: [[[}}}}}}\n").unwrap();
        let result = load_adp(tmp.path().to_str().unwrap());
        assert!(result.is_err());
    }

    // ===== Adp::default tests =====

    #[test]
    fn test_adp_default() {
        let adp = Adp::default();
        assert_eq!(adp.adp_version, "");
        assert_eq!(adp.id, "");
        assert!(adp.conformance_class.is_none());
        assert!(adp.runtime.execution.is_empty());
        assert!(adp.runtime.models.is_none());
        assert_eq!(adp.flow.id, "");
        assert!(adp.flow.graph.nodes.is_empty());
        assert!(adp.flow.graph.edges.is_empty());
        assert!(adp.flow.graph.start_nodes.is_none());
        assert!(adp.flow.graph.end_nodes.is_none());
        assert!(adp.extends.is_none());
        assert!(adp.imports.is_none());
        assert!(adp.overrides.is_none());
        assert!(adp.guardrails.is_none());
        assert!(adp.telemetry.is_none());
        assert!(adp.subagents.is_none());
        assert!(adp.hooks.is_none());
        assert!(adp.tools.is_none());
        assert!(adp.governance.is_none());
        assert!(adp.memory.is_none());
        assert!(adp.workspace.is_none());
        assert!(adp.sandbox.is_none());
        assert!(adp.artifacts.is_none());
        assert!(adp.observability.is_none());
        assert!(adp.interop.is_none());
    }

    // ===== RuntimeEntry::default tests =====

    #[test]
    fn test_runtime_entry_default() {
        let entry = RuntimeEntry::default();
        assert_eq!(entry.backend, "");
        assert_eq!(entry.id, "");
        assert!(entry.entrypoint.is_none());
        assert!(entry.image.is_none());
        assert!(entry.module.is_none());
        assert!(entry.path.is_none());
        assert!(entry.backend_type.is_none());
        assert!(entry.endpoint.is_none());
        assert!(entry.package_manager.is_none());
    }

    // ===== Model::default tests =====

    #[test]
    fn test_model_default() {
        let m = Model::default();
        assert_eq!(m.id, "");
        assert_eq!(m.provider, "");
        assert_eq!(m.model, "");
        assert!(m.api_key_env.is_none());
        assert!(m.temperature.is_none());
        assert!(m.max_tokens.is_none());
        assert!(m.top_p.is_none());
        assert!(m.seed.is_none());
        assert!(m.timeout_ms.is_none());
        assert!(m.use_streaming_api.is_none());
        assert!(m.stop_sequences.is_none());
        assert!(m.frequency_penalty.is_none());
        assert!(m.presence_penalty.is_none());
        assert!(m.structured_output.is_none());
    }

    // ===== Telemetry::default tests =====

    #[test]
    fn test_telemetry_default() {
        let t = Telemetry::default();
        assert!(t.endpoint.is_none());
        assert!(t.protocol.is_none());
        assert!(t.service_name.is_none());
        assert!(t.sampling_rate.is_none());
        assert!(t.required_attributes.is_empty());
    }

    // ===== Serialization/deserialization round-trip tests =====

    #[test]
    fn test_node_kind_serialization() {
        let kinds = vec![
            NodeKind::Input,
            NodeKind::Output,
            NodeKind::LLM,
            NodeKind::Tool,
            NodeKind::Router,
            NodeKind::Retriever,
            NodeKind::Evaluator,
            NodeKind::Subflow,
            NodeKind::Loop,
        ];
        for kind in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            let back: NodeKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, back);
        }
    }

    #[test]
    fn test_auth_scheme_serialization() {
        let schemes = vec![
            AuthScheme::Bearer,
            AuthScheme::ApiKey,
            AuthScheme::OAuth2,
            AuthScheme::MTLS,
            AuthScheme::None,
        ];
        for scheme in schemes {
            let json = serde_json::to_string(&scheme).unwrap();
            let back: AuthScheme = serde_json::from_str(&json).unwrap();
            assert_eq!(scheme, back);
        }
    }

    #[test]
    fn test_memory_working_strategy_serialization() {
        let strategies = vec![
            MemoryWorkingStrategy::SlidingWindow,
            MemoryWorkingStrategy::Full,
            MemoryWorkingStrategy::Summary,
        ];
        for s in strategies {
            let json = serde_json::to_string(&s).unwrap();
            let back: MemoryWorkingStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(s, back);
        }
    }

    #[test]
    fn test_interrupt_trigger_serialization() {
        let triggers = vec![
            InterruptTrigger::ToolCall,
            InterruptTrigger::CostThreshold,
            InterruptTrigger::LoopMaxExceeded,
            InterruptTrigger::Custom,
        ];
        for t in triggers {
            let json = serde_json::to_string(&t).unwrap();
            let back: InterruptTrigger = serde_json::from_str(&json).unwrap();
            assert_eq!(t, back);
        }
    }

    #[test]
    fn test_interrupt_mode_serialization() {
        let modes = vec![
            InterruptMode::PauseAndNotify,
            InterruptMode::Block,
            InterruptMode::Log,
        ];
        for m in modes {
            let json = serde_json::to_string(&m).unwrap();
            let back: InterruptMode = serde_json::from_str(&json).unwrap();
            assert_eq!(m, back);
        }
    }

    #[test]
    fn test_interrupt_execution_mode_serialization() {
        let modes = vec![
            InterruptExecutionMode::Blocking,
            InterruptExecutionMode::Parallel,
        ];
        for m in modes {
            let json = serde_json::to_string(&m).unwrap();
            let back: InterruptExecutionMode = serde_json::from_str(&json).unwrap();
            assert_eq!(m, back);
        }
    }

    #[test]
    fn test_cost_on_threshold_exceeded_serialization() {
        let variants = vec![
            CostOnThresholdExceeded::Block,
            CostOnThresholdExceeded::Warn,
            CostOnThresholdExceeded::Interrupt,
            CostOnThresholdExceeded::Downgrade,
        ];
        for v in variants {
            let json = serde_json::to_string(&v).unwrap();
            let back: CostOnThresholdExceeded = serde_json::from_str(&json).unwrap();
            assert_eq!(v, back);
        }
    }

    #[test]
    fn test_sandbox_provider_serialization() {
        let providers = vec![
            SandboxProvider::Docker,
            SandboxProvider::E2B,
            SandboxProvider::Modal,
            SandboxProvider::Daytona,
            SandboxProvider::Vercel,
            SandboxProvider::Cloudflare,
            SandboxProvider::Runloop,
            SandboxProvider::Blaxel,
            SandboxProvider::Custom,
        ];
        for p in providers {
            let json = serde_json::to_string(&p).unwrap();
            let back: SandboxProvider = serde_json::from_str(&json).unwrap();
            assert_eq!(p, back);
        }
    }

    #[test]
    fn test_sandbox_runtime_serialization() {
        let runtimes = vec![
            SandboxRuntime::Python,
            SandboxRuntime::Node,
            SandboxRuntime::Bash,
            SandboxRuntime::Browser,
            SandboxRuntime::Custom,
        ];
        for r in runtimes {
            let json = serde_json::to_string(&r).unwrap();
            let back: SandboxRuntime = serde_json::from_str(&json).unwrap();
            assert_eq!(r, back);
        }
    }

    #[test]
    fn test_artifact_provider_serialization() {
        let providers = vec![
            ArtifactProvider::GCS,
            ArtifactProvider::S3,
            ArtifactProvider::AzureBlob,
            ArtifactProvider::InMemory,
            ArtifactProvider::Local,
        ];
        for p in providers {
            let json = serde_json::to_string(&p).unwrap();
            let back: ArtifactProvider = serde_json::from_str(&json).unwrap();
            assert_eq!(p, back);
        }
    }

    #[test]
    fn test_load_strategy_serialization() {
        let strategies = vec![
            LoadStrategy::Eager,
            LoadStrategy::Lazy,
            LoadStrategy::OnDemand,
        ];
        for s in strategies {
            let json = serde_json::to_string(&s).unwrap();
            let back: LoadStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(s, back);
        }
    }

    #[test]
    fn test_tracing_backend_serialization() {
        let backends = vec![
            TracingBackend::OTLP,
            TracingBackend::Langfuse,
            TracingBackend::Langsmith,
            TracingBackend::Arize,
            TracingBackend::Phoenix,
            TracingBackend::Stdout,
            TracingBackend::None,
        ];
        for b in backends {
            let json = serde_json::to_string(&b).unwrap();
            let back: TracingBackend = serde_json::from_str(&json).unwrap();
            assert_eq!(b, back);
        }
    }

    #[test]
    fn test_trace_event_serialization() {
        let events = vec![
            TraceEvent::ModelRequest,
            TraceEvent::ToolCall,
            TraceEvent::FlowNode,
            TraceEvent::LoopIteration,
            TraceEvent::Interrupt,
            TraceEvent::CostCheck,
            TraceEvent::ArtifactWrite,
        ];
        for e in events {
            let json = serde_json::to_string(&e).unwrap();
            let back: TraceEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(e, back);
        }
    }

    #[test]
    fn test_memory_store_type_serialization() {
        let types = vec![MemoryStoreType::Episodic, MemoryStoreType::Semantic];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let back: MemoryStoreType = serde_json::from_str(&json).unwrap();
            assert_eq!(t, back);
        }
    }

    #[test]
    fn test_memory_store_scope_serialization() {
        let scopes = vec![
            MemoryStoreScope::Agent,
            MemoryStoreScope::Session,
            MemoryStoreScope::User,
        ];
        for s in scopes {
            let json = serde_json::to_string(&s).unwrap();
            let back: MemoryStoreScope = serde_json::from_str(&json).unwrap();
            assert_eq!(s, back);
        }
    }

    #[test]
    fn test_pii_policy_serialization() {
        let policies = vec![
            PiiPolicy::Redact,
            PiiPolicy::Encrypt,
            PiiPolicy::Block,
            PiiPolicy::Log,
        ];
        for p in policies {
            let json = serde_json::to_string(&p).unwrap();
            let back: PiiPolicy = serde_json::from_str(&json).unwrap();
            assert_eq!(p, back);
        }
    }

    #[test]
    fn test_auto_clear_on_serialization() {
        let variants = vec![
            AutoClearOn::SessionEnd,
            AutoClearOn::AgentStop,
            AutoClearOn::Never,
        ];
        for v in variants {
            let json = serde_json::to_string(&v).unwrap();
            let back: AutoClearOn = serde_json::from_str(&json).unwrap();
            assert_eq!(v, back);
        }
    }

    #[test]
    fn test_context_assembly_source_serialization() {
        let sources = vec![ContextAssemblySource::Working, ContextAssemblySource::Store];
        for s in sources {
            let json = serde_json::to_string(&s).unwrap();
            let back: ContextAssemblySource = serde_json::from_str(&json).unwrap();
            assert_eq!(s, back);
        }
    }

    #[test]
    fn test_context_assembly_position_serialization() {
        let positions = vec![ContextAssemblyPosition::Prepend, ContextAssemblyPosition::Append];
        for p in positions {
            let json = serde_json::to_string(&p).unwrap();
            let back: ContextAssemblyPosition = serde_json::from_str(&json).unwrap();
            assert_eq!(p, back);
        }
    }

    #[test]
    fn test_memory_operation_on_event_serialization() {
        let event = MemoryOperationOnEvent::OnInvokeEnd;
        let json = serde_json::to_string(&event).unwrap();
        let back: MemoryOperationOnEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn test_memory_operation_op_serialization() {
        let ops = vec![
            MemoryOperationOp::Write,
            MemoryOperationOp::Clear,
            MemoryOperationOp::Summarize,
        ];
        for o in ops {
            let json = serde_json::to_string(&o).unwrap();
            let back: MemoryOperationOp = serde_json::from_str(&json).unwrap();
            assert_eq!(o, back);
        }
    }

    #[test]
    fn test_loop_on_max_exceeded_serialization() {
        let variants = vec![
            LoopOnMaxExceeded::Fail,
            LoopOnMaxExceeded::UseLast,
            LoopOnMaxExceeded::Escalate,
        ];
        for v in variants {
            let json = serde_json::to_string(&v).unwrap();
            let back: LoopOnMaxExceeded = serde_json::from_str(&json).unwrap();
            assert_eq!(v, back);
        }
    }

    #[test]
    fn test_backoff_type_serialization() {
        let types = vec![BackoffType::Fixed, BackoffType::Linear, BackoffType::Exponential];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let back: BackoffType = serde_json::from_str(&json).unwrap();
            assert_eq!(t, back);
        }
    }

    #[test]
    fn test_rate_limit_on_limit_exceeded_serialization() {
        let variants = vec![
            RateLimitOnLimitExceeded::Queue,
            RateLimitOnLimitExceeded::Fail,
            RateLimitOnLimitExceeded::Warn,
        ];
        for v in variants {
            let json = serde_json::to_string(&v).unwrap();
            let back: RateLimitOnLimitExceeded = serde_json::from_str(&json).unwrap();
            assert_eq!(v, back);
        }
    }

    #[test]
    fn test_cache_scope_serialization() {
        let scopes = vec![CacheScope::Agent, CacheScope::Session, CacheScope::User];
        for s in scopes {
            let json = serde_json::to_string(&s).unwrap();
            let back: CacheScope = serde_json::from_str(&json).unwrap();
            assert_eq!(s, back);
        }
    }

    #[test]
    fn test_workspace_cleanup_on_serialization() {
        let variants = vec![
            WorkspaceCleanupOn::Delete,
            WorkspaceCleanupOn::Archive,
            WorkspaceCleanupOn::Preserve,
        ];
        for v in variants {
            let json = serde_json::to_string(&v).unwrap();
            let back: WorkspaceCleanupOn = serde_json::from_str(&json).unwrap();
            assert_eq!(v, back);
        }
    }

    #[test]
    fn test_agent_trust_level_serialization() {
        let levels = vec![
            AgentTrustLevel::Sandboxed,
            AgentTrustLevel::Supervised,
            AgentTrustLevel::Autonomous,
        ];
        for l in levels {
            let json = serde_json::to_string(&l).unwrap();
            let back: AgentTrustLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(l, back);
        }
    }

    #[test]
    fn test_cost_track_by_serialization() {
        let variants = vec![
            CostTrackBy::Invocation,
            CostTrackBy::Session,
            CostTrackBy::User,
            CostTrackBy::Day,
        ];
        for v in variants {
            let json = serde_json::to_string(&v).unwrap();
            let back: CostTrackBy = serde_json::from_str(&json).unwrap();
            assert_eq!(v, back);
        }
    }

    #[test]
    fn test_interrupt_on_timeout_serialization() {
        let variants = vec![
            InterruptOnTimeout::Fail,
            InterruptOnTimeout::Approve,
            InterruptOnTimeout::Deny,
        ];
        for v in variants {
            let json = serde_json::to_string(&v).unwrap();
            let back: InterruptOnTimeout = serde_json::from_str(&json).unwrap();
            assert_eq!(v, back);
        }
    }

    #[test]
    fn test_sandbox_network_serialization() {
        let variants = vec![SandboxNetwork::None, SandboxNetwork::Host, SandboxNetwork::Bridge];
        for v in variants {
            let json = serde_json::to_string(&v).unwrap();
            let back: SandboxNetwork = serde_json::from_str(&json).unwrap();
            assert_eq!(v, back);
        }
    }

    #[test]
    fn test_artifact_scope_serialization() {
        let scopes = vec![ArtifactScope::Session, ArtifactScope::User, ArtifactScope::Agent];
        for s in scopes {
            let json = serde_json::to_string(&s).unwrap();
            let back: ArtifactScope = serde_json::from_str(&json).unwrap();
            assert_eq!(s, back);
        }
    }

    #[test]
    fn test_guardrail_rail_default() {
        let rail = GuardrailRail::default();
        assert_eq!(rail.id, "");
        assert_eq!(rail.provider, "");
        assert_eq!(rail.policy_ref, "");
        assert!(rail.mode.is_none());
        assert!(rail.categories.is_none());
        assert!(rail.threshold.is_none());
    }

    #[test]
    fn test_guardrails_default() {
        let g = Guardrails::default();
        assert!(g.input.is_empty());
        assert!(g.output.is_empty());
        assert!(g.on_violation.is_none());
        assert!(g.interrupts.is_none());
        assert!(g.cost.is_none());
        assert!(g.agent_trust.is_none());
    }

    #[test]
    fn test_runtime_default() {
        let r = Runtime::default();
        assert!(r.execution.is_empty());
        assert!(r.models.is_none());
        assert!(r.adapter_hints.is_none());
    }

    #[test]
    fn test_override_entry_with_explicit_op() {
        let entry: OverrideEntry = serde_json::from_str(r#"{"path": "/id", "value": "x", "op": "append"}"#).unwrap();
        assert_eq!(entry.op, "append");
        assert_eq!(entry.path, "/id");
    }

    #[test]
    fn test_memory_untagged_deserialization_legacy() {
        let json = r#"{"type": "buffer", "max_history": 10}"#;
        let memory: Memory = serde_json::from_str(json).unwrap();
        assert!(matches!(memory, Memory::Legacy(_)));
    }

    #[test]
    fn test_memory_untagged_deserialization_structured() {
        let json = r#"{"stores": []}"#;
        let memory: Memory = serde_json::from_str(json).unwrap();
        assert!(matches!(memory, Memory::Structured(_)));
    }

    #[test]
    fn test_model_structured_output_default() {
        let so = ModelStructuredOutput::default();
        assert!(so.format.is_none());
        assert!(so.schema.is_none());
        assert!(so.schema_ref.is_none());
    }

    #[test]
    fn test_full_adp_deserialization() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(tmp, r#"adp_version: "0.3.0"
id: "full-agent"
conformance_class: "full"
runtime:
  execution:
    - id: "r1"
      backend: "python"
      entrypoint: "agent.main:app"
  models:
    - id: "gpt4"
      provider: "openai"
      model: "gpt-4"
      temperature: 0.7
      max_tokens: 4096
      top_p: 0.9
      seed: 42
      timeout_ms: 30000
      use_streaming_api: true
      stop_sequences: ["END"]
      frequency_penalty: 0.5
      presence_penalty: 0.3
flow:
  id: "full.flow"
  graph:
    nodes:
      - id: "n1"
        kind: "input"
    edges: []
    start_nodes: ["n1"]
    end_nodes: ["n1"]
evaluation:
  suites:
    - id: "s1"
      metrics:
        - id: "m1"
          type: "deterministic"
          function: "noop"
          scoring: "boolean"
          threshold: true
telemetry:
  endpoint: "http://otel-collector:4317"
  protocol: "grpc"
  service_name: "my-agent"
  sampling_rate: 0.5
  required_attributes:
    - "gen_ai.request.model"
"#).unwrap();
        let adp = load_adp(tmp.path().to_str().unwrap()).unwrap();
        assert_eq!(adp.id, "full-agent");
        assert_eq!(adp.adp_version, "0.3.0");
        assert_eq!(adp.conformance_class, Some("full".to_string()));
        let models = adp.runtime.models.unwrap();
        assert_eq!(models[0].id, "gpt4");
        assert_eq!(models[0].temperature, Some(0.7));
        assert_eq!(models[0].top_p, Some(0.9));
        assert_eq!(models[0].seed, Some(42));
        let tel = adp.telemetry.unwrap();
        assert_eq!(tel.endpoint, Some("http://otel-collector:4317".to_string()));
        assert_eq!(tel.required_attributes, vec!["gen_ai.request.model"]);
    }
}
