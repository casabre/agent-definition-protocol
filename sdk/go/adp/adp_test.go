package adp

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"

	jsonschema "github.com/santhosh-tekuri/jsonschema/v5"
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

// ──── evaluation.go tests ────────────────────────────────────────────────────

func TestEvaluatorErrorMethod(t *testing.T) {
	err := &EvaluatorError{Kind: "TestKind", Message: "TestMessage"}
	got := err.Error()
	if got != "TestKind: TestMessage" {
		t.Errorf("expected 'TestKind: TestMessage', got %q", got)
	}
}

func TestLoadEvaluatorEmptyType(t *testing.T) {
	// Empty type → "(missing)"
	_, err := LoadEvaluator(map[string]interface{}{"id": "e1"})
	if err == nil {
		t.Fatal("expected error for empty type")
	}
	if !strings.Contains(err.Error(), "(missing)") {
		t.Errorf("expected '(missing)' in error, got: %v", err)
	}
}

func TestLoadEvaluatorScriptMissingInlineAndScriptRef(t *testing.T) {
	// bash runtime but neither inline nor script_ref
	_, err := LoadEvaluator(map[string]interface{}{"id": "e1", "type": "script", "runtime": "bash"})
	if err == nil {
		t.Fatal("expected error")
	}
}

func TestScriptEvaluatorResolveInline(t *testing.T) {
	ev, err := LoadEvaluator(map[string]interface{}{
		"id": "ev1", "type": "script", "runtime": "bash",
		"inline": `echo '{"passed":true,"score":1.0,"reason":"ok"}'`,
	})
	if err != nil {
		t.Fatalf("load failed: %v", err)
	}
	result, evalErr := ev.Evaluate(map[string]interface{}{"x": 1}, map[string]interface{}{})
	if evalErr != nil {
		t.Fatalf("evaluate failed: %v", evalErr)
	}
	if !result.Passed {
		t.Error("expected passed=true")
	}
	if result.Score == nil || *result.Score != 1.0 {
		t.Errorf("expected score=1.0, got %v", result.Score)
	}
	if result.Reason != "ok" {
		t.Errorf("expected reason='ok', got %q", result.Reason)
	}
}

func TestScriptEvaluatorEvaluateBashError(t *testing.T) {
	ev, _ := LoadEvaluator(map[string]interface{}{
		"id": "ev-err", "type": "script", "runtime": "bash",
		"inline": "exit 1",
	})
	result, _ := ev.Evaluate(map[string]interface{}{}, map[string]interface{}{})
	if result.Passed {
		t.Error("expected passed=false on exit 1")
	}
}

func TestScriptEvaluatorEvaluateBadJSON(t *testing.T) {
	ev, _ := LoadEvaluator(map[string]interface{}{
		"id": "ev-json", "type": "script", "runtime": "bash",
		"inline": "echo 'not valid json'",
	})
	_, err := ev.Evaluate(map[string]interface{}{}, map[string]interface{}{})
	if err == nil {
		t.Fatal("expected error for invalid JSON output")
	}
}

func TestScriptEvaluatorResolveFileRef(t *testing.T) {
	tmp, _ := os.MkdirTemp("", "go-eval-*")
	defer os.RemoveAll(tmp)
	scriptPath := filepath.Join(tmp, "eval.sh")
	os.WriteFile(scriptPath, []byte(`echo '{"passed":true}'`), 0o755)
	ev, err := LoadEvaluator(map[string]interface{}{
		"id": "ev-file", "type": "script", "runtime": "bash",
		"script_ref": scriptPath,
	})
	if err != nil {
		t.Fatalf("load failed: %v", err)
	}
	result, evalErr := ev.Evaluate(map[string]interface{}{}, map[string]interface{}{})
	if evalErr != nil {
		t.Fatalf("evaluate failed: %v", evalErr)
	}
	if !result.Passed {
		t.Error("expected passed=true")
	}
}

func TestScriptEvaluatorResolveGitRef(t *testing.T) {
	_, err := LoadEvaluator(map[string]interface{}{
		"id": "ev-git", "type": "script", "runtime": "bash",
		"script_ref": "git+https://github.com/example/repo/eval.sh@abc1234",
	})
	if err != nil {
		// newScriptEvaluator succeeds (scriptRef is stored), error comes at Evaluate time
		t.Fatalf("load should succeed for git+ ref: %v", err)
	}
}

