package adp

// Adapter defines the interface for framework adapters
// that convert between ADP manifests and framework-native configurations
type Adapter interface {
	// FrameworkID returns the unique identifier for the framework
	FrameworkID() string

	// Export converts an ADP manifest to framework-native configuration
	Export(adp *ADP) map[string]interface{}

	// ImportFrom converts framework-native configuration to an ADP manifest
	ImportFrom(config map[string]interface{}) *ADP

	// RoundtripFidelity returns coverage per ADP section
	RoundtripFidelity() map[string]string
}

// DefaultRoundtripFidelity returns the default fidelity mapping
func DefaultRoundtripFidelity() map[string]string {
	return map[string]string{
		"flow.graph":            "faithful",
		"tools":                 "faithful",
		"runtime.models":        "faithful",
		"tools.policy":          "lossy",
		"memory.stores":         "lossy",
		"memory.working":        "lossy",
		"loop.termination":      "lossy",
		"guardrails.interrupts": "lossy",
		"workspace":             "unsupported",
		"sandbox":               "unsupported",
		"artifacts":             "unsupported",
		"observability":         "faithful",
	}
}

// AdapterRegistry manages framework adapters
type AdapterRegistry struct {
	adapters map[string]Adapter
}

// NewAdapterRegistry creates a new adapter registry
func NewAdapterRegistry() *AdapterRegistry {
	return &AdapterRegistry{
		adapters: make(map[string]Adapter),
	}
}

// Register adds an adapter to the registry
func (r *AdapterRegistry) Register(adapter Adapter) {
	r.adapters[adapter.FrameworkID()] = adapter
}

// Get retrieves an adapter by framework ID
func (r *AdapterRegistry) Get(frameworkID string) (Adapter, bool) {
	adapter, ok := r.adapters[frameworkID]
	return adapter, ok
}

// Available returns a list of all available framework IDs
func (r *AdapterRegistry) Available() []string {
	ids := make([]string, 0, len(r.adapters))
	for id := range r.adapters {
		ids = append(ids, id)
	}
	return ids
}

// IsAvailable checks if a framework adapter is available
func (r *AdapterRegistry) IsAvailable(frameworkID string) bool {
	_, ok := r.adapters[frameworkID]
	return ok
}

// =============================================================================
// Framework Adapter Stubs
// These are placeholder implementations that will be fully implemented
// in future work. They mirror the Python and TypeScript implementations.
// =============================================================================

// LangGraphAdapter is the adapter for LangGraph framework
type LangGraphAdapter struct{}

func (a *LangGraphAdapter) FrameworkID() string { return "langgraph" }
func (a *LangGraphAdapter) Export(adp *ADP) map[string]interface{} {
	// TODO: Implement proper export
	return map[string]interface{}{}
}
func (a *LangGraphAdapter) ImportFrom(config map[string]interface{}) *ADP {
	// TODO: Implement proper import
	return &ADP{}
}
func (a *LangGraphAdapter) RoundtripFidelity() map[string]string {
	fidelity := DefaultRoundtripFidelity()
	fidelity["flow.graph"] = "faithful"
	fidelity["loop.termination"] = "faithful"
	fidelity["memory.stores"] = "faithful"
	return fidelity
}

// AutoGenAdapter is the adapter for AutoGen framework
type AutoGenAdapter struct{}

func (a *AutoGenAdapter) FrameworkID() string                           { return "autogen" }
func (a *AutoGenAdapter) Export(adp *ADP) map[string]interface{}        { return map[string]interface{}{} }
func (a *AutoGenAdapter) ImportFrom(config map[string]interface{}) *ADP { return &ADP{} }
func (a *AutoGenAdapter) RoundtripFidelity() map[string]string {
	fidelity := DefaultRoundtripFidelity()
	fidelity["loop.termination"] = "faithful"
	return fidelity
}

// CrewAIAdapter is the adapter for CrewAI framework
type CrewAIAdapter struct{}

func (a *CrewAIAdapter) FrameworkID() string                           { return "crewai" }
func (a *CrewAIAdapter) Export(adp *ADP) map[string]interface{}        { return map[string]interface{}{} }
func (a *CrewAIAdapter) ImportFrom(config map[string]interface{}) *ADP { return &ADP{} }
func (a *CrewAIAdapter) RoundtripFidelity() map[string]string {
	fidelity := DefaultRoundtripFidelity()
	fidelity["tools.policy"] = "faithful"
	return fidelity
}

