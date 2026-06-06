package adp

import (
	"gopkg.in/yaml.v3"
	"os"
)

type RuntimeEntry struct {
	Backend        string `yaml:"backend"          json:"backend"`
	ID             string `yaml:"id"               json:"id"`
	Entrypoint     string `yaml:"entrypoint,omitempty"  json:"entrypoint,omitempty"`
	Image          string `yaml:"image,omitempty"       json:"image,omitempty"`
	Module         string `yaml:"module,omitempty"      json:"module,omitempty"`
	Path           string `yaml:"path,omitempty"        json:"path,omitempty"`
	BackendType    string `yaml:"type,omitempty"        json:"type,omitempty"`
	Endpoint       string `yaml:"endpoint,omitempty"    json:"endpoint,omitempty"`
	PackageManager string `yaml:"package_manager,omitempty" json:"package_manager,omitempty"`
}

type ModelStructuredOutput struct {
	Format    string                 `yaml:"format,omitempty"     json:"format,omitempty"`
	Schema    map[string]interface{} `yaml:"schema,omitempty"     json:"schema,omitempty"`
	SchemaRef string                 `yaml:"schema_ref,omitempty" json:"schema_ref,omitempty"`
}

type Model struct {
	ID          string                 `yaml:"id"                        json:"id"`
	Provider    string                 `yaml:"provider"                  json:"provider"`
	Model       string                 `yaml:"model"                     json:"model"`
	APIKeyEnv   string                 `yaml:"api_key_env,omitempty"     json:"api_key_env,omitempty"`
	BaseURL     string                 `yaml:"base_url,omitempty"        json:"base_url,omitempty"`
	Temperature *float64               `yaml:"temperature,omitempty"     json:"temperature,omitempty"`
	MaxTokens   *int                   `yaml:"max_tokens,omitempty"      json:"max_tokens,omitempty"`
	Extensions  map[string]interface{} `yaml:"extensions,omitempty"      json:"extensions,omitempty"`
	// v0.3.0 model parameters
	TopP             *float64               `yaml:"top_p,omitempty"           json:"top_p,omitempty"`
	Seed             *int64                 `yaml:"seed,omitempty"            json:"seed,omitempty"`
	TimeoutMs        *int                   `yaml:"timeout_ms,omitempty"      json:"timeout_ms,omitempty"`
	UseStreamingAPI  *bool                  `yaml:"use_streaming_api,omitempty" json:"use_streaming_api,omitempty"`
	StopSequences    []string               `yaml:"stop_sequences,omitempty"  json:"stop_sequences,omitempty"`
	FrequencyPenalty *float64               `yaml:"frequency_penalty,omitempty" json:"frequency_penalty,omitempty"`
	PresencePenalty  *float64               `yaml:"presence_penalty,omitempty"  json:"presence_penalty,omitempty"`
	StructuredOutput *ModelStructuredOutput `yaml:"structured_output,omitempty" json:"structured_output,omitempty"`
}

type Runtime struct {
	Execution    []RuntimeEntry         `yaml:"execution"             json:"execution"`
	Models       []Model                `yaml:"models,omitempty"      json:"models,omitempty"`
	AdapterHints map[string]interface{} `yaml:"adapter_hints,omitempty" json:"adapter_hints,omitempty"`
}

type Subagent struct {
	ID             string `yaml:"id"                        json:"id"`
	Ref            string `yaml:"ref"                       json:"ref"`
	Description    string `yaml:"description,omitempty"     json:"description,omitempty"`
	InvocationMode string `yaml:"invocation_mode,omitempty" json:"invocation_mode,omitempty"`
}

// GuardrailRail represents a single guardrail policy rail.
type GuardrailRail struct {
	ID         string   `yaml:"id"                   json:"id"`
	Provider   string   `yaml:"provider"             json:"provider"`
	PolicyRef  string   `yaml:"policy_ref"           json:"policy_ref"`
	Mode       string   `yaml:"mode,omitempty"       json:"mode,omitempty"`
	Categories []string `yaml:"categories,omitempty" json:"categories,omitempty"`
	Threshold  string   `yaml:"threshold,omitempty"  json:"threshold,omitempty"`
}

// Guardrails defines input/output content guardrails.
type Guardrails struct {
	Input       []GuardrailRail `yaml:"input,omitempty"        json:"input,omitempty"`
	Output      []GuardrailRail `yaml:"output,omitempty"       json:"output,omitempty"`
	OnViolation string          `yaml:"on_violation,omitempty" json:"on_violation,omitempty"`
	// v0.3.0 additions
	Interrupts []Interrupt    `yaml:"interrupts,omitempty"   json:"interrupts,omitempty"`
	Cost       *CostGuardrail `yaml:"cost,omitempty"        json:"cost,omitempty"`
	AgentTrust *AgentTrust    `yaml:"agent_trust,omitempty"  json:"agent_trust,omitempty"`
}

// Interrupt defines an interrupt condition for guardrails
type Interrupt struct {
	ID            string        `yaml:"id"                  json:"id"`
	Trigger       string        `yaml:"trigger"             json:"trigger"`
	ToolRefs      []string      `yaml:"tool_refs,omitempty"  json:"tool_refs,omitempty"`
	Mode          string        `yaml:"mode"                json:"mode"`
	ExecutionMode string        `yaml:"execution_mode,omitempty" json:"execution_mode,omitempty"`
	Notification  *Notification `yaml:"notification,omitempty" json:"notification,omitempty"`
	ThresholdUSD  *float64      `yaml:"threshold_usd,omitempty" json:"threshold_usd,omitempty"`
	Extensions    interface{}   `yaml:"extensions,omitempty"  json:"extensions,omitempty"`
}