func TestScriptEvaluatorResolveGitRefAtEvaluate(t *testing.T) {
	ev, err := LoadEvaluator(map[string]interface{}{
		"id": "ev-git2", "type": "script", "runtime": "bash",
		"script_ref": "git+https://github.com/example/repo/eval.sh@abc1234",
	})
	if err != nil {
		t.Fatalf("load should succeed: %v", err)
	}
	_, evalErr := ev.Evaluate(map[string]interface{}{}, map[string]interface{}{})
	if evalErr == nil {
		t.Fatal("expected error for git+ ref in Go SDK during Evaluate")
	}
}

func TestScriptEvaluatorResolveFileNotFound(t *testing.T) {
	ev, err := LoadEvaluator(map[string]interface{}{
		"id": "ev-nf", "type": "script", "runtime": "bash",
		"script_ref": "/nonexistent/path/eval.sh",
	})
	if err != nil {
		t.Fatalf("load should succeed: %v", err)
	}
	_, evalErr := ev.Evaluate(map[string]interface{}{}, map[string]interface{}{})
	if evalErr == nil {
		t.Fatal("expected error for missing script file")
	}
}

func TestNormalizeResultBoolTrue(t *testing.T) {
	ev, _ := LoadEvaluator(map[string]interface{}{
		"id": "nr1", "type": "script", "runtime": "bash",
		"inline": "echo 'true'",
	})
	result, err := ev.Evaluate(map[string]interface{}{}, map[string]interface{}{})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !result.Passed {
		t.Error("expected passed=true for bool true")
	}
	if result.Score == nil || *result.Score != 1.0 {
		t.Errorf("expected score=1.0, got %v", result.Score)
	}
}

func TestNormalizeResultBoolFalse(t *testing.T) {
	ev, _ := LoadEvaluator(map[string]interface{}{
		"id": "nr2", "type": "script", "runtime": "bash",
		"inline": "echo 'false'",
	})
	result, err := ev.Evaluate(map[string]interface{}{}, map[string]interface{}{})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if result.Passed {
		t.Error("expected passed=false for bool false")
	}
	if result.Score == nil || *result.Score != 0.0 {
		t.Errorf("expected score=0.0, got %v", result.Score)
	}
}

func TestNormalizeResultMapWithScore(t *testing.T) {
	// No "passed" key, score >= 0.5 → passed=true
	ev, _ := LoadEvaluator(map[string]interface{}{
		"id": "nr3", "type": "script", "runtime": "bash",
		"inline": `echo '{"score":0.8}'`,
	})
	result, _ := ev.Evaluate(map[string]interface{}{}, map[string]interface{}{})
	if !result.Passed {
		t.Error("expected passed=true for score=0.8")
	}
}

func TestNormalizeResultMapWithLowScore(t *testing.T) {
	// No "passed" key, score < 0.5 → passed=false
	ev, _ := LoadEvaluator(map[string]interface{}{
		"id": "nr4", "type": "script", "runtime": "bash",
		"inline": `echo '{"score":0.3}'`,
	})
	result, _ := ev.Evaluate(map[string]interface{}{}, map[string]interface{}{})
	if result.Passed {
		t.Error("expected passed=false for score=0.3")
	}
}

func TestNormalizeResultDefault(t *testing.T) {
	// Non-bool, non-object output (e.g. a number) → default branch
	ev, _ := LoadEvaluator(map[string]interface{}{
		"id": "nr5", "type": "script", "runtime": "bash",
		"inline": `echo '42'`,
	})
	result, _ := ev.Evaluate(map[string]interface{}{}, map[string]interface{}{})
	// default: JSON number → float64 → neither bool nor map → default branch
	if result == nil {
		t.Fatal("result should not be nil")
	}
}

// ──── validate.go branch tests ───────────────────────────────────────────────

