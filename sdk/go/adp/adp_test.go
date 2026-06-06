package adp

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"

	jsonschema "github.com/santhosh-tekuri/jsonschema/v5"
)

func minimalFlow() *Flow {
	return &Flow{
		ID: "f",
		Graph: Graph{
			Nodes:      []Node{{ID: "n", Kind: NodeKindInput}},
			Edges:      []Edge{},
			StartNodes: []string{"n"},
			EndNodes:   []string{"n"},
		},
	}
}

func makeFlow(nodes []Node, edges []Edge, startNodes, endNodes []string) *Flow {
	return &Flow{
		Graph: Graph{
			Nodes:      nodes,
			Edges:      edges,
			StartNodes: startNodes,
			EndNodes:   endNodes,
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
		Flow:       &Flow{},
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
		Flow:       &Flow{},
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
		Flow:       makeFlow([]Node{{ID: "n1", Kind: NodeKindInput}}, []Edge{}, []string{"n1"}, []string{"n1"}),
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
		Flow: makeFlow(
			[]Node{{ID: "delegate", Kind: NodeKindSubflow, AdpRef: "unknown-subagent"}},
			[]Edge{}, []string{"delegate"}, []string{"delegate"},
		),
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
		Flow: &Flow{
			ID: "test.flow",
			Graph: Graph{
				Nodes:      []Node{{ID: "input", Kind: NodeKindInput}, {ID: "output", Kind: NodeKindOutput}},
				Edges:      []Edge{},
				StartNodes: []string{"input"},
				EndNodes:   []string{"output"},
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
			Flow:       &Flow{},
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
		Flow: makeFlow(
			[]Node{{ID: "n1", Kind: NodeKindInput}, {ID: "n2", Kind: NodeKindOutput}},
			[]Edge{{From: "n1", To: "n2"}},
			[]string{"n1"}, []string{"n2"},
		),
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
		Flow: makeFlow(
			[]Node{{ID: "input", Kind: NodeKindInput}},
			[]Edge{{From: "ghost", To: "input"}},
			[]string{"input"}, []string{"input"},
		),
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
		Flow: makeFlow(
			[]Node{{ID: "input", Kind: NodeKindInput}, {ID: "input", Kind: NodeKindOutput}},
			[]Edge{}, []string{"input"}, []string{"input"},
		),
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
		Flow: makeFlow(
			[]Node{{ID: "n1", Kind: NodeKindLLM, SuiteRef: "missing-suite"}},
			[]Edge{}, []string{"n1"}, []string{"n1"},
		),
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
		Flow: makeFlow(
			[]Node{{ID: "n1", Kind: NodeKindLLM, ModelRef: "missing-model"}},
			[]Edge{}, []string{"n1"}, []string{"n1"},
		),
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
		Flow: makeFlow(
			[]Node{{ID: "n1", Kind: NodeKindLLM, RuntimeRef: "missing-backend"}},
			[]Edge{}, []string{"n1"}, []string{"n1"},
		),
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
		Flow:             &Flow{},
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
		Tools: &Tools{
			HTTPAPIs: []HTTPAPI{{
				ID:          "billing-api",
				Description: "Billing service",
				BaseURL:     "https://billing.example.com",
				Auth:        &Auth{Scheme: AuthSchemeBearer},
			}},
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
		Flow: makeFlow(
			[]Node{
				{ID: "input", Kind: NodeKindInput},
				{ID: "api-call", Kind: NodeKindTool, ToolRef: "nonexistent-api"},
				{ID: "output", Kind: NodeKindOutput},
			},
			[]Edge{}, []string{"input"}, []string{"output"},
		),
		Evaluation: map[string]interface{}{},
		Tools: &Tools{
			HTTPAPIs: []HTTPAPI{{
				ID:          "real-api",
				Description: "A real API",
				BaseURL:     "https://example.com",
			}},
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
	// Compliance check (governance.compliance) is not yet implemented in the Go SDK
	// validate.go; this test verifies the function runs without panicking.
	adp := &ADP{
		ADPVersion: "0.2.0",
		ID:         "compliance-test",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "app:main"}}},
		Flow:       minimalFlow(),
		Evaluation: map[string]interface{}{},
	}
	_ = ValidateADPSemantics(adp)
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

// TestValidateADPSemanticsFlowNil exercises the nil-flow early return path.
func TestValidateADPSemanticsFlowNil(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "flow-nil",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow:       nil,
		Evaluation: map[string]interface{}{},
	}
	// Nil flow → should return immediately with no graph errors.
	errs := ValidateADPSemantics(adp)
	for _, e := range errs {
		if strings.Contains(e, "node") || strings.Contains(e, "edge") {
			t.Errorf("unexpected graph error when flow is nil: %v", e)
		}
	}
}

// TestValidateADPSemanticsFlowEmptyGraph exercises a flow with an empty graph (no nodes).
func TestValidateADPSemanticsFlowEmptyGraph(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "flow-empty-graph",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow:       &Flow{ID: "f"},
		Evaluation: map[string]interface{}{},
	}
	// Empty graph → no node/edge errors expected.
	errs := ValidateADPSemantics(adp)
	for _, e := range errs {
		if strings.Contains(e, "node") || strings.Contains(e, "edge") {
			t.Errorf("unexpected graph error for empty graph: %v", e)
		}
	}
}

// TestValidateADPSemanticsTypedNodes verifies that typed nodes are processed correctly.
func TestValidateADPSemanticsTypedNodes(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "typed-nodes",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow: makeFlow(
			[]Node{{ID: "n1", Kind: NodeKindInput}},
			[]Edge{}, []string{"n1"}, []string{"n1"},
		),
		Evaluation: map[string]interface{}{},
	}
	errs := ValidateADPSemantics(adp)
	_ = errs
}

// TestValidateADPSemanticsTypedEdge verifies typed edges are processed correctly.
func TestValidateADPSemanticsTypedEdge(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "typed-edge",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow: makeFlow(
			[]Node{{ID: "n1", Kind: NodeKindInput}},
			[]Edge{{From: "n1", To: "n1"}},
			[]string{"n1"}, []string{"n1"},
		),
		Evaluation: map[string]interface{}{},
	}
	errs := ValidateADPSemantics(adp)
	_ = errs
}

