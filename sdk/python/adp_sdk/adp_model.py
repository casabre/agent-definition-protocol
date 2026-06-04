from __future__ import annotations

from enum import Enum
from pathlib import Path
from typing import Any, Union

import yaml
from pydantic import BaseModel, Field


# =============================================================================
# Enums for v0.3.0
# =============================================================================

class MemoryStoreType(str, Enum):
    episodic = "episodic"
    semantic = "semantic"


class MemoryStoreScope(str, Enum):
    agent = "agent"
    session = "session"
    user = "user"


class MemoryWorkingStrategy(str, Enum):
    sliding_window = "sliding_window"
    full = "full"
    summary = "summary"


class ContextAssemblySource(str, Enum):
    working = "working"
    store = "store"


class ContextAssemblyPosition(str, Enum):
    prepend = "prepend"
    append = "append"


class MemoryOperationOnEvent(str, Enum):
    on_invoke_end = "on_invoke_end"


class MemoryOperationOp(str, Enum):
    write = "write"
    clear = "clear"
    summarize = "summarize"


class PiiPolicy(str, Enum):
    redact = "redact"
    encrypt = "encrypt"
    block = "block"
    log = "log"


class AutoClearOn(str, Enum):
    session_end = "session_end"
    agent_stop = "agent_stop"
    never = "never"


class WorkspaceCleanupOn(str, Enum):
    agent_stop = "agent_stop"
    session_end = "session_end"
    never = "never"


class MountProvider(str, Enum):
    s3 = "s3"
    gcs = "gcs"
    azure_blob = "azure_blob"
    cloudflare_r2 = "cloudflare_r2"


class SandboxRuntime(str, Enum):
    python = "python"
    node = "node"
    bash = "bash"
    browser = "browser"
    custom = "custom"


class SandboxProvider(str, Enum):
    docker = "docker"
    e2b = "e2b"
    modal = "modal"
    daytona = "daytona"
    vercel = "vercel"
    cloudflare = "cloudflare"
    runloop = "runloop"
    blaxel = "blaxel"
    custom = "custom"


class SandboxNetwork(str, Enum):
    none = "none"
    host = "host"
    bridge = "bridge"


class SandboxRestoreOn(str, Enum):
    failure = "failure"
    always = "always"
    never = "never"


class ArtifactScope(str, Enum):
    session = "session"
    user = "user"
    agent = "agent"


class ArtifactProvider(str, Enum):
    gcs = "gcs"
    s3 = "s3"
    azure_blob = "azure_blob"
    inmemory = "inmemory"
    local = "local"


class LoopOnMaxExceeded(str, Enum):
    fail = "fail"
    use_last = "use_last"
    escalate = "escalate"


class NodeKind(str, Enum):
    input = "input"
    output = "output"
    llm = "llm"
    tool = "tool"
    router = "router"
    retriever = "retriever"
    evaluator = "evaluator"
    subflow = "subflow"
    loop = "loop"


class InterruptTrigger(str, Enum):
    tool_call = "tool_call"
    cost_threshold = "cost_threshold"
    loop_max_exceeded = "loop_max_exceeded"
    custom = "custom"


class InterruptMode(str, Enum):
    pause_and_notify = "pause_and_notify"
    block = "block"
    log = "log"


class InterruptExecutionMode(str, Enum):
    blocking = "blocking"
    parallel = "parallel"


class InterruptOnTimeout(str, Enum):
    fail = "fail"
    approve = "approve"
    deny = "deny"


class CostOnThresholdExceeded(str, Enum):
    block = "block"
    warn = "warn"
    interrupt = "interrupt"
    downgrade = "downgrade"


class CostTrackBy(str, Enum):
    invocation = "invocation"
    session = "session"
    user = "user"
    day = "day"


class AgentTrustLevel(str, Enum):
    sandboxed = "sandboxed"
    supervised = "supervised"
    autonomous = "autonomous"


class TracingBackend(str, Enum):
    otlp = "otlp"
    langfuse = "langfuse"
    langsmith = "langsmith"
    arize = "arize"
    phoenix = "phoenix"
    stdout = "stdout"
    none = "none"


