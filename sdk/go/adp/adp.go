package adp

import (
    "os"
    "gopkg.in/yaml.v3"
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
    ID               string                 `yaml:"id"                        json:"id"`
    Provider         string                 `yaml:"provider"                  json:"provider"`
    Model            string                 `yaml:"model"                     json:"model"`
    APIKeyEnv        string                 `yaml:"api_key_env,omitempty"     json:"api_key_env,omitempty"`
    BaseURL          string                 `yaml:"base_url,omitempty"        json:"base_url,omitempty"`
    Temperature      *float64               `yaml:"temperature,omitempty"     json:"temperature,omitempty"`
    MaxTokens        *int                   `yaml:"max_tokens,omitempty"      json:"max_tokens,omitempty"`
    Extensions       map[string]interface{} `yaml:"extensions,omitempty"      json:"extensions,omitempty"`
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

type ADP struct {
    ADPVersion       string                 `yaml:"adp_version"              json:"adp_version"`
    ID               string                 `yaml:"id"                       json:"id"`
    Name             string                 `yaml:"name,omitempty"           json:"name,omitempty"`
    Description      string                 `yaml:"description,omitempty"    json:"description,omitempty"`
    Owner            string                 `yaml:"owner,omitempty"          json:"owner,omitempty"`
    Tags             []string               `yaml:"tags,omitempty"           json:"tags,omitempty"`
    ConformanceClass string                 `yaml:"conformance_class,omitempty" json:"conformance_class,omitempty"`
    Runtime          Runtime                `yaml:"runtime"                  json:"runtime"`
    Flow             interface{}            `yaml:"flow"                     json:"flow"`
    Evaluation       interface{}            `yaml:"evaluation"               json:"evaluation"`
    Extends          string                 `yaml:"extends,omitempty"        json:"extends,omitempty"`
    Imports          []ImportEntry          `yaml:"import,omitempty"         json:"import,omitempty"`
    Overrides        []OverrideEntry        `yaml:"overrides,omitempty"      json:"overrides,omitempty"`
    Guardrails       *Guardrails            `yaml:"guardrails,omitempty"     json:"guardrails,omitempty"`
    Telemetry        *Telemetry             `yaml:"telemetry,omitempty"      json:"telemetry,omitempty"`
    Tools            interface{}            `yaml:"tools,omitempty"          json:"tools,omitempty"`
    // v0.3.0 fields
    Subagents        []Subagent             `yaml:"subagents,omitempty"      json:"subagents,omitempty"`
    Hooks            interface{}            `yaml:"hooks,omitempty"          json:"hooks,omitempty"`
    Pipeline         interface{}            `yaml:"pipeline,omitempty"       json:"pipeline,omitempty"`
    Streaming        interface{}            `yaml:"streaming,omitempty"      json:"streaming,omitempty"`
    XTesting         map[string]interface{} `yaml:"x_testing,omitempty"      json:"x_testing,omitempty"`
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