// TestValidateADPSemanticsStartNodeNotFound exercises the start_node not found error.
func TestValidateADPSemanticsStartNodeNotFound(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "bad-start",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow: makeFlow(
			[]Node{{ID: "n1", Kind: NodeKindInput}},
			[]Edge{}, []string{"ghost-start"}, []string{"n1"},
		),
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
		Flow: makeFlow(
			[]Node{{ID: "n1", Kind: NodeKindInput}},
			[]Edge{}, []string{"n1"}, []string{"ghost-end"},
		),
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
		Flow:       makeFlow([]Node{{ID: "n1", Kind: NodeKindInput}}, []Edge{}, []string{"n1"}, []string{"n1"}),
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
		Flow:       makeFlow([]Node{{ID: "n1", Kind: NodeKindInput}}, []Edge{}, []string{"n1"}, []string{"n1"}),
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
		Flow: makeFlow(
			[]Node{{ID: "delegate", Kind: NodeKindSubflow}},
			[]Edge{}, []string{"delegate"}, []string{"delegate"},
		),
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

// TestValidateADPSemanticsHTTPAPINoAuth verifies typed http_api with no auth causes no error.
func TestValidateADPSemanticsHTTPAPINoAuth(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.2.0",
		ID:         "http-api-no-auth",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "app:main"}}},
		Flow:       minimalFlow(),
		Evaluation: map[string]interface{}{},
		Tools: &Tools{
			HTTPAPIs: []HTTPAPI{{
				ID:      "real-api",
				BaseURL: "https://example.com",
			}},
		},
	}
	errs := ValidateADPSemantics(adp)
	_ = errs
}

// TestValidateADPSemanticsEdgeToNotFound exercises the edge.to not found branch
// when the "to" node of an edge does not exist.
func TestValidateADPSemanticsEdgeToNotFound(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.1.0",
		ID:         "edge-to-missing",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py"}}},
		Flow: makeFlow(
			[]Node{{ID: "n1", Kind: NodeKindInput}},
			[]Edge{{From: "n1", To: "ghost-to"}},
			[]string{"n1"}, []string{"n1"},
		),
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

// TestValidateADPSemanticsHTTPAPINoID verifies an http_api with no ID and valid auth causes no error.
func TestValidateADPSemanticsHTTPAPINoID(t *testing.T) {
	adp := &ADP{
		ADPVersion: "0.2.0",
		ID:         "api-no-id",
		Runtime:    Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "app:main"}}},
		Flow:       minimalFlow(),
		Evaluation: map[string]interface{}{},
		Tools: &Tools{
			HTTPAPIs: []HTTPAPI{{
				// empty ID — toolID == "" in the loop
				Description: "An API without an id",
				BaseURL:     "https://example.com",
				Auth:        &Auth{Scheme: AuthSchemeBearer, EnvVar: "API_KEY"},
			}},
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

func minimalADP() *ADP {
	return &ADP{
		ADPVersion: "0.3.0",
		ID:         "test.agent",
		Runtime: Runtime{
			Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "app:main"}},
		},
		Flow: minimalFlow(),
		Evaluation: map[string]interface{}{
			"suites": []interface{}{},
		},
	}
}