class TraceEvent(str, Enum):
    model_request = "model_request"
    tool_call = "tool_call"
    flow_node = "flow_node"
    loop_iteration = "loop_iteration"
    interrupt = "interrupt"
    cost_check = "cost_check"
    artifact_write = "artifact_write"


class LoadStrategy(str, Enum):
    eager = "eager"
    lazy = "lazy"
    on_demand = "on_demand"


class BackoffType(str, Enum):
    fixed = "fixed"
    linear = "linear"
    exponential = "exponential"


class RateLimitOnLimitExceeded(str, Enum):
    queue = "queue"
    fail = "fail"
    warn = "warn"


class CacheScope(str, Enum):
    agent = "agent"
    session = "session"
    user = "user"


class AuthScheme(str, Enum):
    bearer = "bearer"
    api_key = "api_key"
    oauth2 = "oauth2"
    mtls = "mtls"
    none = "none"


# =============================================================================
# Runtime Models (existing, extended)
# =============================================================================


class RuntimeEntry(BaseModel):
    backend: str
    id: str
    entrypoint: str | list[str] | None = None
    image: str | None = None
    module: str | None = None
    env: dict[str, str] = Field(default_factory=dict)


class RuntimeModel(BaseModel):
    model_config = dict(extra="allow")
    execution: list[RuntimeEntry]


# =============================================================================
# Flow Models (extended for v0.3.0)
# =============================================================================


class FlowUI(BaseModel):
    label: str | None = None
    icon: str | None = None
    color: str | None = None
    position: dict[str, float] | None = None


class NodeUI(FlowUI):
    pass


class EdgeModel(BaseModel):
    from_node: str = Field(..., alias="from")
    to: str
    condition: str | None = None


class TerminationModel(BaseModel):
    condition: str | None = None
    max_iterations: int | None = None
    max_tokens: int | None = None
    on_max_exceeded: LoopOnMaxExceeded = LoopOnMaxExceeded.fail
    restart_context: bool = False


class NodeModel(BaseModel):
    model_config = dict(extra="allow")

    id: str
    kind: NodeKind
    label: str | None = None
    model_ref: str | None = None
    system_prompt_ref: str | None = None
    prompt_ref: str | None = None
    tool_ref: str | None = None
    adp_ref: str | None = None
    flow_ref: str | None = None
    memory_ref: str | None = None
    suite_ref: str | None = None
    output_ref: str | None = None
    runtime_ref: str | None = None
    blocking: bool | None = None
    strategy: str | None = None  # for router nodes
    params: dict[str, Any] = Field(default_factory=dict)
    ui: NodeUI | None = None
    # v0.3.0 loop additions
    body_nodes: list[str] | None = None
    termination: TerminationModel | None = None


class LoopPolicyModel(BaseModel):
    default_max_iterations: int | None = None
    on_max_exceeded: LoopOnMaxExceeded = LoopOnMaxExceeded.fail
    total_run_max_iterations: int | None = None


class GraphModel(BaseModel):
    nodes: list[NodeModel] | list[dict[str, Any]]
    edges: list[EdgeModel] | list[dict[str, Any]] = Field(default_factory=list)
    start_nodes: list[str]
    end_nodes: list[str]


class FlowModel(BaseModel):
    model_config = dict(extra="allow")

    id: str | None = None
    graph: GraphModel | dict[str, Any] | None = None
    loop_policy: LoopPolicyModel | None = None
    extensions: dict[str, Any] | None = None


# =============================================================================
# Evaluation Models (existing)
# =============================================================================


class EvaluationModel(BaseModel):
    model_config = dict(extra="allow")


# =============================================================================
# Guardrails Models (extended for v0.3.0)
# =============================================================================


class GuardrailMode(str, Enum):
    block = "block"
    flag = "flag"
    redact = "redact"
    log = "log"


class GuardrailThreshold(str, Enum):
    low = "low"
    medium = "medium"
    high = "high"