// LlamaIndexAdapter is the adapter for LlamaIndex framework
type LlamaIndexAdapter struct{}

func (a *LlamaIndexAdapter) FrameworkID() string                           { return "llamaindex" }
func (a *LlamaIndexAdapter) Export(adp *ADP) map[string]interface{}        { return map[string]interface{}{} }
func (a *LlamaIndexAdapter) ImportFrom(config map[string]interface{}) *ADP { return &ADP{} }
func (a *LlamaIndexAdapter) RoundtripFidelity() map[string]string {
	fidelity := DefaultRoundtripFidelity()
	fidelity["memory.stores"] = "faithful"
	fidelity["tools"] = "faithful"
	return fidelity
}

// GoogleADKAdapter is the adapter for Google ADK framework
type GoogleADKAdapter struct{}

func (a *GoogleADKAdapter) FrameworkID() string                           { return "google_adk" }
func (a *GoogleADKAdapter) Export(adp *ADP) map[string]interface{}        { return map[string]interface{}{} }
func (a *GoogleADKAdapter) ImportFrom(config map[string]interface{}) *ADP { return &ADP{} }
func (a *GoogleADKAdapter) RoundtripFidelity() map[string]string {
	fidelity := DefaultRoundtripFidelity()
	fidelity["artifacts"] = "faithful"
	fidelity["memory.stores"] = "faithful"
	fidelity["tools"] = "faithful"
	return fidelity
}

// OpenAIAgentsAdapter is the adapter for OpenAI Agents SDK framework
type OpenAIAgentsAdapter struct{}

func (a *OpenAIAgentsAdapter) FrameworkID() string { return "openai_agents" }
func (a *OpenAIAgentsAdapter) Export(adp *ADP) map[string]interface{} {
	return map[string]interface{}{}
}
func (a *OpenAIAgentsAdapter) ImportFrom(config map[string]interface{}) *ADP { return &ADP{} }
func (a *OpenAIAgentsAdapter) RoundtripFidelity() map[string]string {
	fidelity := DefaultRoundtripFidelity()
	fidelity["guardrails.interrupts"] = "faithful"
	fidelity["observability"] = "faithful"
	return fidelity
}

// PydanticAIAdapter is the adapter for Pydantic AI framework
type PydanticAIAdapter struct{}

func (a *PydanticAIAdapter) FrameworkID() string                           { return "pydantic_ai" }
func (a *PydanticAIAdapter) Export(adp *ADP) map[string]interface{}        { return map[string]interface{}{} }
func (a *PydanticAIAdapter) ImportFrom(config map[string]interface{}) *ADP { return &ADP{} }
func (a *PydanticAIAdapter) RoundtripFidelity() map[string]string {
	fidelity := DefaultRoundtripFidelity()
	fidelity["runtime.models"] = "faithful"
	return fidelity
}

// SemanticKernelAdapter is the adapter for Semantic Kernel framework
type SemanticKernelAdapter struct{}

func (a *SemanticKernelAdapter) FrameworkID() string { return "semantic_kernel" }
func (a *SemanticKernelAdapter) Export(adp *ADP) map[string]interface{} {
	return map[string]interface{}{}
}
func (a *SemanticKernelAdapter) ImportFrom(config map[string]interface{}) *ADP { return &ADP{} }
func (a *SemanticKernelAdapter) RoundtripFidelity() map[string]string {
	fidelity := DefaultRoundtripFidelity()
	fidelity["runtime.models"] = "faithful"
	fidelity["tools"] = "faithful"
	return fidelity
}

// RegisterAllAdapters registers all available framework adapters
func RegisterAllAdapters(registry *AdapterRegistry) {
	registry.Register(&LangGraphAdapter{})
	registry.Register(&AutoGenAdapter{})
	registry.Register(&CrewAIAdapter{})
	registry.Register(&LlamaIndexAdapter{})
	registry.Register(&GoogleADKAdapter{})
	registry.Register(&OpenAIAgentsAdapter{})
	registry.Register(&PydanticAIAdapter{})
	registry.Register(&SemanticKernelAdapter{})
}