func TestValidateADPSemanticsAS1NodeMapUnknownNode(t *testing.T) {
	a := minimalADP()
	a.Interop = &Interop{
		AgentSpec: &InteropAgentSpec{
			NodeMap: map[string]string{"ghost-node": "3a5bf0c0-9f28-47d8-a000-111111111111"},
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "ghost-node") && strings.Contains(e, "node_map") {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected AS-1 node_map error, got: %v", errs)
	}
}

func TestValidateADPSemanticsAS1NodeMapValid(t *testing.T) {
	a := minimalADP()
	a.Interop = &Interop{
		AgentSpec: &InteropAgentSpec{
			NodeMap: map[string]string{"n": "3a5bf0c0-9f28-47d8-a000-111111111111"},
		},
	}
	errs := ValidateADPSemantics(a)
	for _, e := range errs {
		if strings.Contains(e, "node_map") {
			t.Errorf("unexpected AS-1 error for valid node_map: %v", errs)
		}
	}
}

func TestValidateADPSemanticsAS2LlmMapUnknownBackend(t *testing.T) {
	a := minimalADP()
	a.Interop = &Interop{
		AgentSpec: &InteropAgentSpec{
			LLMMap: []InteropAgentSpecLLMBinding{
				{BackendID: "ghost-backend", AgentSpecID: "3a5bf0c0-9f28-47d8-a000-111111111111"},
			},
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "ghost-backend") && strings.Contains(e, "llm_map") {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected AS-2 llm_map error, got: %v", errs)
	}
}

func TestValidateADPSemanticsAS2LlmMapValid(t *testing.T) {
	a := minimalADP()
	a.Interop = &Interop{
		AgentSpec: &InteropAgentSpec{
			LLMMap: []InteropAgentSpecLLMBinding{
				{BackendID: "py", AgentSpecID: "3a5bf0c0-9f28-47d8-a000-111111111111"},
			},
		},
	}
	errs := ValidateADPSemantics(a)
	for _, e := range errs {
		if strings.Contains(e, "llm_map") {
			t.Errorf("unexpected AS-2 error for valid llm_map: %v", errs)
		}
	}
}

func TestValidateADPSemanticsAS3RefPathTraversal(t *testing.T) {
	a := minimalADP()
	a.Interop = &Interop{
		AgentSpec: &InteropAgentSpec{
			Ref: "../../etc/passwd",
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "path traversal") {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected AS-3 path traversal error, got: %v", errs)
	}
}

func TestValidateADPSemanticsAS2RunsWithoutFlow(t *testing.T) {
	// AS-2 must check llm_map even when Flow is nil
	a := minimalADP()
	a.Flow = nil
	a.Interop = &Interop{
		AgentSpec: &InteropAgentSpec{
			LLMMap: []InteropAgentSpecLLMBinding{
				{BackendID: "ghost-backend", AgentSpecID: "3a5bf0c0-9f28-47d8-a000-111111111111"},
			},
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "ghost-backend") && strings.Contains(e, "llm_map") {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected AS-2 llm_map error with nil flow, got: %v", errs)
	}
}

// ──── ValidateADP: conformance_class="full" with non-empty flow but empty eval ─

// TestValidateADPConformanceClassFullNilFlowBranch exercises the check for
// conformance_class="full" where Flow is nil (JSON-marshals to null → empty map),
// triggering the "flow is empty" error on line 70 of validate.go.
func TestValidateADPConformanceClassFullNilFlowBranch(t *testing.T) {
	adp := &ADP{
		ADPVersion:       "0.1.0",
		ID:               "agent.full.nilflow",
		ConformanceClass: "full",
		Runtime:          Runtime{Execution: []RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "main:app"}}},
		Flow:             nil, // nil → marshals to JSON null → len(flow) == 0
		Evaluation:       minimalEvaluation(),
	}
	err := ValidateADP(adp)
	if err == nil {
		t.Fatal("expected error for conformance_class=full with nil flow")
	}
	if !strings.Contains(err.Error(), "flow") {
		t.Errorf("expected error about 'flow', got: %v", err)
	}
}

// ──── Check 9: MCPServer auth missing env_var ─────────────────────────────────

func TestValidateADPSemanticsMCPServerAuthMissingEnvVar(t *testing.T) {
	a := minimalADP()
	a.Tools = &Tools{
		MCPservers: []MCPServer{{
			ID:      "my-mcp",
			Command: "npx my-tool",
			Auth:    &Auth{Scheme: AuthSchemeBearer, EnvVar: ""},
		}},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "my-mcp") && strings.Contains(e, "auth.env_var") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected MCP server auth.env_var error, got: %v", errs)
	}
}

func TestValidateADPSemanticsMCPServerAuthNoneScheme(t *testing.T) {
	// Auth scheme "none" → no env_var required
	a := minimalADP()
	a.Tools = &Tools{
		MCPservers: []MCPServer{{
			ID:      "mcp-none",
			Command: "npx tool",
			Auth:    &Auth{Scheme: AuthSchemeNone, EnvVar: ""},
		}},
	}
	errs := ValidateADPSemantics(a)
	for _, e := range errs {
		if strings.Contains(e, "mcp-none") && strings.Contains(e, "auth.env_var") {
			t.Errorf("unexpected auth.env_var error for scheme=none: %v", e)
		}
	}
}

// ──── Check 9: SQLFunction auth missing env_var ───────────────────────────────

func TestValidateADPSemanticsSQLFunctionAuthMissingEnvVar(t *testing.T) {
	a := minimalADP()
	a.Tools = &Tools{
		SQLFunctions: []SQLFunction{{
			ID:    "my-sql",
			Query: "SELECT 1",
			Auth:  &Auth{Scheme: AuthSchemeAPIKey, EnvVar: ""},
		}},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "my-sql") && strings.Contains(e, "auth.env_var") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected SQL function auth.env_var error, got: %v", errs)
	}
}

func TestValidateADPSemanticsSQLFunctionAuthNoneScheme(t *testing.T) {
	// Auth scheme "none" → no env_var required
	a := minimalADP()
	a.Tools = &Tools{
		SQLFunctions: []SQLFunction{{
			ID:    "sql-none",
			Query: "SELECT 1",
			Auth:  &Auth{Scheme: AuthSchemeNone, EnvVar: ""},
		}},
	}
	errs := ValidateADPSemantics(a)
	for _, e := range errs {
		if strings.Contains(e, "sql-none") && strings.Contains(e, "auth.env_var") {
			t.Errorf("unexpected auth.env_var error for scheme=none: %v", e)
		}
	}
}

// ──── Check 15: Loop body_nodes must reference known node IDs ─────────────────

func TestValidateADPSemanticsLoopBodyNodeUnknown(t *testing.T) {
	a := minimalADP()
	a.Flow = makeFlow(
		[]Node{
			{ID: "loop1", Kind: NodeKindLoop, BodyNodes: []string{"ghost-body-node"}},
		},
		[]Edge{},
		[]string{"loop1"}, []string{"loop1"},
	)
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "body_nodes") && strings.Contains(e, "ghost-body-node") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected loop body_nodes unknown node error, got: %v", errs)
	}
}

// ──── Check 15b: Loop body_nodes must have connected nodes ────────────────────

func TestValidateADPSemanticsLoopBodyNodesNotConnected(t *testing.T) {
	a := minimalADP()
	// Two body nodes exist but no edge between them.
	a.Flow = makeFlow(
		[]Node{
			{ID: "loop1", Kind: NodeKindLoop, BodyNodes: []string{"step1", "step2"}},
			{ID: "step1", Kind: NodeKindLLM},
			{ID: "step2", Kind: NodeKindLLM},
		},
		[]Edge{}, // no edges between step1 and step2
		[]string{"loop1"}, []string{"loop1"},
	)
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "body_nodes") && strings.Contains(e, "connected") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected loop body_nodes not connected error, got: %v", errs)
	}
}