class GuardrailRail(BaseModel):
    model_config = dict(extra="allow")

    id: str
    provider: str
    policy_ref: str
    mode: GuardrailMode | None = None
    categories: list[str] | None = None
    threshold: GuardrailThreshold | None = None


class GuardrailsInputOutput(BaseModel):
    rails: list[GuardrailRail] = Field(default_factory=list)


class InterruptNotification(BaseModel):
    channel: str
    endpoint_env_var: str | None = None
    timeout_seconds: int | None = None
    on_timeout: InterruptOnTimeout | None = None


class InterruptModel(BaseModel):
    id: str
    trigger: InterruptTrigger
    tool_refs: list[str] | None = None
    mode: InterruptMode
    execution_mode: InterruptExecutionMode | None = None
    notification: InterruptNotification | None = None


class CostGuardrailModel(BaseModel):
    threshold_usd: float | None = None
    on_threshold_exceeded: CostOnThresholdExceeded | None = None
    interrupt_ref: str | None = None
    downgrade_model_ref: str | None = None
    track_by: CostTrackBy | None = None
    model_refs: list[str] | None = None


class AgentTrustModel(BaseModel):
    level: AgentTrustLevel | None = None
    side_effect_tool_refs: list[str] | None = None


class GuardrailsModel(BaseModel):
    input: list[GuardrailRail] | None = None
    output: list[GuardrailRail] | None = None
    on_violation: str | None = None
    # v0.3.0 additions
    interrupts: list[InterruptModel] | None = None
    cost: CostGuardrailModel | None = None
    agent_trust: AgentTrustModel | None = None


# =============================================================================
# Telemetry Models (existing, renamed to Observability in v0.3.0)
# =============================================================================


class TelemetryModel(BaseModel):
    endpoint: str | None = None
    protocol: str | None = None
    service_name: str | None = None
    sampling_rate: float | None = None
    required_attributes: list[str] = Field(default_factory=list)


# =============================================================================
# Import/Override Models (existing)
# =============================================================================


class ImportEntry(BaseModel):
    model_config = dict(populate_by_name=True)

    id: str
    from_uri: str = Field(..., alias="from")
    sections: list[str] = Field(default_factory=list)


class OverrideEntry(BaseModel):
    path: str
    value: Any = None
    op: str = "set"


# =============================================================================
# Tools Models (extended for v0.3.0)
# =============================================================================


class AuthModel(BaseModel):
    scheme: AuthScheme
    env_var: str | None = None
    scopes: list[str] | None = None


class RetryPolicy(BaseModel):
    max_attempts: int = 3
    backoff: BackoffType = BackoffType.exponential
    backoff_base_ms: int = 500
    max_delay_ms: int = 10000
    retryable_status_codes: list[int] | None = None


class RateLimitPolicy(BaseModel):
    requests_per_minute: int | None = None
    burst: int = 0
    on_limit_exceeded: RateLimitOnLimitExceeded = RateLimitOnLimitExceeded.queue


class CachePolicy(BaseModel):
    enabled: bool = False
    ttl_seconds: int | None = None
    key_fields: list[str] | None = None
    scope: CacheScope | None = None


class ToolPolicy(BaseModel):
    retry: RetryPolicy | None = None
    timeout_ms: int | None = None
    rate_limit: RateLimitPolicy | None = None
    cache: CachePolicy | None = None


class MCPServerModel(BaseModel):
    model_config = dict(extra="allow")

    id: str
    description: str
    transport: str
    endpoint: str
    auth: AuthModel | None = None
    load_strategy: LoadStrategy = LoadStrategy.eager
    policy: ToolPolicy | None = None


class HTTPAPIModel(BaseModel):
    model_config = dict(extra="allow")

    id: str
    description: str
    base_url: str
    auth: AuthModel | None = None
    load_strategy: LoadStrategy = LoadStrategy.eager
    policy: ToolPolicy | None = None


class SQLFunctionModel(BaseModel):
    model_config = dict(extra="allow")

    id: str
    description: str
    connection: str
    db_schema: str | None = Field(default=None, alias="schema")
    auth: AuthModel | None = None
    load_strategy: LoadStrategy = LoadStrategy.eager
    policy: ToolPolicy | None = None


