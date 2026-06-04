import fs from "fs";
import yaml from "js-yaml";
import { validateAdp } from "./validation";

// ============================================================================
// Enums for v0.3.0
// =============================================================================

export type MemoryStoreType = "episodic" | "semantic";
export type MemoryStoreScope = "agent" | "session" | "user";
export type MemoryWorkingStrategy = "sliding_window" | "full" | "summary";
export type ContextAssemblySource = "working" | "store";
export type ContextAssemblyPosition = "prepend" | "append";
export type MemoryOperationOnEvent = "on_invoke_end";
export type MemoryOperationOp = "write" | "clear" | "summarize";
export type PiiPolicy = "redact" | "encrypt" | "block" | "log";
export type AutoClearOn = "session_end" | "agent_stop" | "never";
export type WorkspaceCleanupOn = "agent_stop" | "session_end" | "never";
export type MountProvider = "s3" | "gcs" | "azure_blob" | "cloudflare_r2";
export type SandboxRuntime = "python" | "node" | "bash" | "browser" | "custom";
export type SandboxProvider = 
  | "docker" | "e2b" | "modal" | "daytona" | "vercel" | "cloudflare" | "runloop" | "blaxel" | "custom";
export type SandboxNetwork = "none" | "host" | "bridge";
export type SandboxRestoreOn = "failure" | "always" | "never";
export type ArtifactScope = "session" | "user" | "agent";
export type ArtifactProvider = "gcs" | "s3" | "azure_blob" | "inmemory" | "local";
export type NodeKind = 
  | "input" | "output" | "llm" | "tool" | "router" | "retriever" | "evaluator" | "subflow" | "loop";
export type LoopOnMaxExceeded = "fail" | "use_last" | "escalate";
export type InterruptTrigger = "tool_call" | "cost_threshold" | "loop_max_exceeded" | "custom";
export type InterruptMode = "pause_and_notify" | "block" | "log";
export type InterruptExecutionMode = "blocking" | "parallel";
export type InterruptOnTimeout = "fail" | "approve" | "deny";
export type CostOnThresholdExceeded = "block" | "warn" | "interrupt" | "downgrade";
export type CostTrackBy = "invocation" | "session" | "user" | "day";
export type AgentTrustLevel = "sandboxed" | "supervised" | "autonomous";
export type TracingBackend = 
  | "otlp" | "langfuse" | "langsmith" | "arize" | "phoenix" | "stdout" | "none";
export type TraceEvent = 
  | "model_request" | "tool_call" | "flow_node" | "loop_iteration"
  | "interrupt" | "cost_check" | "artifact_write";
export type LoadStrategy = "eager" | "lazy" | "on_demand";
export type BackoffType = "fixed" | "linear" | "exponential";
export type RateLimitOnLimitExceeded = "queue" | "fail" | "warn";
export type CacheScope = "agent" | "session" | "user";
export type AuthScheme = "bearer" | "api_key" | "oauth2" | "mtls" | "none";
export type GuardrailMode = "block" | "flag" | "redact" | "log";
export type GuardrailThreshold = "low" | "medium" | "high";

// =============================================================================
// Model Structured Output (existing)
// =============================================================================

export interface ModelStructuredOutput {
  format?: "json_object" | "json_schema" | "text";
  schema?: Record<string, unknown>;
  schema_ref?: string;
}

// =============================================================================
// Model Config (existing, extended with load_strategy)
// =============================================================================

export interface ModelConfig {
  id: string;
  provider: string;
  model: string;
  temperature?: number;
  max_tokens?: number;
  top_p?: number;
  seed?: number;
  timeout_ms?: number;
  use_streaming_api?: boolean;
  stop_sequences?: string[];
  frequency_penalty?: number;
  presence_penalty?: number;
  structured_output?: ModelStructuredOutput;
  extensions?: Record<string, unknown>;
  [key: string]: unknown;
}

// =============================================================================
// Adapter Hints (extended for v0.3.0)
// =============================================================================