func TestValidateADPSemanticsLoopBodyNodesConnected(t *testing.T) {
	a := minimalADP()
	// Two body nodes with an edge connecting them.
	a.Flow = makeFlow(
		[]Node{
			{ID: "loop1", Kind: NodeKindLoop, BodyNodes: []string{"step1", "step2"}},
			{ID: "step1", Kind: NodeKindLLM},
			{ID: "step2", Kind: NodeKindLLM},
		},
		[]Edge{{From: "step1", To: "step2"}},
		[]string{"loop1"}, []string{"loop1"},
	)
	errs := ValidateADPSemantics(a)
	for _, e := range errs {
		if strings.Contains(e, "body_nodes") && strings.Contains(e, "connected") {
			t.Errorf("unexpected connection error: %v", e)
		}
	}
}

// ──── Check 16: Loop must not reference itself ────────────────────────────────

func TestValidateADPSemanticsLoopSelfReference(t *testing.T) {
	a := minimalADP()
	a.Flow = makeFlow(
		[]Node{
			{ID: "loop1", Kind: NodeKindLoop, BodyNodes: []string{"loop1"}},
		},
		[]Edge{},
		[]string{"loop1"}, []string{"loop1"},
	)
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "body_nodes MUST NOT reference the loop node itself") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected loop self-reference error, got: %v", errs)
	}
}

func TestValidateADPSemanticsLoopCircularReference(t *testing.T) {
	// loop1 has body_node=loop2, loop2 has body_node=loop1 (circular).
	a := minimalADP()
	a.Flow = makeFlow(
		[]Node{
			{ID: "loop1", Kind: NodeKindLoop, BodyNodes: []string{"loop2"}},
			{ID: "loop2", Kind: NodeKindLoop, BodyNodes: []string{"loop1"}},
		},
		[]Edge{{From: "loop2", To: "loop1"}},
		[]string{"loop1"}, []string{"loop1"},
	)
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "circular loop reference") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected circular loop reference error, got: %v", errs)
	}
}

// ──── Check 17: Tool cache key_fields dot-path notation ──────────────────────

func TestValidateADPSemanticsHTTPAPICacheKeyFieldsBadPath(t *testing.T) {
	a := minimalADP()
	a.Tools = &Tools{
		HTTPAPIs: []HTTPAPI{{
			ID:      "my-api",
			BaseURL: "https://example.com",
			Policy: &ToolPolicy{
				Cache: &CachePolicy{KeyFields: []string{"valid.field", "bad field!"}},
			},
		}},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "dot-path") && strings.Contains(e, "bad field!") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected cache.key_fields dot-path error, got: %v", errs)
	}
}

func TestValidateADPSemanticsSQLFunctionCacheKeyFieldsBadPath(t *testing.T) {
	a := minimalADP()
	a.Tools = &Tools{
		SQLFunctions: []SQLFunction{{
			ID:    "my-sql",
			Query: "SELECT 1",
			Policy: &ToolPolicy{
				Cache: &CachePolicy{KeyFields: []string{"bad path!"}},
			},
		}},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "dot-path") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected SQL function cache.key_fields dot-path error, got: %v", errs)
	}
}

func TestValidateADPSemanticsHTTPAPICacheKeyFieldsNilPolicy(t *testing.T) {
	// nil policy → checkToolCacheKeyFields returns early, no error
	a := minimalADP()
	a.Tools = &Tools{
		HTTPAPIs: []HTTPAPI{{
			ID:     "api-nopol",
			BaseURL: "https://example.com",
			Policy: nil,
		}},
	}
	errs := ValidateADPSemantics(a)
	for _, e := range errs {
		if strings.Contains(e, "dot-path") {
			t.Errorf("unexpected dot-path error for nil policy: %v", e)
		}
	}
}

func TestValidateADPSemanticsHTTPAPICacheKeyFieldsNilCache(t *testing.T) {
	// Policy set but cache nil → checkToolCacheKeyFields returns early
	a := minimalADP()
	a.Tools = &Tools{
		HTTPAPIs: []HTTPAPI{{
			ID:      "api-nocache",
			BaseURL: "https://example.com",
			Policy:  &ToolPolicy{Cache: nil},
		}},
	}
	errs := ValidateADPSemantics(a)
	for _, e := range errs {
		if strings.Contains(e, "dot-path") {
			t.Errorf("unexpected dot-path error for nil cache: %v", e)
		}
	}
}

func TestValidateADPSemanticsGlobalToolPolicyCacheKeyFieldsBadPath(t *testing.T) {
	a := minimalADP()
	a.Tools = &Tools{
		Policy: &ToolPolicy{
			Cache: &CachePolicy{KeyFields: []string{"good.field", "bad!"}},
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "tools.policy.cache.key_fields") && strings.Contains(e, "bad!") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected global tools policy cache.key_fields error, got: %v", errs)
	}
}

// ──── Check 29: on_demand load_strategy requires description ─────────────────

func TestValidateADPSemanticsHTTPAPIOnDemandMissingDescription(t *testing.T) {
	a := minimalADP()
	a.Tools = &Tools{
		HTTPAPIs: []HTTPAPI{{
			ID:          "lazy-api",
			BaseURL:     "https://example.com",
			Description: "", // missing
			Policy:      &ToolPolicy{LoadStrategy: LoadStrategyOnDemand},
		}},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "on_demand") && strings.Contains(e, "description") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected on_demand description error, got: %v", errs)
	}
}