# =============================================================================
# Sandbox Models (v0.3.0)
# =============================================================================


class SandboxMount(BaseModel):
    source: str
    target: str
    read_only: bool = False


class SandboxSnapshot(BaseModel):
    enabled: bool = False
    restore_on: SandboxRestoreOn = SandboxRestoreOn.never


class SandboxPolicy(BaseModel):
    timeout_ms: int
    max_output_bytes: int | None = None
    network: SandboxNetwork = SandboxNetwork.none
    allow_filesystem_writes: bool = False


class SandboxModel(BaseModel):
    model_config = dict(extra="allow")

    id: str
    runtime: SandboxRuntime
    version: str | None = None
    image: str | None = None
    provider: SandboxProvider | None = None
    mounts: list[SandboxMount] | None = None
    env: dict[str, str] = Field(default_factory=dict)
    snapshot: SandboxSnapshot | None = None
    policy: SandboxPolicy


# =============================================================================
# Tools with Sandbox (v0.3.0)
# =============================================================================


class ToolsModel(BaseModel):
    model_config = dict(extra="allow")

    mcp_servers: list[MCPServerModel] | None = None
    http_apis: list[HTTPAPIModel] | None = None
    sql_functions: list[SQLFunctionModel] | None = None
    sandbox: list[SandboxModel] | None = None


# =============================================================================
# Memory Models (v0.3.0)
# =============================================================================


class MemoryLegacy(BaseModel):
    """Legacy v0.1.x memory format for backward compatibility."""

    provider: str | None = None
    endpoint: str | None = None
    index: str | None = None
    namespace: str | None = None


class MemoryStore(BaseModel):
    id: str
    type: MemoryStoreType
    provider: str
    endpoint: str | None = None
    index: str | None = None
    scope: MemoryStoreScope = MemoryStoreScope.agent
    ttl_seconds: int | None = None
    pii: bool = False


class MemoryWorking(BaseModel):
    strategy: MemoryWorkingStrategy = MemoryWorkingStrategy.sliding_window
    window_size: int | None = None
    max_tokens: int | None = None
    summary_model_ref: str | None = None
    compaction_threshold_tokens: int | None = None


class MemoryContextAssemblySource(BaseModel):
    source: ContextAssemblySource
    store_ref: str | None = None
    top_k: int | None = None
    relevance_threshold: float | None = None


class MemoryStaticInjection(BaseModel):
    id: str
    source: str  # "file" or "inline"
    path: str | None = None
    content: str | None = None
    position: ContextAssemblyPosition
    max_tokens: int | None = None


class MemoryContextAssembly(BaseModel):
    apply_to_node_kinds: list[str] | None = None
    order: list[MemoryContextAssemblySource | dict[str, Any]] | None = None
    max_total_tokens: int | None = None
    static_injection: list[MemoryStaticInjection] | None = None


class MemoryOperation(BaseModel):
    on_event: MemoryOperationOnEvent
    op: MemoryOperationOp
    store_ref: str | None = None
    fields: list[str] | None = None
    when: str | None = None


class MemoryRetention(BaseModel):
    pii_policy: PiiPolicy = PiiPolicy.redact
    user_consent_required: bool = False
    data_residency: list[str] | None = None
    auto_clear_on: AutoClearOn = AutoClearOn.never


class MemoryStructured(BaseModel):
    """Structured v0.3.0 memory format."""

    stores: list[MemoryStore] | None = None
    working: MemoryWorking | None = None
    context_assembly: MemoryContextAssembly | None = None
    operations: list[MemoryOperation] | None = None
    retention: MemoryRetention | None = None


# Union type for memory - either legacy or structured
MemoryModel = Union[MemoryLegacy, MemoryStructured]


# =============================================================================
# Workspace Models (v0.3.0)
# =============================================================================


class WorkspaceGit(BaseModel):
    enabled: bool = False
    auto_commit: bool = False
    branch_per_session: bool = False