// Notification defines how interrupts notify external systems
type Notification struct {
	Channel        string      `yaml:"channel"             json:"channel"`
	EndpointEnvVar string      `yaml:"endpoint_env_var,omitempty" json:"endpoint_env_var,omitempty"`
	TimeoutSeconds *int64      `yaml:"timeout_seconds,omitempty" json:"timeout_seconds,omitempty"`
	OnTimeout      string      `yaml:"on_timeout,omitempty"     json:"on_timeout,omitempty"`
	Extensions     interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// CostGuardrail defines cost-based guardrails
type CostGuardrail struct {
	ThresholdUSD        *float64    `yaml:"threshold_usd,omitempty" json:"threshold_usd,omitempty"`
	OnThresholdExceeded string      `yaml:"on_threshold_exceeded,omitempty" json:"on_threshold_exceeded,omitempty"`
	InterruptRef        string      `yaml:"interrupt_ref,omitempty" json:"interrupt_ref,omitempty"`
	DowngradeModelRef   string      `yaml:"downgrade_model_ref,omitempty" json:"downgrade_model_ref,omitempty"`
	TrackBy             string      `yaml:"track_by,omitempty" json:"track_by,omitempty"`
	ModelRefs           []string    `yaml:"model_refs,omitempty" json:"model_refs,omitempty"`
	Extensions          interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// AgentTrust defines agent trust levels
type AgentTrust struct {
	Level              string      `yaml:"level,omitempty"          json:"level,omitempty"`
	SideEffectToolRefs []string    `yaml:"side_effect_tool_refs,omitempty" json:"side_effect_tool_refs,omitempty"`
	Extensions         interface{} `yaml:"extensions,omitempty"  json:"extensions,omitempty"`
}

// LoopPolicy defines loop execution policies
type LoopPolicy struct {
	DefaultMaxIterations *int        `yaml:"default_max_iterations,omitempty" json:"default_max_iterations,omitempty"`
	OnMaxExceeded        string      `yaml:"on_max_exceeded,omitempty" json:"on_max_exceeded,omitempty"`
	Extensions           interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// NodeKind defines the type of a flow node
type NodeKind string

const (
	NodeKindInput     NodeKind = "input"
	NodeKindOutput    NodeKind = "output"
	NodeKindLLM       NodeKind = "llm"
	NodeKindTool      NodeKind = "tool"
	NodeKindRouter    NodeKind = "router"
	NodeKindRetriever NodeKind = "retriever"
	NodeKindEvaluator NodeKind = "evaluator"
	NodeKindSubflow   NodeKind = "subflow"
	NodeKindLoop      NodeKind = "loop"
)

// Node defines a node in the flow graph
type Node struct {
	ID          string       `yaml:"id"                  json:"id"`
	Kind        NodeKind     `yaml:"kind"                json:"kind"`
	ModelRef    string       `yaml:"model_ref,omitempty"  json:"model_ref,omitempty"`
	ToolRef     string       `yaml:"tool_ref,omitempty"   json:"tool_ref,omitempty"`
	RuntimeRef  string       `yaml:"runtime_ref,omitempty" json:"runtime_ref,omitempty"`
	SuiteRef    string       `yaml:"suite_ref,omitempty"  json:"suite_ref,omitempty"`
	MemoryRef   string       `yaml:"memory_ref,omitempty" json:"memory_ref,omitempty"`
	Strategy    string       `yaml:"strategy,omitempty"   json:"strategy,omitempty"`
	AdpRef      string       `yaml:"adp_ref,omitempty"    json:"adp_ref,omitempty"`
	Label       string       `yaml:"label,omitempty"     json:"label,omitempty"`
	BodyNodes   []string     `yaml:"body_nodes,omitempty" json:"body_nodes,omitempty"`
	Termination *Termination `yaml:"termination,omitempty" json:"termination,omitempty"`
	Params      interface{}  `yaml:"params,omitempty"    json:"params,omitempty"`
	Extensions  interface{}  `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// Termination defines loop termination conditions
type Termination struct {
	MaxIterations *int        `yaml:"max_iterations,omitempty"  json:"max_iterations,omitempty"`
	OnMaxExceeded string      `yaml:"on_max_exceeded,omitempty" json:"on_max_exceeded,omitempty"`
	TimeoutMs     *int64      `yaml:"timeout_ms,omitempty"     json:"timeout_ms,omitempty"`
	Extensions    interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// Edge defines an edge in the flow graph
type Edge struct {
	From       string      `yaml:"from"               json:"from"`
	To         string      `yaml:"to"                 json:"to"`
	Condition  string      `yaml:"condition,omitempty" json:"condition,omitempty"`
	Extensions interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// Graph defines the flow graph
type Graph struct {
	Nodes      []Node      `yaml:"nodes"               json:"nodes"`
	Edges      []Edge      `yaml:"edges"               json:"edges"`
	StartNodes []string    `yaml:"start_nodes,omitempty" json:"start_nodes,omitempty"`
	EndNodes   []string    `yaml:"end_nodes,omitempty"   json:"end_nodes,omitempty"`
	Extensions interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// Flow defines the flow configuration
type Flow struct {
	ID         string      `yaml:"id"                  json:"id"`
	Graph      Graph       `yaml:"graph"               json:"graph"`
	LoopPolicy *LoopPolicy `yaml:"loop_policy,omitempty" json:"loop_policy,omitempty"`
	Extensions interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// AuthScheme defines authentication schemes
type AuthScheme string

const (
	AuthSchemeBearer AuthScheme = "bearer"
	AuthSchemeAPIKey AuthScheme = "api_key"
	AuthSchemeOAuth2 AuthScheme = "oauth2"
	AuthSchemeMTLS   AuthScheme = "mtls"
	AuthSchemeNone   AuthScheme = "none"
)

// Auth defines authentication configuration
type Auth struct {
	Scheme     AuthScheme  `yaml:"scheme,omitempty"    json:"scheme,omitempty"`
	EnvVar     string      `yaml:"env_var,omitempty"  json:"env_var,omitempty"`
	Header     string      `yaml:"header,omitempty"  json:"header,omitempty"`
	APIKey     string      `yaml:"api_key,omitempty"  json:"api_key,omitempty"`
	Extensions interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// BackoffType defines retry backoff strategies
type BackoffType string

const (
	BackoffTypeFixed       BackoffType = "fixed"
	BackoffTypeLinear      BackoffType = "linear"
	BackoffTypeExponential BackoffType = "exponential"
)

// RetryPolicy defines retry configuration
type RetryPolicy struct {
	MaxRetries           *int        `yaml:"max_retries,omitempty" json:"max_retries,omitempty"`
	BaseDelayMs          *int64      `yaml:"base_delay_ms,omitempty" json:"base_delay_ms,omitempty"`
	MaxDelayMs           *int64      `yaml:"max_delay_ms,omitempty" json:"max_delay_ms,omitempty"`
	BackoffType          BackoffType `yaml:"backoff_type,omitempty" json:"backoff_type,omitempty"`
	RetryableStatusCodes []int       `yaml:"retryable_status_codes,omitempty" json:"retryable_status_codes,omitempty"`
	Extensions           interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// RateLimitOnLimitExceeded defines rate limit behavior
type RateLimitOnLimitExceeded string

const (
	RateLimitOnLimitExceededQueue RateLimitOnLimitExceeded = "queue"
	RateLimitOnLimitExceededFail  RateLimitOnLimitExceeded = "fail"
	RateLimitOnLimitExceededWarn  RateLimitOnLimitExceeded = "warn"
)

// RateLimitPolicy defines rate limiting configuration
type RateLimitPolicy struct {
	MaxRequests     *int                     `yaml:"max_requests,omitempty" json:"max_requests,omitempty"`
	PerSeconds      *int64                   `yaml:"per_seconds,omitempty" json:"per_seconds,omitempty"`
	OnLimitExceeded RateLimitOnLimitExceeded `yaml:"on_limit_exceeded,omitempty" json:"on_limit_exceeded,omitempty"`
	Extensions      interface{}              `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// CacheScope defines cache scope
type CacheScope string

const (
	CacheScopeAgent   CacheScope = "agent"
	CacheScopeSession CacheScope = "session"
	CacheScopeUser    CacheScope = "user"
)

// CachePolicy defines caching configuration
type CachePolicy struct {
	Enabled    *bool       `yaml:"enabled,omitempty" json:"enabled,omitempty"`
	TTLSeconds *int64      `yaml:"ttl_seconds,omitempty" json:"ttl_seconds,omitempty"`
	Scope      CacheScope  `yaml:"scope,omitempty" json:"scope,omitempty"`
	KeyFields  []string    `yaml:"key_fields,omitempty" json:"key_fields,omitempty"`
	Extensions interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// LoadStrategy defines tool loading strategies
type LoadStrategy string

const (
	LoadStrategyEager    LoadStrategy = "eager"
	LoadStrategyLazy     LoadStrategy = "lazy"
	LoadStrategyOnDemand LoadStrategy = "on_demand"
)

// ToolPolicy defines tool execution policies
type ToolPolicy struct {
	LoadStrategy LoadStrategy     `yaml:"load_strategy,omitempty" json:"load_strategy,omitempty"`
	Retry        *RetryPolicy     `yaml:"retry,omitempty" json:"retry,omitempty"`
	RateLimit    *RateLimitPolicy `yaml:"rate_limit,omitempty" json:"rate_limit,omitempty"`
	Cache        *CachePolicy     `yaml:"cache,omitempty" json:"cache,omitempty"`
	Extensions   interface{}      `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// MCPServer defines an MCP server tool
type MCPServer struct {
	ID             string      `yaml:"id"                json:"id"`
	Name           string      `yaml:"name,omitempty"    json:"name,omitempty"`
	Description    string      `yaml:"description,omitempty" json:"description,omitempty"`
	Command        string      `yaml:"command"           json:"command"`
	Args           []string    `yaml:"args,omitempty"    json:"args,omitempty"`
	Env            interface{} `yaml:"env,omitempty"     json:"env,omitempty"`
	TimeoutSeconds *int64      `yaml:"timeout_seconds,omitempty" json:"timeout_seconds,omitempty"`
	Auth           *Auth       `yaml:"auth,omitempty"    json:"auth,omitempty"`
	Policy         *ToolPolicy `yaml:"policy,omitempty"  json:"policy,omitempty"`
	Extensions     interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// HTTPAPI defines an HTTP API tool
type HTTPAPI struct {
	ID          string      `yaml:"id"               json:"id"`
	Name        string      `yaml:"name,omitempty"   json:"name,omitempty"`
	Description string      `yaml:"description,omitempty" json:"description,omitempty"`
	BaseURL     string      `yaml:"base_url"          json:"base_url"`
	Path        string      `yaml:"path,omitempty"    json:"path,omitempty"`
	Method      string      `yaml:"method,omitempty"  json:"method,omitempty"`
	Headers     interface{} `yaml:"headers,omitempty" json:"headers,omitempty"`
	Auth        *Auth       `yaml:"auth,omitempty"    json:"auth,omitempty"`
	Policy      *ToolPolicy `yaml:"policy,omitempty"  json:"policy,omitempty"`
	Extensions  interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// SQLFunction defines a SQL function tool
type SQLFunction struct {
	ID          string      `yaml:"id"               json:"id"`
	Name        string      `yaml:"name,omitempty"   json:"name,omitempty"`
	Description string      `yaml:"description,omitempty" json:"description,omitempty"`
	Query       string      `yaml:"query"            json:"query"`
	DBURLEnv    string      `yaml:"db_url_env,omitempty" json:"db_url_env,omitempty"`
	DBSchema    string      `yaml:"db_schema,omitempty" json:"db_schema,omitempty"`
	Auth        *Auth       `yaml:"auth,omitempty"    json:"auth,omitempty"`
	Policy      *ToolPolicy `yaml:"policy,omitempty"  json:"policy,omitempty"`
	Extensions  interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// Tools defines the tools configuration
type Tools struct {
	MCPservers   []MCPServer   `yaml:"mcp_servers,omitempty"  json:"mcp_servers,omitempty"`
	HTTPAPIs     []HTTPAPI     `yaml:"http_apis,omitempty"   json:"http_apis,omitempty"`
	SQLFunctions []SQLFunction `yaml:"sql_functions,omitempty" json:"sql_functions,omitempty"`
	Policy       *ToolPolicy   `yaml:"policy,omitempty"       json:"policy,omitempty"`
	Extensions   interface{}   `yaml:"extensions,omitempty"   json:"extensions,omitempty"`
}

// MemoryStoreType defines memory store types
type MemoryStoreType string

const (
	MemoryStoreTypeEpisodic MemoryStoreType = "episodic"
	MemoryStoreTypeSemantic MemoryStoreType = "semantic"
)

// MemoryStoreScope defines memory store scope
type MemoryStoreScope string

const (
	MemoryStoreScopeAgent   MemoryStoreScope = "agent"
	MemoryStoreScopeSession MemoryStoreScope = "session"
	MemoryStoreScopeUser    MemoryStoreScope = "user"
)

// PiiPolicy defines PII handling policies
type PiiPolicy string

const (
	PiiPolicyRedact  PiiPolicy = "redact"
	PiiPolicyEncrypt PiiPolicy = "encrypt"
	PiiPolicyBlock   PiiPolicy = "block"
	PiiPolicyLog     PiiPolicy = "log"
)

// AutoClearOn defines when to auto-clear memory
type AutoClearOn string

const (
	AutoClearOnSessionEnd AutoClearOn = "session_end"
	AutoClearOnAgentStop  AutoClearOn = "agent_stop"
	AutoClearOnNever      AutoClearOn = "never"
)

// MemoryStore defines a memory store
type MemoryStore struct {
	ID         string           `yaml:"id"                  json:"id"`
	StoreType  MemoryStoreType  `yaml:"type"               json:"type"`
	Provider   string           `yaml:"provider,omitempty"  json:"provider,omitempty"`
	Endpoint   string           `yaml:"endpoint,omitempty"  json:"endpoint,omitempty"`
	Index      string           `yaml:"index,omitempty"     json:"index,omitempty"`
	Scope      MemoryStoreScope `yaml:"scope,omitempty"    json:"scope,omitempty"`
	PiiPolicy  PiiPolicy        `yaml:"pii_policy,omitempty" json:"pii_policy,omitempty"`
	AutoClear  AutoClearOn      `yaml:"auto_clear,omitempty" json:"auto_clear,omitempty"`
	Extensions interface{}      `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// MemoryWorkingStrategy defines working memory strategies
type MemoryWorkingStrategy string

const (
	MemoryWorkingStrategySlidingWindow MemoryWorkingStrategy = "sliding_window"
	MemoryWorkingStrategyFull          MemoryWorkingStrategy = "full"
	MemoryWorkingStrategySummary       MemoryWorkingStrategy = "summary"
)

// MemoryWorking defines working memory configuration
type MemoryWorking struct {
	Strategy                  MemoryWorkingStrategy `yaml:"strategy,omitempty" json:"strategy,omitempty"`
	WindowSize                *int                  `yaml:"window_size,omitempty" json:"window_size,omitempty"`
	MaxTokens                 *int                  `yaml:"max_tokens,omitempty" json:"max_tokens,omitempty"`
	SummaryModelRef           string                `yaml:"summary_model_ref,omitempty" json:"summary_model_ref,omitempty"`
	CompactionThresholdTokens *int                  `yaml:"compaction_threshold_tokens,omitempty" json:"compaction_threshold_tokens,omitempty"`
	Extensions                interface{}           `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// ContextAssemblySource defines the source type for context assembly
type ContextAssemblySource string

const (
	ContextAssemblySourceWorking ContextAssemblySource = "working"
	ContextAssemblySourceStore   ContextAssemblySource = "store"
)

// ContextAssemblyPosition defines context assembly position
type ContextAssemblyPosition string

const (
	ContextAssemblyPositionPrepend ContextAssemblyPosition = "prepend"
	ContextAssemblyPositionAppend  ContextAssemblyPosition = "append"
)

// ContextAssemblyOrderItem defines an item in context_assembly.order
type ContextAssemblyOrderItem struct {
	Source             ContextAssemblySource `yaml:"source" json:"source"`
	StoreRef           string                `yaml:"store_ref,omitempty" json:"store_ref,omitempty"`
	TopK               *int                  `yaml:"top_k,omitempty" json:"top_k,omitempty"`
	RelevanceThreshold *float64              `yaml:"relevance_threshold,omitempty" json:"relevance_threshold,omitempty"`
}

// ContextAssembly defines context assembly configuration
type ContextAssembly struct {
	ApplyToNodeKinds []string                   `yaml:"apply_to_node_kinds,omitempty" json:"apply_to_node_kinds,omitempty"`
	Order            []ContextAssemblyOrderItem `yaml:"order,omitempty" json:"order,omitempty"`
	MaxTotalTokens   *int                       `yaml:"max_total_tokens,omitempty" json:"max_total_tokens,omitempty"`
	StaticInjection  []StaticInjection          `yaml:"static_injection,omitempty" json:"static_injection,omitempty"`
	StoreRef         string                     `yaml:"store_ref,omitempty" json:"store_ref,omitempty"`
	MaxTokens        *int                       `yaml:"max_tokens,omitempty" json:"max_tokens,omitempty"`
	Position         ContextAssemblyPosition    `yaml:"position,omitempty" json:"position,omitempty"`
	Extensions       interface{}                `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// MemoryOperationOnEvent defines when memory operations trigger
type MemoryOperationOnEvent string

const (
	MemoryOperationOnEventOnInvokeEnd MemoryOperationOnEvent = "on_invoke_end"
)

// MemoryOperationOp defines memory operation types
type MemoryOperationOp string

const (
	MemoryOperationOpWrite     MemoryOperationOp = "write"
	MemoryOperationOpClear     MemoryOperationOp = "clear"
	MemoryOperationOpSummarize MemoryOperationOp = "summarize"
)

// MemoryOperation defines a memory operation
type MemoryOperation struct {
	ID         string                 `yaml:"id"                  json:"id"`
	OnEvent    MemoryOperationOnEvent `yaml:"on_event"            json:"on_event"`
	Op         MemoryOperationOp      `yaml:"op"                    json:"op"`
	StoreRef   string                 `yaml:"store_ref,omitempty" json:"store_ref,omitempty"`
	StoreID    string                 `yaml:"store_id,omitempty"  json:"store_id,omitempty"`
	Filter     interface{}            `yaml:"filter,omitempty"    json:"filter,omitempty"`
	Extensions interface{}            `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// MemoryRetention defines memory retention configuration
type MemoryRetention struct {
	TTLSeconds *uint64     `yaml:"ttl_seconds,omitempty" json:"ttl_seconds,omitempty"`
	MaxEntries *int        `yaml:"max_entries,omitempty" json:"max_entries,omitempty"`
	Extensions interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// StaticInjection defines static content injection
type StaticInjection struct {
	ID          string      `yaml:"id"                  json:"id"`
	Source      string      `yaml:"source,omitempty"   json:"source,omitempty"`
	Path        string      `yaml:"path,omitempty"     json:"path,omitempty"`
	Content     string      `yaml:"content,omitempty"   json:"content,omitempty"`
	Position    string      `yaml:"position,omitempty"  json:"position,omitempty"`
	MaxTokens   *int        `yaml:"max_tokens,omitempty" json:"max_tokens,omitempty"`
	Workspace   string      `yaml:"workspace,omitempty" json:"workspace,omitempty"`
	ContentType string      `yaml:"content_type,omitempty" json:"content_type,omitempty"`
	Extensions  interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// MemoryLegacy defines legacy memory configuration
type MemoryLegacy struct {
	MemoryType string      `yaml:"type"                 json:"type"`
	Strategy   string      `yaml:"strategy,omitempty"   json:"strategy,omitempty"`
	MaxHistory *int        `yaml:"max_history,omitempty" json:"max_history,omitempty"`
	Extensions interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// MemoryStructured defines structured memory configuration
type MemoryStructured struct {
	Stores          []MemoryStore     `yaml:"stores,omitempty" json:"stores,omitempty"`
	Working         *MemoryWorking    `yaml:"working,omitempty" json:"working,omitempty"`
	ContextAssembly *ContextAssembly  `yaml:"context_assembly,omitempty" json:"context_assembly,omitempty"`
	Operations      []MemoryOperation `yaml:"operations,omitempty" json:"operations,omitempty"`
	Retention       *MemoryRetention  `yaml:"retention,omitempty" json:"retention,omitempty"`
	StaticInjection []StaticInjection `yaml:"static_injection,omitempty" json:"static_injection,omitempty"`
	Extensions      interface{}       `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// Memory defines memory configuration (union type)
type Memory struct {
	Legacy     *MemoryLegacy     `yaml:"legacy,omitempty" json:"legacy,omitempty"`
	Structured *MemoryStructured `yaml:"structured,omitempty" json:"structured,omitempty"`
	Extensions interface{}       `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// WorkspaceCleanupOn defines when to clean up workspace
type WorkspaceCleanupOn string

const (
	WorkspaceCleanupOnDelete   WorkspaceCleanupOn = "delete"
	WorkspaceCleanupOnArchive  WorkspaceCleanupOn = "archive"
	WorkspaceCleanupOnPreserve WorkspaceCleanupOn = "preserve"
)

// WorkspaceGit defines Git configuration for workspace
type WorkspaceGit struct {
	Enabled    *bool       `yaml:"enabled,omitempty" json:"enabled,omitempty"`
	RepoURL    string      `yaml:"repo_url,omitempty" json:"repo_url,omitempty"`
	Branch     string      `yaml:"branch,omitempty" json:"branch,omitempty"`
	Commit     string      `yaml:"commit,omitempty" json:"commit,omitempty"`
	AutoCommit *bool       `yaml:"auto_commit,omitempty" json:"auto_commit,omitempty"`
	Extensions interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// WorkspacePermissions defines workspace permissions
type WorkspacePermissions struct {
	Read       []string    `yaml:"read,omitempty" json:"read,omitempty"`
	Write      []string    `yaml:"write,omitempty" json:"write,omitempty"`
	Execute    []string    `yaml:"execute,omitempty" json:"execute,omitempty"`
	Extensions interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// WorkspaceMountSource defines the source of a workspace mount
type WorkspaceMountSource struct {
	Workspace  string      `yaml:"workspace,omitempty" json:"workspace,omitempty"`
	Path       string      `yaml:"path,omitempty" json:"path,omitempty"`
	MountID    string      `yaml:"mount_id,omitempty" json:"mount_id,omitempty"`
	Extensions interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// WorkspaceMount defines a workspace mount
type WorkspaceMount struct {
	ID         string               `yaml:"id" json:"id"`
	Source     WorkspaceMountSource `yaml:"source" json:"source"`
	Target     string               `yaml:"target" json:"target"`
	ReadOnly   *bool                `yaml:"read_only,omitempty" json:"read_only,omitempty"`
	Extensions interface{}          `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// WorkspaceCleanup defines workspace cleanup configuration
type WorkspaceCleanup struct {
	OnAgentStop    WorkspaceCleanupOn `yaml:"on_agent_stop,omitempty" json:"on_agent_stop,omitempty"`
	OnSessionEnd   WorkspaceCleanupOn `yaml:"on_session_end,omitempty" json:"on_session_end,omitempty"`
	RetainPatterns []string           `yaml:"retain_patterns,omitempty" json:"retain_patterns,omitempty"`
	Extensions     interface{}        `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// Workspace defines workspace configuration
type Workspace struct {
	Root        string                `yaml:"root,omitempty" json:"root,omitempty"`
	RootEnvVar  string                `yaml:"root_env_var,omitempty" json:"root_env_var,omitempty"`
	Git         *WorkspaceGit         `yaml:"git,omitempty" json:"git,omitempty"`
	Permissions *WorkspacePermissions `yaml:"permissions,omitempty" json:"permissions,omitempty"`
	Mounts      []WorkspaceMount      `yaml:"mounts,omitempty" json:"mounts,omitempty"`
	Cleanup     *WorkspaceCleanup     `yaml:"cleanup,omitempty" json:"cleanup,omitempty"`
	Extensions  interface{}           `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// SandboxRuntime defines sandbox runtime
type SandboxRuntime string

const (
	SandboxRuntimePython  SandboxRuntime = "python"
	SandboxRuntimeNode    SandboxRuntime = "node"
	SandboxRuntimeBash    SandboxRuntime = "bash"
	SandboxRuntimeBrowser SandboxRuntime = "browser"
	SandboxRuntimeCustom  SandboxRuntime = "custom"
)

// SandboxProvider defines sandbox provider
type SandboxProvider string

const (
	SandboxProviderDocker     SandboxProvider = "docker"
	SandboxProviderE2B        SandboxProvider = "e2b"
	SandboxProviderModal      SandboxProvider = "modal"
	SandboxProviderDaytona    SandboxProvider = "daytona"
	SandboxProviderVercel     SandboxProvider = "vercel"
	SandboxProviderCloudflare SandboxProvider = "cloudflare"
	SandboxProviderRunloop    SandboxProvider = "runloop"
	SandboxProviderBlaxel     SandboxProvider = "blaxel"
	SandboxProviderCustom     SandboxProvider = "custom"
)

// SandboxNetwork defines sandbox network mode
type SandboxNetwork string

const (
	SandboxNetworkNone   SandboxNetwork = "none"
	SandboxNetworkHost   SandboxNetwork = "host"
	SandboxNetworkBridge SandboxNetwork = "bridge"
)

// SandboxMountSource defines the source of a sandbox mount
type SandboxMountSource struct {
	Workspace  string      `yaml:"workspace,omitempty" json:"workspace,omitempty"`
	Path       string      `yaml:"path,omitempty" json:"path,omitempty"`
	URL        string      `yaml:"url,omitempty" json:"url,omitempty"`
	Extensions interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// SandboxMount defines a sandbox mount
type SandboxMount struct {
	ID         string             `yaml:"id" json:"id"`
	Source     SandboxMountSource `yaml:"source" json:"source"`
	Target     string             `yaml:"target" json:"target"`
	ReadOnly   *bool              `yaml:"read_only,omitempty" json:"read_only,omitempty"`
	Extensions interface{}        `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// SandboxSnapshot defines sandbox snapshot configuration
type SandboxSnapshot struct {
	Enabled         *bool       `yaml:"enabled,omitempty" json:"enabled,omitempty"`
	Provider        string      `yaml:"provider,omitempty" json:"provider,omitempty"`
	IntervalSeconds *int64      `yaml:"interval_seconds,omitempty" json:"interval_seconds,omitempty"`
	RetentionCount  *int        `yaml:"retention_count,omitempty" json:"retention_count,omitempty"`
	Extensions      interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// SandboxPolicy defines sandbox execution policy
type SandboxPolicy struct {
	TimeoutMs    *int64         `yaml:"timeout_ms,omitempty" json:"timeout_ms,omitempty"`
	MaxProcesses *int           `yaml:"max_processes,omitempty" json:"max_processes,omitempty"`
	Network      SandboxNetwork `yaml:"network,omitempty" json:"network,omitempty"`
	AllowedHosts []string       `yaml:"allowed_hosts,omitempty" json:"allowed_hosts,omitempty"`
	AllowedPorts []int          `yaml:"allowed_ports,omitempty" json:"allowed_ports,omitempty"`
	Extensions   interface{}    `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// Sandbox defines sandbox configuration
type Sandbox struct {
	Runtime    SandboxRuntime   `yaml:"runtime" json:"runtime"`
	Provider   SandboxProvider  `yaml:"provider,omitempty" json:"provider,omitempty"`
	Image      string           `yaml:"image,omitempty" json:"image,omitempty"`
	Mounts     []SandboxMount   `yaml:"mounts,omitempty" json:"mounts,omitempty"`
	Snapshot   *SandboxSnapshot `yaml:"snapshot,omitempty" json:"snapshot,omitempty"`
	Policy     *SandboxPolicy   `yaml:"policy,omitempty" json:"policy,omitempty"`
	Extensions interface{}      `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// ArtifactProvider defines artifact storage provider
type ArtifactProvider string

const (
	ArtifactProviderGCS       ArtifactProvider = "gcs"
	ArtifactProviderS3        ArtifactProvider = "s3"
	ArtifactProviderAzureBlob ArtifactProvider = "azure_blob"
	ArtifactProviderInMemory  ArtifactProvider = "inmemory"
	ArtifactProviderLocal     ArtifactProvider = "local"
)

// ArtifactScope defines artifact scope
type ArtifactScope string

const (
	ArtifactScopeSession ArtifactScope = "session"
	ArtifactScopeUser    ArtifactScope = "user"
	ArtifactScopeAgent   ArtifactScope = "agent"
)

// ArtifactStore defines an artifact store
type ArtifactStore struct {
	ID         string           `yaml:"id" json:"id"`
	Provider   ArtifactProvider `yaml:"provider" json:"provider"`
	Bucket     string           `yaml:"bucket,omitempty" json:"bucket,omitempty"`
	Prefix     string           `yaml:"prefix,omitempty" json:"prefix,omitempty"`
	Scope      ArtifactScope    `yaml:"scope,omitempty" json:"scope,omitempty"`
	Extensions interface{}      `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// Artifacts defines artifacts configuration
type Artifacts struct {
	Stores     []ArtifactStore `yaml:"stores,omitempty" json:"stores,omitempty"`
	Extensions interface{}     `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// TracingBackend defines tracing backend
type TracingBackend string

const (
	TracingBackendOTLP      TracingBackend = "otlp"
	TracingBackendLangfuse  TracingBackend = "langfuse"
	TracingBackendLangsmith TracingBackend = "langsmith"
	TracingBackendArize     TracingBackend = "arize"
	TracingBackendPhoenix   TracingBackend = "phoenix"
	TracingBackendStdout    TracingBackend = "stdout"
	TracingBackendNone      TracingBackend = "none"
)

// TraceEvent defines tracing event types
type TraceEvent string

const (
	TraceEventModelRequest  TraceEvent = "model_request"
	TraceEventToolCall      TraceEvent = "tool_call"
	TraceEventFlowNode      TraceEvent = "flow_node"
	TraceEventLoopIteration TraceEvent = "loop_iteration"
	TraceEventInterrupt     TraceEvent = "interrupt"
	TraceEventCostCheck     TraceEvent = "cost_check"
	TraceEventArtifactWrite TraceEvent = "artifact_write"
)

// Tracing defines tracing configuration
type Tracing struct {
	Backend      TracingBackend `yaml:"backend,omitempty" json:"backend,omitempty"`
	TraceEvents  []TraceEvent   `yaml:"trace_events,omitempty" json:"trace_events,omitempty"`
	SamplingRate *float64       `yaml:"sampling_rate,omitempty" json:"sampling_rate,omitempty"`
	Extensions   interface{}    `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// CostReporting defines cost reporting configuration
type CostReporting struct {
	Enabled     *bool       `yaml:"enabled,omitempty" json:"enabled,omitempty"`
	Granularity string      `yaml:"granularity,omitempty" json:"granularity,omitempty"`
	ModelRefs   []string    `yaml:"model_refs,omitempty" json:"model_refs,omitempty"`
	Extensions  interface{} `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// Observability defines observability configuration
type Observability struct {
	Tracing       *Tracing       `yaml:"tracing,omitempty" json:"tracing,omitempty"`
	CostReporting *CostReporting `yaml:"cost_reporting,omitempty" json:"cost_reporting,omitempty"`
	Extensions    interface{}    `yaml:"extensions,omitempty" json:"extensions,omitempty"`
}

// Telemetry defines observability / OTEL export configuration.
type Telemetry struct {
	Endpoint           string   `yaml:"endpoint,omitempty"            json:"endpoint,omitempty"`
	Protocol           string   `yaml:"protocol,omitempty"            json:"protocol,omitempty"`
	ServiceName        string   `yaml:"service_name,omitempty"        json:"service_name,omitempty"`
	SamplingRate       float64  `yaml:"sampling_rate,omitempty"       json:"sampling_rate,omitempty"`
	RequiredAttributes []string `yaml:"required_attributes,omitempty" json:"required_attributes,omitempty"`
}

// ImportEntry describes a module import from another ADP manifest.
type ImportEntry struct {
	ID       string   `yaml:"id"                 json:"id"`
	From     string   `yaml:"from"               json:"from"`
	Sections []string `yaml:"sections,omitempty" json:"sections,omitempty"`
}

// OverrideEntry describes a targeted override using a JSON-Pointer–style path.
type OverrideEntry struct {
	Path  string      `yaml:"path"            json:"path"`
	Value interface{} `yaml:"value,omitempty" json:"value,omitempty"`
	Op    string      `yaml:"op,omitempty"    json:"op,omitempty"`
}

// InteropAgentSpecLLMBinding maps an ADP runtime backend to an AgentSpec LLM config component.
type InteropAgentSpecLLMBinding struct {
	BackendID     string `yaml:"backend_id"               json:"backend_id"`
	AgentSpecID   string `yaml:"agentspec_id"             json:"agentspec_id"`
	AgentSpecType string `yaml:"agentspec_type,omitempty" json:"agentspec_type,omitempty"`
}

// InteropAgentSpec declares this manifest's relationship to an AgentSpec configuration.
type InteropAgentSpec struct {
	Ref             string                       `yaml:"ref,omitempty"              json:"ref,omitempty"`
	Version         string                       `yaml:"version,omitempty"          json:"version,omitempty"`
	ComponentType   string                       `yaml:"component_type,omitempty"   json:"component_type,omitempty"`
	ComponentID     string                       `yaml:"component_id,omitempty"     json:"component_id,omitempty"`
	RuntimeAdapters []string                     `yaml:"runtime_adapters,omitempty" json:"runtime_adapters,omitempty"`
	NodeMap         map[string]string            `yaml:"node_map,omitempty"         json:"node_map,omitempty"`
	LLMMap          []InteropAgentSpecLLMBinding `yaml:"llm_map,omitempty"          json:"llm_map,omitempty"`
}

// Interop holds interoperability mappings (A2A, AgentSpec, etc.).
type Interop struct {
	A2A       map[string]interface{} `yaml:"a2a,omitempty"        json:"a2a,omitempty"`
	AgentSpec *InteropAgentSpec      `yaml:"agentspec,omitempty"  json:"agentspec,omitempty"`
}

type ADP struct {
	ADPVersion       string          `yaml:"adp_version"              json:"adp_version"`
	ID               string          `yaml:"id"                       json:"id"`
	Name             string          `yaml:"name,omitempty"           json:"name,omitempty"`
	Description      string          `yaml:"description,omitempty"    json:"description,omitempty"`
	Owner            string          `yaml:"owner,omitempty"          json:"owner,omitempty"`
	Tags             []string        `yaml:"tags,omitempty"           json:"tags,omitempty"`
	ConformanceClass string          `yaml:"conformance_class,omitempty" json:"conformance_class,omitempty"`
	Runtime          Runtime         `yaml:"runtime"                  json:"runtime"`
	Flow             *Flow           `yaml:"flow"                     json:"flow"`
	Evaluation       interface{}     `yaml:"evaluation"               json:"evaluation"`
	Extends          string          `yaml:"extends,omitempty"        json:"extends,omitempty"`
	Imports          []ImportEntry   `yaml:"import,omitempty"         json:"import,omitempty"`
	Overrides        []OverrideEntry        `yaml:"overrides,omitempty"      json:"overrides,omitempty"`
	Guardrails       *Guardrails     `yaml:"guardrails,omitempty"     json:"guardrails,omitempty"`
	Telemetry        *Telemetry      `yaml:"telemetry,omitempty"      json:"telemetry,omitempty"`
	Tools            *Tools          `yaml:"tools,omitempty"          json:"tools,omitempty"`
	// v0.3.0 fields
	Subagents []Subagent             `yaml:"subagents,omitempty"      json:"subagents,omitempty"`
	Hooks     interface{}            `yaml:"hooks,omitempty"          json:"hooks,omitempty"`
	Pipeline  interface{}            `yaml:"pipeline,omitempty"       json:"pipeline,omitempty"`
	Streaming interface{}            `yaml:"streaming,omitempty"      json:"streaming,omitempty"`
	XTesting  map[string]interface{} `yaml:"x_testing,omitempty"      json:"x_testing,omitempty"`
	// Additional v0.3.0 top-level fields
	Memory        *Memory        `yaml:"memory,omitempty"        json:"memory,omitempty"`
	Workspace     *Workspace     `yaml:"workspace,omitempty"     json:"workspace,omitempty"`
	Sandbox       *Sandbox       `yaml:"sandbox,omitempty"       json:"sandbox,omitempty"`
	Artifacts     *Artifacts     `yaml:"artifacts,omitempty"     json:"artifacts,omitempty"`
	Observability *Observability `yaml:"observability,omitempty"  json:"observability,omitempty"`
	Interop       *Interop       `yaml:"interop,omitempty"       json:"interop,omitempty"`
}

func LoadADP(path string) (*ADP, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var adp ADP
	if err := yaml.Unmarshal(data, &adp); err != nil {
		return nil, err
	}
	return &adp, nil
}