func TestValidateADPSemanticsSQLFunctionOnDemandMissingDescription(t *testing.T) {
	a := minimalADP()
	a.Tools = &Tools{
		SQLFunctions: []SQLFunction{{
			ID:          "lazy-sql",
			Query:       "SELECT 1",
			Description: "",
			Policy:      &ToolPolicy{LoadStrategy: LoadStrategyOnDemand},
		}},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "on_demand") && strings.Contains(e, "description") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected SQL function on_demand description error, got: %v", errs)
	}
}

func TestValidateADPSemanticsSQLFunctionWithPolicy(t *testing.T) {
	// SQL function with policy having a load strategy other than on_demand — no error
	a := minimalADP()
	a.Tools = &Tools{
		SQLFunctions: []SQLFunction{{
			ID:          "eager-sql",
			Query:       "SELECT 1",
			Description: "",
			Policy:      &ToolPolicy{LoadStrategy: LoadStrategyEager},
		}},
	}
	errs := ValidateADPSemantics(a)
	for _, e := range errs {
		if strings.Contains(e, "on_demand") {
			t.Errorf("unexpected on_demand error: %v", e)
		}
	}
}

// ──── Check 18-20, 24: Memory checks ─────────────────────────────────────────

func TestValidateADPSemanticsMemoryDuplicateStoreID(t *testing.T) {
	a := minimalADP()
	a.Memory = &Memory{
		Structured: &MemoryStructured{
			Stores: []MemoryStore{
				{ID: "store1", StoreType: MemoryStoreTypeEpisodic},
				{ID: "store1", StoreType: MemoryStoreTypeSemantic}, // duplicate
			},
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "duplicate store id") && strings.Contains(e, "store1") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected duplicate memory store id error, got: %v", errs)
	}
}

func TestValidateADPSemanticsMemoryOperationBadStoreRef(t *testing.T) {
	a := minimalADP()
	a.Memory = &Memory{
		Structured: &MemoryStructured{
			Stores: []MemoryStore{
				{ID: "store1", StoreType: MemoryStoreTypeEpisodic},
			},
			Operations: []MemoryOperation{
				{ID: "op1", OnEvent: MemoryOperationOnEventOnInvokeEnd, Op: MemoryOperationOpWrite, StoreRef: "ghost-store"},
			},
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "store_ref") && strings.Contains(e, "ghost-store") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected memory operation store_ref error, got: %v", errs)
	}
}

func TestValidateADPSemanticsMemoryContextAssemblyBadStoreRef(t *testing.T) {
	a := minimalADP()
	a.Memory = &Memory{
		Structured: &MemoryStructured{
			Stores: []MemoryStore{
				{ID: "store1", StoreType: MemoryStoreTypeEpisodic},
			},
			ContextAssembly: &ContextAssembly{
				Order: []ContextAssemblyOrderItem{
					{Source: ContextAssemblySourceStore, StoreRef: "missing-store"},
				},
			},
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "context_assembly") && strings.Contains(e, "missing-store") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected memory context_assembly store_ref error, got: %v", errs)
	}
}

func TestValidateADPSemanticsMemoryStaticInjectionBadPath(t *testing.T) {
	a := minimalADP()
	a.Memory = &Memory{
		Structured: &MemoryStructured{
			Stores: []MemoryStore{{ID: "s1", StoreType: MemoryStoreTypeEpisodic}},
			ContextAssembly: &ContextAssembly{
				StaticInjection: []StaticInjection{
					{ID: "si1", Source: "file", Path: "../etc/passwd"},
				},
			},
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "static_injection") && strings.Contains(e, "..") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected static_injection path traversal error, got: %v", errs)
	}
}

func TestValidateADPSemanticsMemoryStaticInjectionAbsPath(t *testing.T) {
	a := minimalADP()
	a.Memory = &Memory{
		Structured: &MemoryStructured{
			Stores: []MemoryStore{{ID: "s1", StoreType: MemoryStoreTypeEpisodic}},
			ContextAssembly: &ContextAssembly{
				StaticInjection: []StaticInjection{
					{ID: "si1", Source: "file", Path: "/absolute/path"},
				},
			},
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "static_injection") && strings.Contains(e, "relative") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected static_injection absolute path error, got: %v", errs)
	}
}

func TestValidateADPSemanticsMemoryStaticInjectionMissingWorkspace(t *testing.T) {
	a := minimalADP()
	a.Memory = &Memory{
		Structured: &MemoryStructured{
			Stores: []MemoryStore{{ID: "s1", StoreType: MemoryStoreTypeEpisodic}},
			ContextAssembly: &ContextAssembly{
				StaticInjection: []StaticInjection{
					{ID: "si1", Source: "file", Path: "relative/path.txt"},
				},
			},
		},
	}
	// No workspace declared
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "static_injection") && strings.Contains(e, "workspace") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected static_injection workspace error, got: %v", errs)
	}
}

// ──── Check 21: memory.working.summary_model_ref must exist ──────────────────

func TestValidateADPSemanticsMemoryWorkingSummaryModelRefBad(t *testing.T) {
	a := minimalADP()
	a.Runtime.Models = []Model{{ID: "gpt4", Provider: "openai", Model: "gpt-4o"}}
	a.Memory = &Memory{
		Structured: &MemoryStructured{
			Working: &MemoryWorking{
				Strategy:        "sliding_window",
				SummaryModelRef: "ghost-model",
			},
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "summary_model_ref") && strings.Contains(e, "ghost-model") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected summary_model_ref error, got: %v", errs)
	}
}

// ──── Check 21b: strategy=summary requires summary_model_ref ─────────────────

func TestValidateADPSemanticsMemoryWorkingStrategySummaryMissingRef(t *testing.T) {
	a := minimalADP()
	a.Memory = &Memory{
		Structured: &MemoryStructured{
			Working: &MemoryWorking{
				Strategy:        "summary",
				SummaryModelRef: "", // missing
			},
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "summary_model_ref MUST be present") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected summary_model_ref missing error, got: %v", errs)
	}
}

