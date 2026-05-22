import fs from "fs";
import yaml from "js-yaml";
import { validateAdp } from "./validation";

export interface ModelStructuredOutput {
  format?: "json_object" | "json_schema" | "text";
  schema?: Record<string, unknown>;
  schema_ref?: string;
}

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

export interface AdapterHints {
  langgraph?: {
    recursion_limit?: number;
    stream_mode?: "values" | "updates" | "debug";
    checkpointer?: "memory" | "sqlite" | "postgres" | "none";
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
    [key: string]: unknown;
  };
  semantic_kernel?: {
    execution_type?: "sequential" | "stepwise";
    [key: string]: unknown;
  };
  [key: string]: unknown;
}

export interface Runtime {
  execution: Array<{ id: string; backend: string; [key: string]: unknown }>;
  models?: ModelConfig[];
  adapter_hints?: AdapterHints;
}

export interface FlowNode {
  id: string;
  kind: "input" | "output" | "llm" | "tool" | "router" | "retriever" | "evaluator" | "subflow";
  [key: string]: unknown;
}

export interface Flow {
  id: string;
  graph: {
    nodes: FlowNode[];
    edges: Array<{ from: string; to: string; condition?: string }>;
    start_nodes: string[];
    end_nodes: string[];
  };
}

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

export interface GuardrailRail {
  id: string;
  provider: string;
  policy_ref: string;
  mode?: string;
  categories?: string[];
  threshold?: string;
  [key: string]: unknown;
}

export interface Guardrails {
  input?: GuardrailRail[];
  output?: GuardrailRail[];
  on_violation?: string;
}

export interface Telemetry {
  endpoint?: string;
  protocol?: string;
  service_name?: string;
  sampling_rate?: number;
  required_attributes?: string[];
}

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

export interface Subagent {
  id: string;
  ref: string;
  description?: string;
  invocation_mode?: "synchronous" | "asynchronous";
}

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

export interface Streaming {
  enabled?: boolean;
  mode?: "token" | "message" | "event" | "none";
  chunk_format?: "text" | "json" | "server_sent_events";
  buffer_lines?: number;
  include_node_events?: boolean;
}

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