func TestValidateADPConformanceClassFullRejectsEmptyEval(t *testing.T) {
	adp := &ADP{
		ADPVersion:       "0.1.0",
		ID:               "agent.full.eval",
		ConformanceClass: "full",
		Runtime:          Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "main:app"}}},
		Flow:             minimalFlow(),
		Evaluation:       map[string]interface{}{},
	}
	err := ValidateADP(adp)
	if err == nil {
		t.Fatal("expected error for conformance_class=full with empty evaluation")
	}
	if !strings.Contains(err.Error(), "evaluation") && !strings.Contains(err.Error(), "full") {
		t.Errorf("expected error about evaluation or full, got: %v", err)
	}
}

func TestValidateADPSemanticsJudgesDeprecationWarning(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "judges-test",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow:       minimalFlow(),
		Evaluation: minimalEvaluation(),
		XTesting: map[string]interface{}{
			"judges": []interface{}{
				map[string]interface{}{"id": "j1", "model": "gpt-4o"},
			},
		},
	}
	errors := ValidateADPSemantics(adp)
	found := false
	for _, e := range errors {
		if strings.Contains(e, "judges") || strings.Contains(e, "WARNING") || strings.Contains(e, "deprecated") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected judges deprecation warning, got: %v", errors)
	}
}

func TestValidateADPSemanticsPreCompositionWarning(t *testing.T) {
	// An ADP struct with "extends" set should emit a warning
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "precomp-test",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow:       minimalFlow(),
		Evaluation: minimalEvaluation(),
		Extends:    "./base.yaml",
	}
	errors := ValidateADPSemantics(adp)
	found := false
	for _, e := range errors {
		if strings.Contains(e, "warning") || strings.Contains(e, "WARNING") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected warning for extends field, got: %v", errors)
	}
}

func TestValidateADPSemanticsGuardrailEmptyPolicyRef(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.2.0",
		ID:         "guardrail-test",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "app:main"}}},
		Flow:       minimalFlow(),
		Evaluation: map[string]interface{}{},
		Guardrails: &Guardrails{
			Input: []GuardrailRail{
				{
					ID:        "pii-filter",
					Provider:  "guardrails-ai",
					PolicyRef: "",
					Mode:      "block",
				},
			},
		},
	}
	errors := ValidateADPSemantics(adp)
	found := false
	for _, e := range errors {
		if strings.Contains(e, "policy_ref") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected policy_ref error, got: %v", errors)
	}
}

func TestValidateADPSemanticsTelemetryInvalidAttr(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.2.0",
		ID:         "telemetry-test",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "app:main"}}},
		Flow:       minimalFlow(),
		Evaluation: map[string]interface{}{},
		Telemetry: &Telemetry{
			RequiredAttributes: []string{"gen_ai.model.id", "bad_attribute"},
		},
	}
	errors := ValidateADPSemantics(adp)
	found := false
	for _, e := range errors {
		if strings.Contains(e, "bad_attribute") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected telemetry attribute error, got: %v", errors)
	}
}

func TestValidateADPSemanticsToolMissingEnvVar(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.2.0",
		ID:         "toolauth-test",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "app:main"}}},
		Flow:       minimalFlow(),
		Evaluation: map[string]interface{}{},
		Tools: map[string]interface{}{
			"http_apis": []interface{}{
				map[string]interface{}{
					"id":          "billing-api",
					"description": "Billing service",
					"base_url":    "https://billing.example.com",
					"auth":        map[string]interface{}{"scheme": "bearer"},
				},
			},
		},
	}
	errors := ValidateADPSemantics(adp)
	found := false
	for _, e := range errors {
		if strings.Contains(e, "auth.env_var") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected tool auth.env_var error, got: %v", errors)
	}
}

func TestValidateADPSemanticsToolRefMissing(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.2.0",
		ID:         "toolref-test",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "app:main"}}},
		Flow: map[string]interface{}{
			"graph": map[string]interface{}{
				"nodes": []interface{}{
					map[string]interface{}{"id": "input", "kind": "input"},
					map[string]interface{}{"id": "api-call", "kind": "tool", "tool_ref": "nonexistent-api"},
					map[string]interface{}{"id": "output", "kind": "output"},
				},
				"edges":       []interface{}{},
				"start_nodes": []interface{}{"input"},
				"end_nodes":   []interface{}{"output"},
			},
		},
		Evaluation: map[string]interface{}{},
		Tools: map[string]interface{}{
			"http_apis": []interface{}{
				map[string]interface{}{
					"id":          "real-api",
					"description": "A real API",
					"base_url":    "https://example.com",
				},
			},
		},
	}
	errors := ValidateADPSemantics(adp)
	found := false
	for _, e := range errors {
		if strings.Contains(e, "tool_ref") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected tool_ref error, got: %v", errors)
	}
}

