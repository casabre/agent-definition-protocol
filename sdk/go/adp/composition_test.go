package adp

import (
	"fmt"
	"path/filepath"
	"strings"
	"testing"
)

// inMemoryResolver builds a Resolver backed by a map of filename->YAML-string.
// It matches on the base filename portion of the URI, so tests are
// path-independent.
func inMemoryResolver(files map[string]string) Resolver {
	return func(uri string) ([]byte, error) {
		// Try exact match first.
		if content, ok := files[uri]; ok {
			return []byte(content), nil
		}
		// Fall back to basename match so tests work regardless of absolute path.
		base := filepath.Base(uri)
		if content, ok := files[base]; ok {
			return []byte(content), nil
		}
		return nil, fmt.Errorf("file not found: %s", uri)
	}
}

// TestResolveADPBasicExtends verifies that a child manifest inherits fields from
// a base manifest via the extends directive and that local fields win (RFC 7396).
func TestResolveADPBasicExtends(t *testing.T) {
	baseYAML := `
adp_version: "0.1.0"
id: "base.agent"
name: "Base Agent"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "base.flow"
  graph:
    nodes:
      - { id: "input",  kind: "input" }
      - { id: "output", kind: "output" }
    edges: []
    start_nodes: ["input"]
    end_nodes:   ["output"]
evaluation:
  suites:
    - id: "base-suite"
      metrics:
        - { id: "m1", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
`
	childYAML := `
adp_version: "0.1.0"
id: "child.agent"
extends: "base.yaml"
name: "Child Agent"
`
	files := map[string]string{
		"child.yaml": childYAML,
		"base.yaml":  baseYAML,
	}
	resolver := inMemoryResolver(files)

	adp, errs := ResolveADP("child.yaml", resolver)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if adp == nil {
		t.Fatal("expected non-nil ADP")
	}
	// Child's id must override base's id.
	if adp.ID != "child.agent" {
		t.Errorf("expected id 'child.agent', got %q", adp.ID)
	}
	// Child's name overrides base's name.
	if adp.Name != "Child Agent" {
		t.Errorf("expected name 'Child Agent', got %q", adp.Name)
	}
	// Runtime should be inherited from base.
	if len(adp.Runtime.Execution) != 1 {
		t.Errorf("expected 1 execution entry inherited from base, got %d", len(adp.Runtime.Execution))
	}
}

// TestResolveADPCycleDetection verifies that a circular extends chain produces
// an error.
func TestResolveADPCycleDetection(t *testing.T) {
	cycleAYAML := `
adp_version: "0.1.0"
id: "cycle.a"
extends: "cycle_b.yaml"
`
	cycleBYAML := `
adp_version: "0.1.0"
id: "cycle.b"
extends: "cycle_a.yaml"
`
	files := map[string]string{
		"cycle_a.yaml": cycleAYAML,
		"cycle_b.yaml": cycleBYAML,
	}
	resolver := inMemoryResolver(files)

	_, errs := ResolveADP("cycle_a.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected cycle detection error")
	}
	found := false
	for _, e := range errs {
		if strings.Contains(e, "circular") {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected 'circular' in errors, got: %v", errs)
	}
}

// TestResolveADPImportAdditive verifies that import merges arrays additively.
func TestResolveADPImportAdditive(t *testing.T) {
	mainYAML := `
adp_version: "0.1.0"
id: "main.agent"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "main.flow"
  graph:
    nodes:
      - { id: "input",  kind: "input" }
      - { id: "output", kind: "output" }
    edges: []
    start_nodes: ["input"]
    end_nodes:   ["output"]
evaluation:
  suites:
    - id: "main-suite"
      metrics:
        - { id: "m1", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
import:
  - id: "extra-evals"
    from: "module_evals.yaml"
    sections: ["evaluation"]
`
	moduleYAML := `
evaluation:
  suites:
    - id: "imported-suite"
      metrics:
        - { id: "m2", type: "llm_judge", threshold: 0.9 }
`
	files := map[string]string{
		"main.yaml":        mainYAML,
		"module_evals.yaml": moduleYAML,
	}
	resolver := inMemoryResolver(files)

	adp, errs := ResolveADP("main.yaml", resolver)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if adp == nil {
		t.Fatal("expected non-nil ADP")
	}

	// Evaluation suites should contain both main-suite AND imported-suite.
	evalMap, ok := adp.Evaluation.(map[string]interface{})
	if !ok {
		t.Fatalf("expected evaluation to be a map, got %T", adp.Evaluation)
	}
	suitesRaw, _ := evalMap["suites"].([]interface{})
	if len(suitesRaw) < 2 {
		t.Errorf("expected at least 2 suites after additive import, got %d", len(suitesRaw))
	}
	suiteIDs := make(map[string]bool)
	for _, s := range suitesRaw {
		if sm, ok := s.(map[string]interface{}); ok {
			if id, ok := sm["id"].(string); ok {
				suiteIDs[id] = true
			}
		}
	}
	if !suiteIDs["main-suite"] {
		t.Error("expected 'main-suite' in merged evaluation")
	}
	if !suiteIDs["imported-suite"] {
		t.Error("expected 'imported-suite' in merged evaluation after import")
	}
}