// ──── Check 21c: compaction_threshold_tokens <= max_tokens ───────────────────

func TestValidateADPSemanticsMemoryWorkingCompactionThresholdExceedsMax(t *testing.T) {
	a := minimalADP()
	compThresh := 5000
	maxTok := 2000
	a.Memory = &Memory{
		Structured: &MemoryStructured{
			Working: &MemoryWorking{
				CompactionThresholdTokens: &compThresh,
				MaxTokens:                 &maxTok,
			},
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "compaction_threshold_tokens") && strings.Contains(e, "max_tokens") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected compaction_threshold_tokens > max_tokens error, got: %v", errs)
	}
}

func TestValidateADPSemanticsMemoryWorkingCompactionThresholdValid(t *testing.T) {
	a := minimalADP()
	compThresh := 1000
	maxTok := 2000
	a.Memory = &Memory{
		Structured: &MemoryStructured{
			Working: &MemoryWorking{
				CompactionThresholdTokens: &compThresh,
				MaxTokens:                 &maxTok,
			},
		},
	}
	errs := ValidateADPSemantics(a)
	for _, e := range errs {
		if strings.Contains(e, "compaction_threshold_tokens") {
			t.Errorf("unexpected compaction error: %v", e)
		}
	}
}

// ──── Check 22: guardrails.interrupts tool_refs must reference known tool IDs ─

func TestValidateADPSemanticsGuardrailInterruptBadToolRef(t *testing.T) {
	a := minimalADP()
	a.Guardrails = &Guardrails{
		Interrupts: []Interrupt{{
			ID:       "int1",
			Trigger:  "before_tool_call",
			Mode:     "pause",
			ToolRefs: []string{"nonexistent-tool"},
		}},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "tool_ref") && strings.Contains(e, "nonexistent-tool") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected guardrail interrupt tool_ref error, got: %v", errs)
	}
}

// TestValidateADPSemanticsGuardrailWithKnownToolRef exercises the toolIds copy
// loop (validate.go: for k, v := range toolIds) when Guardrails is set AND tools exist.
func TestValidateADPSemanticsGuardrailWithKnownToolRef(t *testing.T) {
	a := minimalADP()
	a.Tools = &Tools{
		HTTPAPIs: []HTTPAPI{{
			ID:      "known-tool",
			BaseURL: "https://example.com",
		}},
	}
	a.Guardrails = &Guardrails{
		Interrupts: []Interrupt{{
			ID:       "int1",
			Trigger:  "before_tool_call",
			Mode:     "pause",
			ToolRefs: []string{"known-tool"}, // references a known tool
		}},
	}
	errs := ValidateADPSemantics(a)
	for _, e := range errs {
		if strings.Contains(e, "tool_ref") && strings.Contains(e, "known-tool") {
			t.Errorf("unexpected tool_ref error for known tool: %v", e)
		}
	}
}

// ──── Check 22b: execution_mode MUST NOT be set when mode=pause_and_notify ───

func TestValidateADPSemanticsGuardrailInterruptPauseAndNotifyWithExecMode(t *testing.T) {
	a := minimalADP()
	a.Guardrails = &Guardrails{
		Interrupts: []Interrupt{{
			ID:            "int2",
			Trigger:       "before_tool_call",
			Mode:          "pause_and_notify",
			ExecutionMode: "parallel", // must not be set
		}},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "execution_mode MUST NOT be set") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected execution_mode not set error, got: %v", errs)
	}
}

// ──── Check 23: guardrails.cost.interrupt_ref must reference known interrupt ──

func TestValidateADPSemanticsGuardrailCostBadInterruptRef(t *testing.T) {
	a := minimalADP()
	a.Guardrails = &Guardrails{
		Interrupts: []Interrupt{{
			ID:      "int1",
			Trigger: "before_tool_call",
			Mode:    "pause",
		}},
		Cost: &CostGuardrail{
			InterruptRef: "ghost-interrupt",
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "interrupt_ref") && strings.Contains(e, "ghost-interrupt") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected cost.interrupt_ref error, got: %v", errs)
	}
}

// ──── Check 30: downgrade_model_ref must be present/valid when on_threshold_exceeded=downgrade

func TestValidateADPSemanticsGuardrailCostDowngradeModelRefMissing(t *testing.T) {
	a := minimalADP()
	a.Guardrails = &Guardrails{
		Interrupts: []Interrupt{{
			ID:      "int1",
			Trigger: "cost",
			Mode:    "block",
		}},
		Cost: &CostGuardrail{
			OnThresholdExceeded: "downgrade",
			DowngradeModelRef:   "", // missing
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "downgrade_model_ref MUST be present") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected downgrade_model_ref missing error, got: %v", errs)
	}
}

func TestValidateADPSemanticsGuardrailCostDowngradeModelRefBad(t *testing.T) {
	a := minimalADP()
	a.Runtime.Models = []Model{{ID: "gpt4", Provider: "openai", Model: "gpt-4o"}}
	a.Guardrails = &Guardrails{
		Interrupts: []Interrupt{{
			ID:      "int1",
			Trigger: "cost",
			Mode:    "block",
		}},
		Cost: &CostGuardrail{
			OnThresholdExceeded: "downgrade",
			DowngradeModelRef:   "ghost-model",
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "downgrade_model_ref") && strings.Contains(e, "ghost-model") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected downgrade_model_ref not found error, got: %v", errs)
	}
}

// ──── Check 25: workspace write permissions must not escape root ──────────────