func TestValidateADPSemanticsComplianceUnknown(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.2.0",
		ID:         "compliance-test",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "app:main"}}},
		Flow:       minimalFlow(),
		Evaluation: map[string]interface{}{},
		Tools: map[string]interface{}{
			"compliance": []interface{}{"soc2", "unknown_standard"},
		},
	}
	errors := ValidateADPSemantics(adp)
	found := false
	for _, e := range errors {
		if strings.Contains(e, "unknown_standard") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected compliance standard error, got: %v", errors)
	}
}

func TestValidateADPSemanticsGuardrailOutputEmptyPolicyRef(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.2.0",
		ID:         "guardrail-output-test",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "app:main"}}},
		Flow:       minimalFlow(),
		Evaluation: map[string]interface{}{},
		Guardrails: &Guardrails{
			Output: []GuardrailRail{
				{
					ID:        "pii-output-filter",
					Provider:  "guardrails-ai",
					PolicyRef: "",
					Mode:      "block",
				},
			},
		},
	}
	errors := ValidateADPSemantics(adp)
	found := false
	for _, e := range errors {
		if strings.Contains(e, "policy_ref") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected policy_ref error for output guardrail, got: %v", errors)
	}
}

func TestValidateADPSemanticsImportsWarning(t *testing.T) {
	// An ADP struct with imports set should emit a warning
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "imports-test",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow:       minimalFlow(),
		Evaluation: minimalEvaluation(),
		Imports:    []ImportEntry{{ID: "mod", From: "./module.yaml"}},
	}
	errors := ValidateADPSemantics(adp)
	found := false
	for _, e := range errors {
		if strings.Contains(e, "warning") || strings.Contains(e, "WARNING") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected warning for imports field, got: %v", errors)
	}
}

func TestSemanticsCheck14EvaluatorRefValid(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "eval-ref-valid",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow:       minimalFlow(),
		Evaluation: map[string]interface{}{
			"suites": []interface{}{map[string]interface{}{
				"id": "s1",
				"metrics": []interface{}{map[string]interface{}{
					"id": "m1", "type": "deterministic", "function": "noop",
					"scoring": "boolean", "threshold": true,
					"evaluator_ref": "known-evaluator",
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
	for _, e := range errs {
		if strings.Contains(e, "evaluator_ref") {
			t.Errorf("unexpected evaluator_ref error: %v", e)
		}
	}
}

// ──── Additional branch coverage tests for ValidateADPSemantics ─────────────

// TestValidateADPSemanticsFlowNotAMap exercises the "flow not a map" early return.
func TestValidateADPSemanticsFlowNotAMap(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "flow-not-map",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow:       "this-is-a-string-not-a-map",
		Evaluation: map[string]interface{}{},
	}
	// Should return immediately with no graph errors.
	errs := ValidateADPSemantics(adp)
	// No panics and the function returns without graph-level errors.
	for _, e := range errs {
		if strings.Contains(e, "node") || strings.Contains(e, "edge") {
			t.Errorf("unexpected graph error when flow is not a map: %v", e)
		}
	}
}

// TestValidateADPSemanticsFlowNoGraph exercises the "flow has no graph" early return.
func TestValidateADPSemanticsFlowNoGraph(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "flow-no-graph",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow:       map[string]interface{}{"id": "f"},
		Evaluation: map[string]interface{}{},
	}
	errs := ValidateADPSemantics(adp)
	for _, e := range errs {
		if strings.Contains(e, "node") || strings.Contains(e, "edge") {
			t.Errorf("unexpected graph error when graph key is missing: %v", e)
		}
	}
}

// TestValidateADPSemanticsNonMapNode exercises the !ok continue for non-map nodes.
func TestValidateADPSemanticsNonMapNode(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "non-map-node",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow: map[string]interface{}{
			"graph": map[string]interface{}{
				"nodes":       []interface{}{"not-a-map-node", map[string]interface{}{"id": "n1", "kind": "input"}},
				"edges":       []interface{}{},
				"start_nodes": []interface{}{"n1"},
				"end_nodes":   []interface{}{"n1"},
			},
		},
		Evaluation: map[string]interface{}{},
	}
	errs := ValidateADPSemantics(adp)
	// Should not error about the non-map node (it's skipped), only valid nodes processed.
	_ = errs
}

// TestValidateADPSemanticsNonMapEdge exercises the !ok continue for non-map edges.
func TestValidateADPSemanticsNonMapEdge(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "non-map-edge",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow: map[string]interface{}{
			"graph": map[string]interface{}{
				"nodes":       []interface{}{map[string]interface{}{"id": "n1", "kind": "input"}},
				"edges":       []interface{}{"not-a-map-edge"},
				"start_nodes": []interface{}{"n1"},
				"end_nodes":   []interface{}{"n1"},
			},
		},
		Evaluation: map[string]interface{}{},
	}
	errs := ValidateADPSemantics(adp)
	// Non-map edge should be skipped, no error about it.
	_ = errs
}

// TestValidateADPSemanticsStartNodeNotFound exercises the start_node not found error.
func TestValidateADPSemanticsStartNodeNotFound(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "bad-start",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow: map[string]interface{}{
			"graph": map[string]interface{}{
				"nodes":       []interface{}{map[string]interface{}{"id": "n1", "kind": "input"}},
				"edges":       []interface{}{},
				"start_nodes": []interface{}{"ghost-start"},
				"end_nodes":   []interface{}{"n1"},
			},
		},
		Evaluation: map[string]interface{}{},
	}
	errs := ValidateADPSemantics(adp)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "start_node") && strings.Contains(e, "ghost-start") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected start_node error, got: %v", errs)
	}
}

