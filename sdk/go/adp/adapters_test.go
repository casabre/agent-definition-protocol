package adp

import (
	"sort"
	"testing"
)

// ──── DefaultRoundtripFidelity ───────────────────────────────────────────────

func TestDefaultRoundtripFidelityKeys(t *testing.T) {
	fidelity := DefaultRoundtripFidelity()
	expected := []string{
		"artifacts",
		"flow.graph",
		"guardrails.interrupts",
		"loop.termination",
		"memory.stores",
		"memory.working",
		"observability",
		"runtime.models",
		"sandbox",
		"tools",
		"tools.policy",
		"workspace",
	}
	got := make([]string, 0, len(fidelity))
	for k := range fidelity {
		got = append(got, k)
	}
	sort.Strings(got)
	sort.Strings(expected)
	if len(got) != len(expected) {
		t.Fatalf("expected %d keys, got %d: %v", len(expected), len(got), got)
	}
	for i, k := range expected {
		if got[i] != k {
			t.Errorf("key mismatch at %d: expected %q got %q", i, k, got[i])
		}
	}
}

func TestDefaultRoundtripFidelityValues(t *testing.T) {
	fidelity := DefaultRoundtripFidelity()
	cases := map[string]string{
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
	for key, want := range cases {
		got, ok := fidelity[key]
		if !ok {
			t.Errorf("key %q not found in DefaultRoundtripFidelity", key)
			continue
		}
		if got != want {
			t.Errorf("key %q: expected %q, got %q", key, want, got)
		}
	}
}

// ──── AdapterRegistry ────────────────────────────────────────────────────────

func TestNewAdapterRegistry(t *testing.T) {
	r := NewAdapterRegistry()
	if r == nil {
		t.Fatal("expected non-nil registry")
	}
	if len(r.Available()) != 0 {
		t.Errorf("expected empty registry, got: %v", r.Available())
	}
}

func TestAdapterRegistryRegisterAndGet(t *testing.T) {
	r := NewAdapterRegistry()
	a := &LangGraphAdapter{}
	r.Register(a)

	got, ok := r.Get("langgraph")
	if !ok {
		t.Fatal("expected to find 'langgraph' adapter")
	}
	if got.FrameworkID() != "langgraph" {
		t.Errorf("expected 'langgraph', got %q", got.FrameworkID())
	}
}

func TestAdapterRegistryGetMissing(t *testing.T) {
	r := NewAdapterRegistry()
	_, ok := r.Get("nonexistent")
	if ok {
		t.Error("expected ok=false for nonexistent adapter")
	}
}

func TestAdapterRegistryAvailable(t *testing.T) {
	r := NewAdapterRegistry()
	r.Register(&LangGraphAdapter{})
	r.Register(&AutoGenAdapter{})

	ids := r.Available()
	if len(ids) != 2 {
		t.Fatalf("expected 2 available adapters, got %d: %v", len(ids), ids)
	}
	// Check that both IDs are present.
	idSet := make(map[string]bool)
	for _, id := range ids {
		idSet[id] = true
	}
	if !idSet["langgraph"] {
		t.Error("expected 'langgraph' in Available()")
	}
	if !idSet["autogen"] {
		t.Error("expected 'autogen' in Available()")
	}
}

func TestAdapterRegistryIsAvailable(t *testing.T) {
	r := NewAdapterRegistry()
	r.Register(&CrewAIAdapter{})

	if !r.IsAvailable("crewai") {
		t.Error("expected IsAvailable('crewai') = true")
	}
	if r.IsAvailable("langgraph") {
		t.Error("expected IsAvailable('langgraph') = false")
	}
}

// ──── RegisterAllAdapters ────────────────────────────────────────────────────

func TestRegisterAllAdapters(t *testing.T) {
	r := NewAdapterRegistry()
	RegisterAllAdapters(r)

	expectedIDs := []string{
		"langgraph",
		"autogen",
		"crewai",
		"llamaindex",
		"google_adk",
		"openai_agents",
		"pydantic_ai",
		"semantic_kernel",
	}
	for _, id := range expectedIDs {
		if !r.IsAvailable(id) {
			t.Errorf("expected adapter %q to be registered", id)
		}
	}
	if len(r.Available()) != len(expectedIDs) {
		t.Errorf("expected %d adapters, got %d: %v", len(expectedIDs), len(r.Available()), r.Available())
	}
}

// ──── LangGraphAdapter ───────────────────────────────────────────────────────

func TestLangGraphAdapterFrameworkID(t *testing.T) {
	a := &LangGraphAdapter{}
	if a.FrameworkID() != "langgraph" {
		t.Errorf("expected 'langgraph', got %q", a.FrameworkID())
	}
}

func TestLangGraphAdapterExport(t *testing.T) {
	a := &LangGraphAdapter{}
	result := a.Export(&ADP{})
	if result == nil {
		t.Error("expected non-nil Export result")
	}
}

func TestLangGraphAdapterImportFrom(t *testing.T) {
	a := &LangGraphAdapter{}
	result := a.ImportFrom(map[string]interface{}{"key": "val"})
	if result == nil {
		t.Error("expected non-nil ImportFrom result")
	}
}

func TestLangGraphAdapterRoundtripFidelity(t *testing.T) {
	a := &LangGraphAdapter{}
	f := a.RoundtripFidelity()
	if f["flow.graph"] != "faithful" {
		t.Errorf("expected flow.graph=faithful, got %q", f["flow.graph"])
	}
	if f["loop.termination"] != "faithful" {
		t.Errorf("expected loop.termination=faithful, got %q", f["loop.termination"])
	}
	if f["memory.stores"] != "faithful" {
		t.Errorf("expected memory.stores=faithful, got %q", f["memory.stores"])
	}
}

// ──── AutoGenAdapter ─────────────────────────────────────────────────────────

func TestAutoGenAdapterFrameworkID(t *testing.T) {
	a := &AutoGenAdapter{}
	if a.FrameworkID() != "autogen" {
		t.Errorf("expected 'autogen', got %q", a.FrameworkID())
	}
}

func TestAutoGenAdapterExport(t *testing.T) {
	a := &AutoGenAdapter{}
	if a.Export(&ADP{}) == nil {
		t.Error("expected non-nil Export result")
	}
}

func TestAutoGenAdapterImportFrom(t *testing.T) {
	a := &AutoGenAdapter{}
	if a.ImportFrom(map[string]interface{}{}) == nil {
		t.Error("expected non-nil ImportFrom result")
	}
}

func TestAutoGenAdapterRoundtripFidelity(t *testing.T) {
	a := &AutoGenAdapter{}
	f := a.RoundtripFidelity()
	if f["loop.termination"] != "faithful" {
		t.Errorf("expected loop.termination=faithful, got %q", f["loop.termination"])
	}
	// The default for memory.working is lossy; autogen doesn't override it.
	if f["memory.working"] != "lossy" {
		t.Errorf("expected memory.working=lossy (default), got %q", f["memory.working"])
	}
}

// ──── CrewAIAdapter ──────────────────────────────────────────────────────────

func TestCrewAIAdapterFrameworkID(t *testing.T) {
	a := &CrewAIAdapter{}
	if a.FrameworkID() != "crewai" {
		t.Errorf("expected 'crewai', got %q", a.FrameworkID())
	}
}

func TestCrewAIAdapterExport(t *testing.T) {
	a := &CrewAIAdapter{}
	if a.Export(&ADP{}) == nil {
		t.Error("expected non-nil Export result")
	}
}

func TestCrewAIAdapterImportFrom(t *testing.T) {
	a := &CrewAIAdapter{}
	if a.ImportFrom(map[string]interface{}{}) == nil {
		t.Error("expected non-nil ImportFrom result")
	}
}

func TestCrewAIAdapterRoundtripFidelity(t *testing.T) {
	a := &CrewAIAdapter{}
	f := a.RoundtripFidelity()
	if f["tools.policy"] != "faithful" {
		t.Errorf("expected tools.policy=faithful, got %q", f["tools.policy"])
	}
}

// ──── LlamaIndexAdapter ──────────────────────────────────────────────────────

func TestLlamaIndexAdapterFrameworkID(t *testing.T) {
	a := &LlamaIndexAdapter{}
	if a.FrameworkID() != "llamaindex" {
		t.Errorf("expected 'llamaindex', got %q", a.FrameworkID())
	}
}

func TestLlamaIndexAdapterExport(t *testing.T) {
	a := &LlamaIndexAdapter{}
	if a.Export(&ADP{}) == nil {
		t.Error("expected non-nil Export result")
	}
}

func TestLlamaIndexAdapterImportFrom(t *testing.T) {
	a := &LlamaIndexAdapter{}
	if a.ImportFrom(map[string]interface{}{}) == nil {
		t.Error("expected non-nil ImportFrom result")
	}
}

func TestLlamaIndexAdapterRoundtripFidelity(t *testing.T) {
	a := &LlamaIndexAdapter{}
	f := a.RoundtripFidelity()
	if f["memory.stores"] != "faithful" {
		t.Errorf("expected memory.stores=faithful, got %q", f["memory.stores"])
	}
	if f["tools"] != "faithful" {
		t.Errorf("expected tools=faithful, got %q", f["tools"])
	}
}

// ──── GoogleADKAdapter ───────────────────────────────────────────────────────

func TestGoogleADKAdapterFrameworkID(t *testing.T) {
	a := &GoogleADKAdapter{}
	if a.FrameworkID() != "google_adk" {
		t.Errorf("expected 'google_adk', got %q", a.FrameworkID())
	}
}

func TestGoogleADKAdapterExport(t *testing.T) {
	a := &GoogleADKAdapter{}
	if a.Export(&ADP{}) == nil {
		t.Error("expected non-nil Export result")
	}
}

func TestGoogleADKAdapterImportFrom(t *testing.T) {
	a := &GoogleADKAdapter{}
	if a.ImportFrom(map[string]interface{}{}) == nil {
		t.Error("expected non-nil ImportFrom result")
	}
}

func TestGoogleADKAdapterRoundtripFidelity(t *testing.T) {
	a := &GoogleADKAdapter{}
	f := a.RoundtripFidelity()
	if f["artifacts"] != "faithful" {
		t.Errorf("expected artifacts=faithful, got %q", f["artifacts"])
	}
	if f["memory.stores"] != "faithful" {
		t.Errorf("expected memory.stores=faithful, got %q", f["memory.stores"])
	}
	if f["tools"] != "faithful" {
		t.Errorf("expected tools=faithful, got %q", f["tools"])
	}
}

// ──── OpenAIAgentsAdapter ────────────────────────────────────────────────────

func TestOpenAIAgentsAdapterFrameworkID(t *testing.T) {
	a := &OpenAIAgentsAdapter{}
	if a.FrameworkID() != "openai_agents" {
		t.Errorf("expected 'openai_agents', got %q", a.FrameworkID())
	}
}

func TestOpenAIAgentsAdapterExport(t *testing.T) {
	a := &OpenAIAgentsAdapter{}
	if a.Export(&ADP{}) == nil {
		t.Error("expected non-nil Export result")
	}
}

func TestOpenAIAgentsAdapterImportFrom(t *testing.T) {
	a := &OpenAIAgentsAdapter{}
	if a.ImportFrom(map[string]interface{}{}) == nil {
		t.Error("expected non-nil ImportFrom result")
	}
}

func TestOpenAIAgentsAdapterRoundtripFidelity(t *testing.T) {
	a := &OpenAIAgentsAdapter{}
	f := a.RoundtripFidelity()
	if f["guardrails.interrupts"] != "faithful" {
		t.Errorf("expected guardrails.interrupts=faithful, got %q", f["guardrails.interrupts"])
	}
	if f["observability"] != "faithful" {
		t.Errorf("expected observability=faithful, got %q", f["observability"])
	}
}

// ──── PydanticAIAdapter ──────────────────────────────────────────────────────

func TestPydanticAIAdapterFrameworkID(t *testing.T) {
	a := &PydanticAIAdapter{}
	if a.FrameworkID() != "pydantic_ai" {
		t.Errorf("expected 'pydantic_ai', got %q", a.FrameworkID())
	}
}

func TestPydanticAIAdapterExport(t *testing.T) {
	a := &PydanticAIAdapter{}
	if a.Export(&ADP{}) == nil {
		t.Error("expected non-nil Export result")
	}
}

func TestPydanticAIAdapterImportFrom(t *testing.T) {
	a := &PydanticAIAdapter{}
	if a.ImportFrom(map[string]interface{}{}) == nil {
		t.Error("expected non-nil ImportFrom result")
	}
}

func TestPydanticAIAdapterRoundtripFidelity(t *testing.T) {
	a := &PydanticAIAdapter{}
	f := a.RoundtripFidelity()
	if f["runtime.models"] != "faithful" {
		t.Errorf("expected runtime.models=faithful, got %q", f["runtime.models"])
	}
}

// ──── SemanticKernelAdapter ──────────────────────────────────────────────────

func TestSemanticKernelAdapterFrameworkID(t *testing.T) {
	a := &SemanticKernelAdapter{}
	if a.FrameworkID() != "semantic_kernel" {
		t.Errorf("expected 'semantic_kernel', got %q", a.FrameworkID())
	}
}

func TestSemanticKernelAdapterExport(t *testing.T) {
	a := &SemanticKernelAdapter{}
	if a.Export(&ADP{}) == nil {
		t.Error("expected non-nil Export result")
	}
}

func TestSemanticKernelAdapterImportFrom(t *testing.T) {
	a := &SemanticKernelAdapter{}
	if a.ImportFrom(map[string]interface{}{}) == nil {
		t.Error("expected non-nil ImportFrom result")
	}
}

func TestSemanticKernelAdapterRoundtripFidelity(t *testing.T) {
	a := &SemanticKernelAdapter{}
	f := a.RoundtripFidelity()
	if f["runtime.models"] != "faithful" {
		t.Errorf("expected runtime.models=faithful, got %q", f["runtime.models"])
	}
	if f["tools"] != "faithful" {
		t.Errorf("expected tools=faithful, got %q", f["tools"])
	}
}

// ──── Interface compliance ────────────────────────────────────────────────────

// Verify all concrete adapters satisfy the Adapter interface at compile time.
var _ Adapter = (*LangGraphAdapter)(nil)
var _ Adapter = (*AutoGenAdapter)(nil)
var _ Adapter = (*CrewAIAdapter)(nil)
var _ Adapter = (*LlamaIndexAdapter)(nil)
var _ Adapter = (*GoogleADKAdapter)(nil)
var _ Adapter = (*OpenAIAgentsAdapter)(nil)
var _ Adapter = (*PydanticAIAdapter)(nil)
var _ Adapter = (*SemanticKernelAdapter)(nil)