export interface AdapterHints {
  langgraph?: {
    recursion_limit?: number;
    stream_mode?: "values" | "updates" | "debug";
    checkpointer?: "memory" | "sqlite" | "postgres" | "none";
    memory_store?: string;
    [key: string]: unknown;
  };
  autogen?: {
    max_turns?: number;
    human_input_mode?: "NEVER" | "TERMINATE" | "ALWAYS";
    [key: string]: unknown;
  };
  crewai?: {
    process?: "sequential" | "hierarchical" | "parallel";
    verbose?: boolean;
    memory?: boolean;
    max_rpm?: number;
    [key: string]: unknown;
  };
  llamaindex?: {
    embedder_config?: Record<string, unknown>;
    [key: string]: unknown;
  };
  google_adk?: {
    memory_store?: string;
    [key: string]: unknown;
  };
  openai_agents?: {
    [key: string]: unknown;
  };
  pydantic_ai?: {
    embedder_config?: Record<string, unknown>;
    [key: string]: unknown;
  };
  semantic_kernel?: {
    execution_type?: "sequential" | "stepwise";
    [key: string]: unknown;
  };
  [key: string]: unknown;
}

// =============================================================================
// Runtime (existing)
// =============================================================================

export interface Runtime {
  execution: Array<{ id: string; backend: string; [key: string]: unknown }>;
  models?: ModelConfig[];
  adapter_hints?: AdapterHints;
}

// =============================================================================
// Flow Types (extended for v0.3.0)
// =============================================================================

export interface FlowNodeTermination {
  condition?: string;
  max_iterations?: number;
  max_tokens?: number;
  on_max_exceeded?: LoopOnMaxExceeded;
  restart_context?: boolean;
}

export interface FlowNode {
  id: string;
  kind: NodeKind;
  label?: string;
  model_ref?: string;
  system_prompt_ref?: string;
  prompt_ref?: string;
  tool_ref?: string;
  adp_ref?: string;
  flow_ref?: string;
  memory_ref?: string;
  suite_ref?: string;
  output_ref?: string;
  runtime_ref?: string;
  blocking?: boolean;
  strategy?: string; // for router nodes
  params?: Record<string, unknown>;
  // v0.3.0 loop additions
  body_nodes?: string[];
  termination?: FlowNodeTermination;
  [key: string]: unknown;
}

export interface LoopPolicy {
  default_max_iterations?: number;
  on_max_exceeded?: LoopOnMaxExceeded;
  total_run_max_iterations?: number;
}

export interface Flow {
  id: string;
  graph: {
    nodes: FlowNode[];
    edges: Array<{ from: string; to: string; condition?: string }>;
    start_nodes: string[];
    end_nodes: string[];
  };
  loop_policy?: LoopPolicy;
  extensions?: Record<string, unknown>;
}

// =============================================================================
// Evaluation (existing)
// =============================================================================

export interface EvaluationMetric {
  id: string;
  type: string;
  function: string;
  scoring: string;
  threshold: unknown;
}

export interface EvaluationSuite {
  id: string;
  metrics: EvaluationMetric[];
}

export interface Evaluation {
  suites: EvaluationSuite[];
  promotion_policy?: { require_passing_suites?: string[] };
}

// =============================================================================
// Guardrails (extended for v0.3.0)
// =============================================================================

export interface GuardrailRail {
  id: string;
  provider: string;
  policy_ref: string;
  mode?: GuardrailMode;
  categories?: string[];
  threshold?: GuardrailThreshold;
  [key: string]: unknown;
}

export interface InterruptNotification {
  channel: string;
  endpoint_env_var?: string;
  timeout_seconds?: number;
  on_timeout?: InterruptOnTimeout;
}

export interface Interrupt {
  id: string;
  trigger: InterruptTrigger;
  tool_refs?: string[];
  mode: InterruptMode;
  execution_mode?: InterruptExecutionMode;
  notification?: InterruptNotification;
}