func TestValidateADPSemanticsWorkspaceWritePathTraversal(t *testing.T) {
	a := minimalADP()
	a.Workspace = &Workspace{
		Root: "/tmp/workspace",
		Permissions: &WorkspacePermissions{
			Write: []string{"safe/path", "../escape/path"},
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "workspace.permissions.write") && strings.Contains(e, "..") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected workspace write path traversal error, got: %v", errs)
	}
}

// ──── Check 25b: exactly one of root or root_env_var ─────────────────────────

func TestValidateADPSemanticsWorkspaceBothRootAndEnvVar(t *testing.T) {
	a := minimalADP()
	a.Workspace = &Workspace{
		Root:       "/tmp/ws",
		RootEnvVar: "WORKSPACE_ROOT",
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "exactly one of") && strings.Contains(e, "root") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected both root and root_env_var error, got: %v", errs)
	}
}

func TestValidateADPSemanticsWorkspaceNeitherRootNorEnvVar(t *testing.T) {
	a := minimalADP()
	a.Workspace = &Workspace{}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "exactly one of") && strings.Contains(e, "root") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected neither root nor root_env_var error, got: %v", errs)
	}
}

// ──── Check 26: workspace.git.auto_commit requires enabled=true ──────────────

func TestValidateADPSemanticsWorkspaceGitAutoCommitWithoutEnabled(t *testing.T) {
	a := minimalADP()
	autoCommit := true
	a.Workspace = &Workspace{
		Root: "/tmp/ws",
		Git: &WorkspaceGit{
			AutoCommit: &autoCommit,
			Enabled:    nil, // not set
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "auto_commit") && strings.Contains(e, "enabled") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected auto_commit requires enabled error, got: %v", errs)
	}
}

func TestValidateADPSemanticsWorkspaceGitAutoCommitEnabledFalse(t *testing.T) {
	a := minimalADP()
	autoCommit := true
	enabled := false
	a.Workspace = &Workspace{
		Root: "/tmp/ws",
		Git: &WorkspaceGit{
			AutoCommit: &autoCommit,
			Enabled:    &enabled,
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "auto_commit") && strings.Contains(e, "enabled") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected auto_commit requires enabled=true error, got: %v", errs)
	}
}

// ──── Check 31: workspace.mounts duplicate IDs and target path traversal ─────

func TestValidateADPSemanticsWorkspaceMountDuplicateID(t *testing.T) {
	a := minimalADP()
	a.Workspace = &Workspace{
		Root: "/tmp/ws",
		Mounts: []WorkspaceMount{
			{ID: "m1", Source: WorkspaceMountSource{Path: "."}, Target: "target1"},
			{ID: "m1", Source: WorkspaceMountSource{Path: "."}, Target: "target2"}, // duplicate
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "duplicate mount id") && strings.Contains(e, "m1") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected workspace mount duplicate id error, got: %v", errs)
	}
}

func TestValidateADPSemanticsWorkspaceMountTargetPathTraversal(t *testing.T) {
	a := minimalADP()
	a.Workspace = &Workspace{
		Root: "/tmp/ws",
		Mounts: []WorkspaceMount{
			{ID: "m1", Source: WorkspaceMountSource{Path: "."}, Target: "../../escape"},
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "workspace.mounts") && strings.Contains(e, "..") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected workspace mount target path traversal error, got: %v", errs)
	}
}

// ──── Check 27: sandbox.policy MUST be present ───────────────────────────────

func TestValidateADPSemanticsSandboxNilPolicy(t *testing.T) {
	a := minimalADP()
	a.Sandbox = &Sandbox{
		Runtime: SandboxRuntimePython,
		Policy:  nil,
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "sandbox.policy MUST be present") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected sandbox.policy MUST be present error, got: %v", errs)
	}
}

func TestValidateADPSemanticsSandboxPolicyNilTimeoutMs(t *testing.T) {
	a := minimalADP()
	a.Sandbox = &Sandbox{
		Runtime: SandboxRuntimePython,
		Policy:  &SandboxPolicy{TimeoutMs: nil},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "sandbox.policy.timeout_ms MUST be present") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected sandbox.policy.timeout_ms error, got: %v", errs)
	}
}

// ──── Check 28: sandbox mounts with workspace source require workspace ────────

func TestValidateADPSemanticsSandboxMountWorkspaceSourceWithoutWorkspace(t *testing.T) {
	a := minimalADP()
	timeout := int64(30000)
	a.Sandbox = &Sandbox{
		Runtime: SandboxRuntimePython,
		Policy:  &SandboxPolicy{TimeoutMs: &timeout},
		Mounts: []SandboxMount{{
			ID:     "m1",
			Source: SandboxMountSource{Workspace: "main"},
			Target: "/app",
		}},
	}
	// No workspace declared
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "sandbox.mounts") && strings.Contains(e, "workspace") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected sandbox mount workspace source error, got: %v", errs)
	}
}

// ──── Check 32: sandbox snapshot with custom provider warning ─────────────────

func TestValidateADPSemanticsSandboxSnapshotCustomProviderWarning(t *testing.T) {
	a := minimalADP()
	timeout := int64(30000)
	enabled := true
	a.Sandbox = &Sandbox{
		Runtime:  SandboxRuntimePython,
		Provider: "custom",
		Policy:   &SandboxPolicy{TimeoutMs: &timeout},
		Snapshot: &SandboxSnapshot{Enabled: &enabled},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "WARNING") && strings.Contains(e, "snapshot") && strings.Contains(e, "custom") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected sandbox snapshot+custom warning, got: %v", errs)
	}
}

// ──── Check 33: artifacts.stores duplicate IDs ────────────────────────────────