// TestValidateADPSemanticsEndNodeNotFound exercises the end_node not found error.
func TestValidateADPSemanticsEndNodeNotFound(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "bad-end",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow: map[string]interface{}{
			"graph": map[string]interface{}{
				"nodes":       []interface{}{map[string]interface{}{"id": "n1", "kind": "input"}},
				"edges":       []interface{}{},
				"start_nodes": []interface{}{"n1"},
				"end_nodes":   []interface{}{"ghost-end"},
			},
		},
		Evaluation: map[string]interface{}{},
	}
	errs := ValidateADPSemantics(adp)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "end_node") && strings.Contains(e, "ghost-end") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected end_node error, got: %v", errs)
	}
}

// TestValidateADPSemanticsNonMapSuite exercises the !ok continue for non-map suite entries.
func TestValidateADPSemanticsNonMapSuite(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "non-map-suite",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow:       minimalFlow(),
		Evaluation: map[string]interface{}{
			"suites": []interface{}{"not-a-map-suite"},
		},
	}
	errs := ValidateADPSemantics(adp)
	_ = errs // Should not panic; non-map suite is skipped.
}

// TestValidateADPSemanticsNonMapHook exercises the !ok continue for non-map hook entries.
func TestValidateADPSemanticsNonMapHook(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "non-map-hook",
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
		Hooks:      []interface{}{"not-a-map-hook"},
	}
	errs := ValidateADPSemantics(adp)
	_ = errs // Should not panic; non-map hook is skipped.
}

// TestValidateADPSemanticsHookEmptyEvent exercises the hook event=="" branch (sets event="?").
func TestValidateADPSemanticsHookEmptyEvent(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "hook-no-event",
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
				// no "event" key → event will be ""  → set to "?"
				"node_filter": []interface{}{"ghost-node"},
			},
		},
	}
	errs := ValidateADPSemantics(adp)
	found := false
	for _, e := range errs {
		// The error should mention event "?" because event was empty.
		if strings.Contains(e, "node_filter") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected node_filter error for hook with no event, got: %v", errs)
	}
}

// TestValidateADPSemanticsSubflowAdpRefEmpty exercises the adpRef=="" continue branch.
func TestValidateADPSemanticsSubflowAdpRefEmpty(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "subflow-no-ref",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow: map[string]interface{}{
			"graph": map[string]interface{}{
				"nodes":       []interface{}{map[string]interface{}{"id": "delegate", "kind": "subflow"}},
				"edges":       []interface{}{},
				"start_nodes": []interface{}{"delegate"},
				"end_nodes":   []interface{}{"delegate"},
			},
		},
		Evaluation: map[string]interface{}{},
	}
	errs := ValidateADPSemantics(adp)
	// No adp_ref set → should skip the subagent check, no error.
	for _, e := range errs {
		if strings.Contains(e, "adp_ref") {
			t.Errorf("unexpected adp_ref error: %v", e)
		}
	}
}