// TestResolveADPOverrideSet verifies that override op:set replaces an existing value.
func TestResolveADPOverrideSet(t *testing.T) {
	mainYAML := `
adp_version: "0.1.0"
id: "override.agent"
name: "Original Name"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "ov.flow"
  graph:
    nodes:
      - { id: "input",  kind: "input" }
      - { id: "output", kind: "output" }
    edges: []
    start_nodes: ["input"]
    end_nodes:   ["output"]
evaluation:
  suites:
    - id: "s1"
      metrics:
        - { id: "m1", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
overrides:
  - path: "/name"
    op: "set"
    value: "Overridden Name"
`
	files := map[string]string{
		"main.yaml": mainYAML,
	}
	resolver := inMemoryResolver(files)

	adp, errs := ResolveADP("main.yaml", resolver)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if adp.Name != "Overridden Name" {
		t.Errorf("expected name 'Overridden Name' after override, got %q", adp.Name)
	}
}

// TestResolveADPOverrideAppend verifies that override op:append adds an element
// to an existing array.
func TestResolveADPOverrideAppend(t *testing.T) {
	mainYAML := `
adp_version: "0.1.0"
id: "append.agent"
tags:
  - "existing-tag"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "ap.flow"
  graph:
    nodes:
      - { id: "input",  kind: "input" }
      - { id: "output", kind: "output" }
    edges: []
    start_nodes: ["input"]
    end_nodes:   ["output"]
evaluation:
  suites:
    - id: "s1"
      metrics:
        - { id: "m1", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
overrides:
  - path: "/tags"
    op: "append"
    value: "new-tag"
`
	files := map[string]string{
		"main.yaml": mainYAML,
	}
	resolver := inMemoryResolver(files)

	adp, errs := ResolveADP("main.yaml", resolver)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if len(adp.Tags) != 2 {
		t.Errorf("expected 2 tags after append override, got %d: %v", len(adp.Tags), adp.Tags)
	}
	found := false
	for _, tag := range adp.Tags {
		if tag == "new-tag" {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected 'new-tag' in tags, got: %v", adp.Tags)
	}
}

// TestResolveADPDepthExceeded verifies that an extends chain deeper than maxDepth
// returns an error.
func TestResolveADPDepthExceeded(t *testing.T) {
	// Build a chain of 12 manifests (exceeds maxDepth=10).
	files := map[string]string{}
	for i := 12; i >= 1; i-- {
		var content string
		if i == 12 {
			content = fmt.Sprintf(`adp_version: "0.1.0"
id: "depth.%d"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
`, i)
		} else {
			content = fmt.Sprintf(`adp_version: "0.1.0"
id: "depth.%d"
extends: "depth_%d.yaml"
`, i, i+1)
		}
		files[fmt.Sprintf("depth_%d.yaml", i)] = content
	}
	resolver := inMemoryResolver(files)

	_, errs := ResolveADP("depth_1.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected depth exceeded error")
	}
	found := false
	for _, e := range errs {
		if strings.Contains(e, "depth") {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("expected depth error message, got: %v", errs)
	}
}

// TestResolveADPMissingBase verifies that a missing base file is an error.
func TestResolveADPMissingBase(t *testing.T) {
	childYAML := `
adp_version: "0.1.0"
id: "child.agent"
extends: "nonexistent.yaml"
`
	files := map[string]string{
		"child.yaml": childYAML,
	}
	resolver := inMemoryResolver(files)

	_, errs := ResolveADP("child.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected error for missing base manifest")
	}
}