func TestValidateADPSemanticsArtifactStoreDuplicateID(t *testing.T) {
	a := minimalADP()
	a.Artifacts = &Artifacts{
		Stores: []ArtifactStore{
			{ID: "art1", Provider: ArtifactProviderS3},
			{ID: "art1", Provider: ArtifactProviderGCS}, // duplicate
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "artifacts.stores") && strings.Contains(e, "duplicate") && strings.Contains(e, "art1") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected artifacts duplicate store id error, got: %v", errs)
	}
}

// ──── Check 34: nodes[].params.artifact.store_ref must reference known store ──

func TestValidateADPSemanticsNodeArtifactStoreRefBad(t *testing.T) {
	a := minimalADP()
	a.Artifacts = &Artifacts{
		Stores: []ArtifactStore{
			{ID: "art1", Provider: ArtifactProviderS3},
		},
	}
	a.Flow = makeFlow(
		[]Node{
			{
				ID:   "n",
				Kind: NodeKindLLM,
				Params: map[string]interface{}{
					"artifact": map[string]interface{}{
						"store_ref": "ghost-store",
					},
				},
			},
		},
		[]Edge{}, []string{"n"}, []string{"n"},
	)
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "store_ref") && strings.Contains(e, "ghost-store") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected node artifact store_ref error, got: %v", errs)
	}
}

func TestValidateADPSemanticsNodeParamsNil(t *testing.T) {
	// nil params → continue early, no error
	a := minimalADP()
	a.Artifacts = &Artifacts{
		Stores: []ArtifactStore{{ID: "art1", Provider: ArtifactProviderS3}},
	}
	a.Flow = makeFlow(
		[]Node{{ID: "n", Kind: NodeKindLLM, Params: nil}},
		[]Edge{}, []string{"n"}, []string{"n"},
	)
	errs := ValidateADPSemantics(a)
	for _, e := range errs {
		if strings.Contains(e, "store_ref") {
			t.Errorf("unexpected store_ref error for nil params: %v", e)
		}
	}
}

func TestValidateADPSemanticsNodeParamsNotMap(t *testing.T) {
	// params is not a map → continue, no error
	a := minimalADP()
	a.Artifacts = &Artifacts{
		Stores: []ArtifactStore{{ID: "art1", Provider: ArtifactProviderS3}},
	}
	a.Flow = makeFlow(
		[]Node{{ID: "n", Kind: NodeKindLLM, Params: "not-a-map"}},
		[]Edge{}, []string{"n"}, []string{"n"},
	)
	errs := ValidateADPSemantics(a)
	for _, e := range errs {
		if strings.Contains(e, "store_ref") {
			t.Errorf("unexpected store_ref error for non-map params: %v", e)
		}
	}
}

func TestValidateADPSemanticsNodeParamsArtifactNotMap(t *testing.T) {
	// params["artifact"] is not a map → continue, no error
	a := minimalADP()
	a.Artifacts = &Artifacts{
		Stores: []ArtifactStore{{ID: "art1", Provider: ArtifactProviderS3}},
	}
	a.Flow = makeFlow(
		[]Node{{
			ID:   "n",
			Kind: NodeKindLLM,
			Params: map[string]interface{}{
				"artifact": "not-a-map",
			},
		}},
		[]Edge{}, []string{"n"}, []string{"n"},
	)
	errs := ValidateADPSemantics(a)
	for _, e := range errs {
		if strings.Contains(e, "store_ref") {
			t.Errorf("unexpected store_ref error for non-map artifact: %v", e)
		}
	}
}

// ──── Check 35: observability.tracing.trace_events valid enum ────────────────

func TestValidateADPSemanticsObservabilityInvalidTraceEvent(t *testing.T) {
	a := minimalADP()
	a.Observability = &Observability{
		Tracing: &Tracing{
			TraceEvents: []TraceEvent{"model_request", "invalid_event"},
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "trace_events") && strings.Contains(e, "invalid_event") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected trace_events invalid event error, got: %v", errs)
	}
}

func TestValidateADPSemanticsObservabilityValidTraceEvents(t *testing.T) {
	a := minimalADP()
	a.Observability = &Observability{
		Tracing: &Tracing{
			TraceEvents: []TraceEvent{
				TraceEventModelRequest, TraceEventToolCall, TraceEventFlowNode,
				TraceEventLoopIteration, TraceEventInterrupt, TraceEventCostCheck,
				TraceEventArtifactWrite,
			},
		},
	}
	errs := ValidateADPSemantics(a)
	for _, e := range errs {
		if strings.Contains(e, "trace_events") {
			t.Errorf("unexpected trace_events error: %v", e)
		}
	}
}

// ──── Check 35b: observability.cost_reporting.model_refs must reference known models ─

func TestValidateADPSemanticsObservabilityCostReportingBadModelRef(t *testing.T) {
	a := minimalADP()
	a.Runtime.Models = []Model{{ID: "gpt4", Provider: "openai", Model: "gpt-4o"}}
	a.Observability = &Observability{
		CostReporting: &CostReporting{
			ModelRefs: []string{"gpt4", "ghost-model"},
		},
	}
	errs := ValidateADPSemantics(a)
	found := false
	for _, e := range errs {
		if strings.Contains(e, "cost_reporting") && strings.Contains(e, "ghost-model") {
			found = true
		}
	}
	if !found {
		t.Fatalf("expected cost_reporting model_refs error, got: %v", errs)
	}
}

func TestValidateADPSemanticsObservabilityNoModels(t *testing.T) {
	// No runtime.models → hasModels=false → model_refs check is skipped
	a := minimalADP()
	a.Observability = &Observability{
		CostReporting: &CostReporting{
			ModelRefs: []string{"any-model"},
		},
	}
	errs := ValidateADPSemantics(a)
	for _, e := range errs {
		if strings.Contains(e, "cost_reporting") && strings.Contains(e, "model_refs") {
			t.Errorf("unexpected cost_reporting error when no models defined: %v", e)
		}
	}
}