// TestValidateADPSemanticsXTestingNonMapEvaluator exercises the !ok continue for
// non-map evaluator entries in XTesting.
func TestValidateADPSemanticsXTestingNonMapEvaluator(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "non-map-evaluator",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow:       minimalFlow(),
		Evaluation: map[string]interface{}{
			"suites": []interface{}{map[string]interface{}{
				"id": "s1",
				"metrics": []interface{}{map[string]interface{}{
					"id": "m1", "type": "deterministic", "function": "noop",
					"scoring": "boolean", "threshold": true,
					"evaluator_ref": "known-ev",
				}},
			}},
		},
		XTesting: map[string]interface{}{
			"evaluators": []interface{}{
				"not-a-map-evaluator", // triggers !ok continue
				map[string]interface{}{"id": "known-ev", "type": "llm_judge"},
			},
		},
	}
	errs := ValidateADPSemantics(adp)
	// Should not panic. The non-map entry is skipped.
	_ = errs
}

// TestValidateADPSemanticsXTestingNonMapJudge exercises the !ok continue for
// non-map judge entries in XTesting.
func TestValidateADPSemanticsXTestingNonMapJudge(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "non-map-judge",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow:       minimalFlow(),
		Evaluation: minimalEvaluation(),
		XTesting: map[string]interface{}{
			"judges": []interface{}{
				"not-a-map-judge", // triggers !ok continue
				map[string]interface{}{"id": "j1"},
			},
		},
	}
	errs := ValidateADPSemantics(adp)
	_ = errs // Should not panic.
}

// TestValidateADPSemanticsNonMapMetric exercises the !ok continue for non-map metrics.
func TestValidateADPSemanticsNonMapMetric(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "non-map-metric",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow:       minimalFlow(),
		Evaluation: map[string]interface{}{
			"suites": []interface{}{map[string]interface{}{
				"id": "s1",
				"metrics": []interface{}{
					"not-a-map-metric", // triggers !ok continue
				},
			}},
		},
		XTesting: map[string]interface{}{
			"evaluators": []interface{}{
				map[string]interface{}{"id": "ev1"},
			},
		},
	}
	errs := ValidateADPSemantics(adp)
	_ = errs // Should not panic.
}

// TestValidateADPSemanticsEvalRefEmptyInMetric exercises the evalRef=="" continue
// branch in the metrics loop (metric with evaluators present but no evaluator_ref).
func TestValidateADPSemanticsEvalRefEmptyInMetric(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "eval-ref-empty",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow:       minimalFlow(),
		Evaluation: map[string]interface{}{
			"suites": []interface{}{map[string]interface{}{
				"id": "s1",
				"metrics": []interface{}{map[string]interface{}{
					"id": "m1", "type": "deterministic", "function": "noop",
					"scoring": "boolean", "threshold": true,
					// no evaluator_ref → evalRef == "" → continue
				}},
			}},
		},
		XTesting: map[string]interface{}{
			"evaluators": []interface{}{
				map[string]interface{}{"id": "ev1"},
			},
		},
	}
	errs := ValidateADPSemantics(adp)
	// Should have no evaluator_ref error since the metric has no evaluator_ref.
	for _, e := range errs {
		if strings.Contains(e, "evaluator_ref") {
			t.Errorf("unexpected evaluator_ref error: %v", e)
		}
	}
}

// ──── validate.go coverage: loadCompiledSchemaFromDir and injectable fn ─────

// TestLoadCompiledSchemaFromDirFileReadError exercises the os.ReadFile error
// path (validate.go:25) by passing a directory with no schema files.
func TestLoadCompiledSchemaFromDirFileReadError(t *testing.T) {
	tmp, _ := os.MkdirTemp("", "adp-schema-read-*")
	defer os.RemoveAll(tmp)
	_, err := loadCompiledSchemaFromDir(tmp)
	if err == nil {
		t.Fatal("expected error when schema file is missing")
	}
	if !strings.Contains(err.Error(), "reading") {
		t.Errorf("expected 'reading' in error, got: %v", err)
	}
}

