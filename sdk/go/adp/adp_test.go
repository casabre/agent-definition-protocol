package adp

import (
	"os"
	"path/filepath"
	"testing"
)

func minimalFlow() interface{} {
	return map[string]interface{}{
		"id": "f",
		"graph": map[string]interface{}{
			"nodes":       []interface{}{map[string]interface{}{"id": "n", "kind": "input"}},
			"edges":       []interface{}{},
			"start_nodes": []interface{}{"n"},
			"end_nodes":   []interface{}{"n"},
		},
	}
}

func minimalEvaluation() interface{} {
	return map[string]interface{}{
		"suites": []interface{}{map[string]interface{}{
			"id": "s",
			"metrics": []interface{}{map[string]interface{}{
				"id": "m", "type": "deterministic", "function": "noop",
				"scoring": "boolean", "threshold": true,
			}},
		}},
	}
}

func writeAgent(dir, content string) string {
	os.MkdirAll(filepath.Join(dir, "adp"), 0o755)
	path := filepath.Join(dir, "adp", "agent.yaml")
	os.WriteFile(path, []byte(content), 0o644)
	return path
}

func TestLoadADPPaths(t *testing.T) {
	tmp, _ := os.MkdirTemp("", "adp-unit-*")
	defer os.RemoveAll(tmp)
	path := writeAgent(tmp, `adp_version: "0.1.0"
id: "unit.agent"
runtime:
  execution:
    - backend: python
      id: py
      entrypoint: main:app
flow:
  id: "f"
  graph:
    nodes:
      - id: "n"
        kind: "input"
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - id: "m"
          type: "deterministic"
          function: "noop"
          scoring: "boolean"
          threshold: true
`)
	adp, err := LoadADP(path)
	if err != nil {
		t.Fatalf("load failed: %v", err)
	}
	if adp.ID != "unit.agent" {
		t.Errorf("expected id 'unit.agent', got '%s'", adp.ID)
	}
	if adp.ADPVersion != "0.1.0" {
		t.Errorf("expected version '0.1.0', got '%s'", adp.ADPVersion)
	}
	if len(adp.Runtime.Execution) != 1 {
		t.Errorf("expected 1 execution entry, got %d", len(adp.Runtime.Execution))
	}
	if adp.Runtime.Execution[0].Backend != "python" {
		t.Errorf("expected backend 'python', got '%s'", adp.Runtime.Execution[0].Backend)
	}
	
	if _, err := LoadADP(filepath.Join(tmp, "missing.yaml")); err == nil {
		t.Fatal("expected missing file error")
	}
	bad := writeAgent(tmp, ":::bad")
	if _, err := LoadADP(bad); err == nil {
		t.Fatal("expected yaml parse error")
	}
}

func TestValidateADP(t *testing.T) {
	valid := &ADP{
		ADPVersion: "0.1.0",
		ID:         "ok",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "main:app"}}},
		Flow:       minimalFlow(),
		Evaluation: minimalEvaluation(),
	}
	if err := ValidateADP(valid); err != nil {
		t.Fatalf("unexpected validation error: %v", err)
	}

	invalid := &ADP{ADPVersion: "0.0.1", Runtime: Runtime{Execution: []RuntimeEntry{}}}
	if err := ValidateADP(invalid); err == nil {
		t.Fatal("expected validation error")
	}
}

func TestValidateADPEmptyExecution(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "test",
		Runtime:    Runtime{Execution: []RuntimeEntry{}},
		Flow:       map[string]interface{}{},
		Evaluation: map[string]interface{}{},
	}
	if err := ValidateADP(adp); err == nil {
		t.Fatal("expected validation error for empty execution")
	}
}

func TestValidateADPEmptyID(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "main:app"}}},
	}
	err := ValidateADP(adp)
	if err == nil {
		t.Fatal("expected validation error for empty id")
	}
}

func TestValidateADPInvalidVersion(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.3.0", // Invalid version (not in schema enum)
		ID:         "test",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "main:app"}}},
		Flow:       map[string]interface{}{},
		Evaluation: map[string]interface{}{},
	}
	if err := ValidateADP(adp); err == nil {
		t.Fatal("expected validation error for invalid version")
	}
}

func TestValidateADPV0_1_0(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "agent.v0.1.0",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "main:app"}}},
		Flow: map[string]interface{}{
			"id": "test.flow",
			"graph": map[string]interface{}{
				"nodes":       []interface{}{map[string]interface{}{"id": "input", "kind": "input"}, map[string]interface{}{"id": "output", "kind": "output"}},
				"edges":       []interface{}{},
				"start_nodes": []interface{}{"input"},
				"end_nodes":   []interface{}{"output"},
			},
		},
		Evaluation: minimalEvaluation(),
	}
	if err := ValidateADP(adp); err != nil {
		t.Fatalf("unexpected validation error for v0.1.0: %v", err)
	}
}

func TestValidateADPMultipleBackends(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "multi",
		Runtime: Runtime{Execution: []RuntimeEntry{
			{Backend: "docker", ID: "docker", Image: "registry/img:latest"},
			{Backend: "python", ID: "python", Entrypoint: "main:app"},
			{Backend: "wasm", ID: "wasm", Module: "agent.wasm"},
		}},
		Flow:       minimalFlow(),
		Evaluation: minimalEvaluation(),
	}
	if err := ValidateADP(adp); err != nil {
		t.Fatalf("unexpected validation error: %v", err)
	}
	if len(adp.Runtime.Execution) != 3 {
		t.Errorf("expected 3 execution entries, got %d", len(adp.Runtime.Execution))
	}
}

func TestValidateADPDifferentBackends(t *testing.T) {
	backends := []string{"docker", "wasm", "python", "typescript", "binary", "custom"}
	for _, backend := range backends {
		adp := &ADP{
			ADPVersion: "0.1.0",
			ID:         backend,
			Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: backend, ID: backend + "-id"}}},
			Flow:       map[string]interface{}{},
			Evaluation: map[string]interface{}{},
		}
		// Should not crash, validation may or may not check backend type
		_ = ValidateADP(adp)
	}
}

func TestValidateADPWithOptionalFields(t *testing.T) {
	adp := &ADP{
		ADPVersion:  "0.1.0",
		ID:          "full",
		Name:        "Full Agent",
		Description: "Test agent with all optional fields",
		Owner:       "test-team",
		Tags:        []string{"test", "demo"},
		Runtime:     Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "main:app"}}},
		Flow:        minimalFlow(),
		Evaluation:  minimalEvaluation(),
	}
	if err := ValidateADP(adp); err != nil {
		t.Fatalf("unexpected validation error: %v", err)
	}
	if adp.ID != "full" {
		t.Errorf("expected id 'full', got '%s'", adp.ID)
	}
	if len(adp.Runtime.Execution) != 1 {
		t.Errorf("expected 1 execution entry, got %d", len(adp.Runtime.Execution))
	}
}
