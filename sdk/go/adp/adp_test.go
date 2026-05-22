package adp

import (
	"os"
	"path/filepath"
	"strings"
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
		ADPVersion: "9.9.9",
		ID:         "test",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "main:app"}}},
		Flow:       map[string]interface{}{},
		Evaluation: map[string]interface{}{},
	}
	if err := ValidateADP(adp); err == nil {
		t.Fatal("expected validation error for invalid version")
	}
}

func TestValidateADPV0_3_0(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.3.0",
		ID:         "agent.v0.3.0",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "main:app"}}},
		Flow:       minimalFlow(),
		Evaluation: minimalEvaluation(),
	}
	if err := ValidateADP(adp); err != nil {
		t.Fatalf("unexpected error for v0.3.0: %v", err)
	}
}

func TestSemanticsCheck12HookNodeFilterUnknown(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "hook-check",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow: map[string]interface{}{
			"graph": map[string]interface{}{
				"nodes":       []interface{}{map[string]interface{}{"id": "n1", "kind": "input"}},
				"edges":       []interface{}{},
				"start_nodes": []interface{}{"n1"},
				"end_nodes":   []interface{}{"n1"},
			},
		},
		Evaluation: map[string]interface{}{},
		Hooks: []interface{}{
			map[string]interface{}{
				"event":       "on_node_end",
				"node_filter": []interface{}{"n1", "ghost-node"},
				"handler":     map[string]interface{}{"type": "function", "function_ref": "mod:fn"},
			},
		},
	}
	errs := ValidateADPSemantics(adp)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "node_filter") && strings.Contains(e, "ghost-node") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected hook node_filter error, got: %v", errs)
	}
}

func TestSemanticsCheck13SubflowAdpRefUnknown(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "subflow-check",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow: map[string]interface{}{
			"graph": map[string]interface{}{
				"nodes":       []interface{}{map[string]interface{}{"id": "delegate", "kind": "subflow", "adp_ref": "unknown-subagent"}},
				"edges":       []interface{}{},
				"start_nodes": []interface{}{"delegate"},
				"end_nodes":   []interface{}{"delegate"},
			},
		},
		Evaluation: map[string]interface{}{},
		Subagents:  []Subagent{{ID: "known-subagent", Ref: "./other/agent.yaml"}},
	}
	errs := ValidateADPSemantics(adp)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "adp_ref") || strings.Contains(e, "subagents") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected subflow adp_ref error, got: %v", errs)
	}
}

func TestSemanticsCheck14EvaluatorRefUnknown(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "eval-ref-check",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow:       minimalFlow(),
		Evaluation: map[string]interface{}{
			"suites": []interface{}{map[string]interface{}{
				"id": "s1",
				"metrics": []interface{}{map[string]interface{}{
					"id": "m1", "type": "deterministic", "function": "noop",
					"scoring": "boolean", "threshold": true,
					"evaluator_ref": "missing-evaluator",
				}},
			}},
		},
		XTesting: map[string]interface{}{
			"evaluators": []interface{}{
				map[string]interface{}{"id": "known-evaluator", "type": "llm_judge", "model": "gpt-4o"},
			},
		},
	}
	errs := ValidateADPSemantics(adp)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "evaluator_ref") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected evaluator_ref error, got: %v", errs)
	}
}

func TestLoadEvaluatorUnsupportedTypes(t *testing.T) {
	for _, evalType := range []string{"deterministic", "llm_judge", "container"} {
		_, err := LoadEvaluator(map[string]interface{}{"id": "e1", "type": evalType})
		if err == nil {
			t.Errorf("expected error for type %q, got nil", evalType)
		}
	}
}

func TestLoadEvaluatorScriptMissingRuntime(t *testing.T) {
	_, err := LoadEvaluator(map[string]interface{}{"id": "e1", "type": "script", "inline": "echo hi"})
	if err == nil {
		t.Fatal("expected error when runtime is missing")
	}
}

func TestLoadEvaluatorScriptNonBashRuntime(t *testing.T) {
	_, err := LoadEvaluator(map[string]interface{}{
		"id": "e1", "type": "script", "runtime": "python",
		"inline": "def evaluate(o, c): return True",
	})
	if err == nil {
		t.Fatal("expected error for python runtime in Go SDK")
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

func TestValidateADPSemanticsPasses(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "ok",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "main:app"}}},
		Flow: map[string]interface{}{
			"graph": map[string]interface{}{
				"nodes":       []interface{}{map[string]interface{}{"id": "n1", "kind": "input"}, map[string]interface{}{"id": "n2", "kind": "output"}},
				"edges":       []interface{}{map[string]interface{}{"from": "n1", "to": "n2"}},
				"start_nodes": []interface{}{"n1"},
				"end_nodes":   []interface{}{"n2"},
			},
		},
		Evaluation: map[string]interface{}{
			"suites": []interface{}{map[string]interface{}{"id": "s1", "metrics": []interface{}{}}},
		},
	}
	errors := ValidateADPSemantics(adp)
	if len(errors) != 0 {
		t.Fatalf("expected no semantic errors, got: %v", errors)
	}
}