export interface CostGuardrail {
  threshold_usd?: number;
  on_threshold_exceeded?: CostOnThresholdExceeded;
  interrupt_ref?: string;
  downgrade_model_ref?: string;
  track_by?: CostTrackBy;
  model_refs?: string[];
}

export interface AgentTrust {
  level?: AgentTrustLevel;
  side_effect_tool_refs?: string[];
}

export interface Guardrails {
  input?: GuardrailRail[];
  output?: GuardrailRail[];
  on_violation?: string;
  // v0.3.0 additions
  interrupts?: Interrupt[];
  cost?: CostGuardrail;
  agent_trust?: AgentTrust;
}

// =============================================================================
// Telemetry (existing)
// =============================================================================

export interface Telemetry {
  endpoint?: string;
  protocol?: string;
  service_name?: string;
  sampling_rate?: number;
  required_attributes?: string[];
}

// =============================================================================
// Import/Override (existing)
// =============================================================================

export interface ImportEntry {
  id: string;
  from: string; // serialized as "from" in JSON/YAML
  sections?: string[];
}

export interface OverrideEntry {
  path: string;
  value?: unknown;
  op?: "set" | "delete" | "append";
}

// =============================================================================
// Subagent (existing)
// =============================================================================

export interface Subagent {
  id: string;
  ref: string;
  description?: string;
  invocation_mode?: "synchronous" | "asynchronous";
}

// =============================================================================
// Pipeline (existing)
// =============================================================================

export interface PipelineStage {
  id: string;
  type: "function" | "script" | "json_schema";
  description?: string;
  function_ref?: string;
  runtime?: "python" | "bash" | "javascript";
  inline?: string;
  script_ref?: string;
  schema?: Record<string, unknown>;
  schema_ref?: string;
  on_error?: "fail" | "warn" | "skip";
}

export interface Pipeline {
  pre_process?: PipelineStage[];
  post_process?: PipelineStage[];
}

// =============================================================================
// Hooks (existing)
// =============================================================================

export interface HookHandler {
  type: "function" | "script";
  function_ref?: string;
  runtime?: "python" | "bash" | "javascript";
  inline?: string;
  script_ref?: string;
}

export interface Hook {
  event:
    | "on_invoke_start"
    | "on_invoke_end"
    | "on_node_start"
    | "on_node_end"
    | "on_stream_start"
    | "on_stream_chunk"
    | "on_stream_end"
    | "on_error";
  node_filter?: string[];
  handler: HookHandler;
  on_error?: "log" | "fail" | "skip";
}

// =============================================================================
// Streaming (existing)
// =============================================================================

export interface Streaming {
  enabled?: boolean;
  mode?: "token" | "message" | "event" | "none";
  chunk_format?: "text" | "json" | "server_sent_events";
  buffer_lines?: number;
  include_node_events?: boolean;
}

// =============================================================================
// Interop Types
// =============================================================================

export interface InteropAgentSpecLLMBinding {
  backend_id: string;
  agentspec_id: string;
  agentspec_type?: string;
}

export interface InteropAgentSpec {
  ref?: string;
  version?: string;
  component_type?: "Agent" | "Flow";
  component_id?: string;
  runtime_adapters?: string[];
  node_map?: Record<string, string>;
  llm_map?: InteropAgentSpecLLMBinding[];
}

export interface Interop {
  a2a?: Record<string, unknown>;
  agentspec?: InteropAgentSpec;
  [key: string]: unknown;
}

// =============================================================================
// A2A Types (existing)
// =============================================================================

export interface A2AAuthentication {
  schemes: string[];
  credentials?: string;
  oauth2?: {
    authorization_url?: string;
    token_url?: string;
    scopes?: string[];
  };
  [key: string]: unknown;
}

export interface A2ASkill {
  id: string;
  name: string;
  description?: string;
  tags?: string[];
  examples?: string[];
  input_modes?: string[];
  output_modes?: string[];
}