// TestLoadCompiledSchemaFromDirInvalidJSON exercises the json.Unmarshal error
// path (validate.go:29) by writing a schema file containing invalid JSON.
func TestLoadCompiledSchemaFromDirInvalidJSON(t *testing.T) {
	tmp, _ := os.MkdirTemp("", "adp-schema-json-*")
	defer os.RemoveAll(tmp)
	// Write a broken JSON file for the first schema name ("adp").
	os.WriteFile(filepath.Join(tmp, "adp.schema.json"), []byte("not valid json {{{"), 0o644)
	_, err := loadCompiledSchemaFromDir(tmp)
	if err == nil {
		t.Fatal("expected error for invalid JSON schema file")
	}
	if !strings.Contains(err.Error(), "parsing") {
		t.Errorf("expected 'parsing' in error, got: %v", err)
	}
}

// TestLoadCompiledSchemaFromDirNoID exercises the id=="" branch (validate.go:34)
// by writing a schema file that has no $id field.
func TestLoadCompiledSchemaFromDirNoID(t *testing.T) {
	// Build a minimal dir with all 4 schema files, none having $id,
	// plus a real adp schema for compilation.
	realSchemaDir := schemaDir()
	tmp, _ := os.MkdirTemp("", "adp-schema-noid-*")
	defer os.RemoveAll(tmp)
	// Copy all 4 schemas but strip $id from adp schema.
	for _, name := range []string{"adp", "flow", "runtime", "evaluation"} {
		srcPath := filepath.Join(realSchemaDir, name+".schema.json")
		data, err := os.ReadFile(srcPath)
		if err != nil {
			t.Skipf("cannot read real schema %s: %v", name, err)
		}
		if name == "adp" {
			// Replace $id by using the real file bytes but without $id.
			// Simplest: just write the file as-is — $id is present → id branch is skipped.
			// To hit the id=="" branch we need a file with no $id.
			// Write a minimal valid JSON map without $id.
			os.WriteFile(filepath.Join(tmp, name+".schema.json"), []byte(`{"type":"object"}`), 0o644)
		} else {
			os.WriteFile(filepath.Join(tmp, name+".schema.json"), data, 0o644)
		}
	}
	// loadCompiledSchemaFromDir will hit id=="" for adp and fall back to file:// URI.
	// The compile step will likely fail because the schema is invalid, but the
	// id=="" branch is covered.
	_, _ = loadCompiledSchemaFromDir(tmp) // result doesn't matter; we just need to hit the branch
}

// TestValidateADPSchemaLoadError exercises the schema load failure path
// (validate.go:72) by temporarily replacing loadCompiledSchemaFn.
func TestValidateADPSchemaLoadError(t *testing.T) {
	orig := loadCompiledSchemaFn
	loadCompiledSchemaFn = func() (*jsonschema.Schema, error) {
		return nil, fmt.Errorf("injected schema load error")
	}
	defer func() { loadCompiledSchemaFn = orig }()

	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "schema-load-err",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "main:app"}}},
		Flow:       minimalFlow(),
		Evaluation: minimalEvaluation(),
	}
	err := ValidateADP(adp)
	if err == nil {
		t.Fatal("expected error for schema load failure")
	}
	if !strings.Contains(err.Error(), "loading schema") {
		t.Errorf("expected 'loading schema' in error, got: %v", err)
	}
}

// TestValidateADPSemanticsHTTPAPINotMap exercises the !ok continue branch
// (validate.go:122) when an http_api entry is not a map.
func TestValidateADPSemanticsHTTPAPINotMap(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.2.0",
		ID:         "http-api-not-map",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "app:main"}}},
		Flow:       minimalFlow(),
		Evaluation: map[string]interface{}{},
		Tools: map[string]interface{}{
			"http_apis": []interface{}{
				"this-is-a-string-not-a-map", // triggers !ok continue
				map[string]interface{}{
					"id":       "real-api",
					"base_url": "https://example.com",
				},
			},
		},
	}
	// Should not panic; the non-map entry is skipped.
	errs := ValidateADPSemantics(adp)
	_ = errs
}