func TestValidateADPSemanticsDanglingEdge(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "dangling",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "main:app"}}},
		Flow: map[string]interface{}{
			"graph": map[string]interface{}{
				"nodes":       []interface{}{map[string]interface{}{"id": "input", "kind": "input"}},
				"edges":       []interface{}{map[string]interface{}{"from": "ghost", "to": "input"}},
				"start_nodes": []interface{}{"input"},
				"end_nodes":   []interface{}{"input"},
			},
		},
		Evaluation: map[string]interface{}{},
	}
	errors := ValidateADPSemantics(adp)
	found := false
	for _, e := range errors {
		if strings.Contains(e, "ghost") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected dangling edge error mentioning 'ghost', got: %v", errors)
	}
}

func TestValidateADPSemanticsDuplicateNode(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "dup",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "main:app"}}},
		Flow: map[string]interface{}{
			"graph": map[string]interface{}{
				"nodes":       []interface{}{map[string]interface{}{"id": "input", "kind": "input"}, map[string]interface{}{"id": "input", "kind": "output"}},
				"edges":       []interface{}{},
				"start_nodes": []interface{}{"input"},
				"end_nodes":   []interface{}{"input"},
			},
		},
		Evaluation: map[string]interface{}{},
	}
	errors := ValidateADPSemantics(adp)
	found := false
	for _, e := range errors {
		if strings.Contains(e, "duplicate") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected duplicate node error, got: %v", errors)
	}
}

func TestValidateADPSemanticsBadSuiteRef(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "suite-ref",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "main:app"}}},
		Flow: map[string]interface{}{
			"graph": map[string]interface{}{
				"nodes":       []interface{}{map[string]interface{}{"id": "n1", "kind": "llm", "suite_ref": "missing-suite"}},
				"edges":       []interface{}{},
				"start_nodes": []interface{}{"n1"},
				"end_nodes":   []interface{}{"n1"},
			},
		},
		Evaluation: map[string]interface{}{
			"suites": []interface{}{map[string]interface{}{"id": "suite1", "metrics": []interface{}{}}},
		},
	}
	errors := ValidateADPSemantics(adp)
	found := false
	for _, e := range errors {
		if strings.Contains(e, "suite_ref") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected suite_ref error, got: %v", errors)
	}
}

func TestValidateADPSemanticsBadModelRef(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "model-ref",
		Runtime: Runtime{
			Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "main:app"}},
			Models:    []Model{{ID: "gpt4", Provider: "openai", Model: "gpt-4o"}},
		},
		Flow: map[string]interface{}{
			"graph": map[string]interface{}{
				"nodes":       []interface{}{map[string]interface{}{"id": "n1", "kind": "llm", "model_ref": "missing-model"}},
				"edges":       []interface{}{},
				"start_nodes": []interface{}{"n1"},
				"end_nodes":   []interface{}{"n1"},
			},
		},
		Evaluation: map[string]interface{}{},
	}
	errors := ValidateADPSemantics(adp)
	found := false
	for _, e := range errors {
		if strings.Contains(e, "model_ref") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected model_ref error, got: %v", errors)
	}
}

func TestValidateADPSemanticsBadRuntimeRef(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "runtime-ref",
		Runtime: Runtime{
			Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "main:app"}},
		},
		Flow: map[string]interface{}{
			"graph": map[string]interface{}{
				"nodes":       []interface{}{map[string]interface{}{"id": "n1", "kind": "llm", "runtime_ref": "missing-backend"}},
				"edges":       []interface{}{},
				"start_nodes": []interface{}{"n1"},
				"end_nodes":   []interface{}{"n1"},
			},
		},
		Evaluation: map[string]interface{}{},
	}
	errors := ValidateADPSemantics(adp)
	found := false
	for _, e := range errors {
		if strings.Contains(e, "runtime_ref") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected runtime_ref error, got: %v", errors)
	}
}

func TestValidateADPConformanceClassFullRejectsEmptyFlow(t *testing.T) {
	adp := &ADP{
		ADPVersion:       "0.1.0",
		ID:               "agent.full",
		ConformanceClass: "full",
		Runtime:          Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "main:app"}}},
		Flow:             map[string]interface{}{},
		Evaluation:       minimalEvaluation(),
	}
	err := ValidateADP(adp)
	if err == nil {
		t.Fatal("expected error for conformance_class=full with empty flow")
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