export interface A2AAgentCard {
  name: string;
  url: string;
  version: string;
  description?: string;
  documentation_url?: string;
  provider?: { organization?: string; url?: string };
  capabilities?: {
    streaming?: boolean;
    push_notifications?: boolean;
    state_transition_history?: boolean;
  };
  authentication?: A2AAuthentication;
  default_input_modes?: string[];
  default_output_modes?: string[];
  skills?: A2ASkill[];
}

export interface A2AAuthenticatedExtension {
  capabilities?: {
    streaming?: boolean;
    push_notifications?: boolean;
    state_transition_history?: boolean;
  };
  authentication?: A2AAuthentication;
  default_input_modes?: string[];
  default_output_modes?: string[];
  skills?: A2ASkill[];
  push_notification_config?: { endpoint?: string; auth_scheme?: string };
}

// =============================================================================
// Tools Types (extended for v0.3.0)
// =============================================================================

export interface Auth {
  scheme: AuthScheme;
  env_var?: string;
  scopes?: string[];
}

export interface RetryPolicy {
  max_attempts?: number;
  backoff?: BackoffType;
  backoff_base_ms?: number;
  max_delay_ms?: number;
  retryable_status_codes?: number[];
}

export interface RateLimitPolicy {
  requests_per_minute?: number;
  burst?: number;
  on_limit_exceeded?: RateLimitOnLimitExceeded;
}

export interface CachePolicy {
  enabled?: boolean;
  ttl_seconds?: number;
  key_fields?: string[];
  scope?: CacheScope;
}

export interface ToolPolicy {
  retry?: RetryPolicy;
  timeout_ms?: number;
  rate_limit?: RateLimitPolicy;
  cache?: CachePolicy;
}

export interface ToolBase {
  id: string;
  description: string;
  auth?: Auth;
  load_strategy?: LoadStrategy;
  policy?: ToolPolicy;
}

export interface MCPServer extends ToolBase {
  transport: string;
  endpoint: string;
}

export interface HTTPAPI extends ToolBase {
  base_url: string;
}

export interface SQLFunction extends ToolBase {
  connection: string;
  schema?: string;
}

// =============================================================================
// Sandbox Types (v0.3.0)
// =============================================================================

export interface SandboxMount {
  source: string;
  target: string;
  read_only?: boolean;
}

export interface SandboxSnapshot {
  enabled?: boolean;
  restore_on?: SandboxRestoreOn;
}

export interface SandboxPolicy {
  timeout_ms: number;
  max_output_bytes?: number;
  network?: SandboxNetwork;
  allow_filesystem_writes?: boolean;
}

export interface SandboxTool {
  id: string;
  runtime: SandboxRuntime;
  version?: string;
  image?: string | null;
  provider?: SandboxProvider;
  mounts?: SandboxMount[];
  env?: Record<string, string>;
  snapshot?: SandboxSnapshot;
  policy: SandboxPolicy;
}

export interface Tools {
  mcp_servers?: MCPServer[];
  http_apis?: HTTPAPI[];
  sql_functions?: SQLFunction[];
  sandbox?: SandboxTool[];
}

// =============================================================================
// Memory Types (v0.3.0)
// =============================================================================

export interface MemoryLegacy {
  provider?: string;
  endpoint?: string;
  index?: string;
  namespace?: string;
}

export interface MemoryStore {
  id: string;
  type: MemoryStoreType;
  provider: string;
  endpoint?: string;
  index?: string;
  scope?: MemoryStoreScope;
  ttl_seconds?: number;
  pii?: boolean;
}

export interface MemoryWorking {
  strategy?: MemoryWorkingStrategy;
  window_size?: number;
  max_tokens?: number;
  summary_model_ref?: string;
  compaction_threshold_tokens?: number;
}

export interface MemoryContextAssemblySource {
  source: ContextAssemblySource;
  store_ref?: string;
  top_k?: number;
  relevance_threshold?: number;
}