class WorkspacePermissions(BaseModel):
    read: list[str] = Field(default_factory=lambda: ["**"])
    write: list[str] = Field(default_factory=list)
    exec: list[str] = Field(default_factory=list)


class WorkspaceMount(BaseModel):
    id: str
    provider: MountProvider
    bucket: str | None = None
    prefix: str | None = None
    target: str
    read_only: bool = True
    credentials_env_var: str | None = None


class WorkspaceCleanup(BaseModel):
    on: WorkspaceCleanupOn = WorkspaceCleanupOn.never
    exclude: list[str] | None = None


class WorkspaceModel(BaseModel):
    root: str | None = None
    root_env_var: str | None = None
    git: WorkspaceGit | None = None
    permissions: WorkspacePermissions | None = None
    mounts: list[WorkspaceMount] | None = None
    cleanup: WorkspaceCleanup | None = None


# =============================================================================
# Artifacts Models (v0.3.0)
# =============================================================================


class ArtifactStore(BaseModel):
    id: str
    scope: ArtifactScope
    provider: ArtifactProvider
    bucket: str | None = None
    path_prefix: str | None = None
    ttl_seconds: int | None = None
    versioned: bool = True
    credentials_env_var: str | None = None


class ArtifactsModel(BaseModel):
    stores: list[ArtifactStore]


# =============================================================================
# Observability Models (v0.3.0)
# =============================================================================


class TracingModel(BaseModel):
    backend: TracingBackend | None = None
    endpoint_env_var: str | None = None
    api_key_env_var: str | None = None
    trace_events: list[TraceEvent] | None = None
    sampling_rate: float = 1.0
    service_name: str | None = None


class CostReportingModel(BaseModel):
    enabled: bool = False
    track_by: CostTrackBy | None = None
    emit_metric: str = "gen_ai.cost.usd"
    model_refs: list[str] | None = None


class ObservabilityModel(BaseModel):
    tracing: TracingModel | None = None
    cost_reporting: CostReportingModel | None = None


# =============================================================================
# Interop Models
# =============================================================================


class InteropAgentSpecLLMBinding(BaseModel):
    backend_id: str
    agentspec_id: str
    agentspec_type: str | None = None


class InteropAgentSpec(BaseModel):
    ref: str | None = None
    version: str | None = None
    component_type: str | None = None
    component_id: str | None = None
    runtime_adapters: list[str] | None = None
    node_map: dict[str, str] | None = None
    llm_map: list[InteropAgentSpecLLMBinding] | None = None


class InteropModel(BaseModel):
    a2a: dict[str, Any] | None = None
    agentspec: InteropAgentSpec | None = None

    model_config = dict(extra="allow")


# =============================================================================
# Main ADP Model (extended for v0.3.0)
# =============================================================================


class ADP(BaseModel):
    adp_version: str
    id: str
    runtime: RuntimeModel
    flow: dict[str, Any] | FlowModel = Field(default_factory=dict)
    evaluation: dict[str, Any] | EvaluationModel = Field(default_factory=dict)
    name: str | None = None
    description: str | None = None
    extends: str | None = None
    imports: list[ImportEntry] | None = Field(None, alias="import")
    overrides: list[OverrideEntry] | None = None
    guardrails: GuardrailsModel | None = None
    telemetry: TelemetryModel | None = None

    # v0.3.0 additions
    memory: MemoryModel | None = None
    workspace: WorkspaceModel | None = None
    artifacts: ArtifactsModel | None = None
    observability: ObservabilityModel | None = None
    tools: ToolsModel | None = None
    interop: InteropModel | None = None

    model_config = dict(extra="allow", populate_by_name=True)

    @classmethod
    def from_file(cls, path: str | Path) -> "ADP":
        data = yaml.safe_load(Path(path).read_text())
        return cls.model_validate(data)

    def to_yaml(self, path: str | Path | None = None) -> str:
        text = yaml.safe_dump(
            self.model_dump(by_alias=True, exclude_none=True), sort_keys=False
        )
        if path:
            Path(path).write_text(text)
        return text