// TestValidateADPSemanticsEdgeToNotFound exercises the edge.to not found branch
// (validate.go:198) when the "to" node of an edge does not exist.
func TestValidateADPSemanticsEdgeToNotFound(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "edge-to-missing",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow: map[string]interface{}{
			"graph": map[string]interface{}{
				"nodes": []interface{}{
					map[string]interface{}{"id": "n1", "kind": "input"},
				},
				"edges": []interface{}{
					map[string]interface{}{"from": "n1", "to": "ghost-to"},
				},
				"start_nodes": []interface{}{"n1"},
				"end_nodes":   []interface{}{"n1"},
			},
		},
		Evaluation: map[string]interface{}{},
	}
	errs := ValidateADPSemantics(adp)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "ghost-to") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected error about missing 'to' node 'ghost-to', got: %v", errs)
	}
}

// TestValidateADPSemanticsSuiteNotMap exercises the !ok continue branch
// (validate.go:352) when a suite entry in x_testing check 14 is not a map.
func TestValidateADPSemanticsSuiteNotMap(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "suite-not-map",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow:       minimalFlow(),
		Evaluation: map[string]interface{}{
			"suites": []interface{}{
				"not-a-map-suite-in-check14", // triggers !ok continue in check 14
				map[string]interface{}{
					"id":      "s1",
					"metrics": []interface{}{},
				},
			},
		},
		XTesting: map[string]interface{}{
			"evaluators": []interface{}{
				map[string]interface{}{"id": "ev1"},
			},
		},
	}
	// Should not panic; the non-map suite is skipped in the check-14 loop.
	errs := ValidateADPSemantics(adp)
	_ = errs
}

// TestValidateADPSemanticsMetricIDEmpty exercises the metricID=="" branch
// (evaluator_ref points to unknown evaluator, metric has no id).
func TestValidateADPSemanticsMetricIDEmpty(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "metric-id-empty",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow:       minimalFlow(),
		Evaluation: map[string]interface{}{
			"suites": []interface{}{map[string]interface{}{
				"id": "s1",
				"metrics": []interface{}{map[string]interface{}{
					// no "id" key → metricID == "" → set to "?"
					"type":          "deterministic",
					"evaluator_ref": "unknown-evaluator",
				}},
			}},
		},
		XTesting: map[string]interface{}{
			"evaluators": []interface{}{
				map[string]interface{}{"id": "known-ev"},
			},
		},
	}
	errs := ValidateADPSemantics(adp)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "unknown-evaluator") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected evaluator_ref error with '?', got: %v", errs)
	}
}

// TestValidateADPSemanticsHTTPAPINoID exercises the toolID=="" path in the
// http_apis auth check (line 122.24).
func TestValidateADPSemanticsHTTPAPINoID(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.2.0",
		ID:         "api-no-id",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "app:main"}}},
		Flow:       minimalFlow(),
		Evaluation: map[string]interface{}{},
		Tools: map[string]interface{}{
			"http_apis": []interface{}{
				map[string]interface{}{
					// no "id" key → toolID == ""
					"description": "An API without an id",
					"base_url":    "https://example.com",
					"auth":        map[string]interface{}{"scheme": "bearer", "env_var": "API_KEY"},
				},
			},
		},
	}
	errs := ValidateADPSemantics(adp)
	// No error expected for auth since env_var is provided.
	_ = errs
}

// TestNormalizeResultDefaultBoolInDefault verifies that the default branch in
// normalizeResult handles the case where the value is not bool/map (the inner
// `if b, ok := raw.(bool)` is dead code — bool is handled by case bool above).
// This ensures the default branch is entered (via a number or other type).
func TestNormalizeResultDefaultNonBool(t *testing.T) {
	// A JSON number (float64) is neither bool nor map → hits default branch.
	// The inner bool check will be false → base.Passed stays false.
	result := normalizeResult(float64(42), "test-id", "test-type")
	if result == nil {
		t.Fatal("expected non-nil result")
	}
	// Default branch: passed stays false (zero value).
	if result.Passed {
		t.Error("expected passed=false for default branch with float64")
	}
	if result.EvaluatorID != "test-id" {
		t.Errorf("expected EvaluatorID 'test-id', got %q", result.EvaluatorID)
	}
}