export interface MemoryStaticInjection {
  id: string;
  source: "file" | "inline";
  path?: string;
  content?: string;
  position: ContextAssemblyPosition;
  max_tokens?: number;
}

export interface MemoryContextAssembly {
  apply_to_node_kinds?: string[];
  order?: MemoryContextAssemblySource[];
  max_total_tokens?: number;
  static_injection?: MemoryStaticInjection[];
}

export interface MemoryOperation {
  on_event: MemoryOperationOnEvent;
  op: MemoryOperationOp;
  store_ref?: string;
  fields?: string[];
  when?: string;
}

export interface MemoryRetention {
  pii_policy?: PiiPolicy;
  user_consent_required?: boolean;
  data_residency?: string[];
  auto_clear_on?: AutoClearOn;
}

export interface MemoryStructured {
  stores?: MemoryStore[];
  working?: MemoryWorking;
  context_assembly?: MemoryContextAssembly;
  operations?: MemoryOperation[];
  retention?: MemoryRetention;
}

export type Memory = MemoryLegacy | MemoryStructured;

// =============================================================================
// Workspace Types (v0.3.0)
// =============================================================================

export interface WorkspaceGit {
  enabled?: boolean;
  auto_commit?: boolean;
  branch_per_session?: boolean;
}

export interface WorkspacePermissions {
  read?: string[];
  write?: string[];
  exec?: string[];
}

export interface WorkspaceMount {
  id: string;
  provider: MountProvider;
  bucket?: string;
  prefix?: string;
  target: string;
  read_only?: boolean;
  credentials_env_var?: string;
}

export interface WorkspaceCleanup {
  on?: WorkspaceCleanupOn;
  exclude?: string[];
}

export interface Workspace {
  root?: string;
  root_env_var?: string;
  git?: WorkspaceGit;
  permissions?: WorkspacePermissions;
  mounts?: WorkspaceMount[];
  cleanup?: WorkspaceCleanup;
}

// =============================================================================
// Artifacts Types (v0.3.0)
// =============================================================================

export interface ArtifactStore {
  id: string;
  scope: ArtifactScope;
  provider: ArtifactProvider;
  bucket?: string;
  path_prefix?: string;
  ttl_seconds?: number;
  versioned?: boolean;
  credentials_env_var?: string;
}

export interface Artifacts {
  stores: ArtifactStore[];
}

// =============================================================================
// Observability Types (v0.3.0)
// =============================================================================

export interface Tracing {
  backend?: TracingBackend;
  endpoint_env_var?: string;
  api_key_env_var?: string;
  trace_events?: TraceEvent[];
  sampling_rate?: number;
  service_name?: string;
}

export interface CostReporting {
  enabled?: boolean;
  track_by?: CostTrackBy;
  emit_metric?: string;
  model_refs?: string[];
}

export interface Observability {
  tracing?: Tracing;
  cost_reporting?: CostReporting;
}

// =============================================================================
// Main ADP Interface (extended for v0.3.0)
// =============================================================================

export interface ADP {
  adp_version: string;
  id: string;
  runtime: Runtime;
  flow: Flow;
  evaluation: Evaluation;
  extends?: string;
  imports?: ImportEntry[]; // JSON key is "import"
  overrides?: OverrideEntry[];
  guardrails?: Guardrails;
  telemetry?: Telemetry;
  subagents?: Subagent[];
  pipeline?: Pipeline;
  hooks?: Hook[];
  streaming?: Streaming;
  // v0.3.0 additions
  memory?: Memory;
  workspace?: Workspace;
  artifacts?: Artifacts;
  observability?: Observability;
  tools?: Tools;
  interop?: Interop;
  [key: string]: unknown;
}

export function parseADP(path: string): ADP {
  const data = yaml.load(fs.readFileSync(path, "utf8")) as ADP;
  return data;
}

export function validateADPFile(path: string): string[] {
  const adp = parseADP(path);
  return validateAdp(adp);
}
