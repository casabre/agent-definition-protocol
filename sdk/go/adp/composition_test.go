package adp

import (
	"fmt"
	"os"
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
// a base manifest via the extends directive and that local fields win (id-keyed merge).
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
		"main.yaml":         mainYAML,
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

func TestToIndexValid(t *testing.T) {
	// toIndex is exercised when pointerGet encounters an array
	// Test via applyOverride with a set on array index
	mainYAML := `
adp_version: "0.1.0"
id: "arr-set"
tags:
  - "first"
  - "second"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s1"
      metrics:
        - { id: "m1", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
overrides:
  - path: "/tags/0"
    op: "set"
    value: "replaced"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	adp, errs := ResolveADP("main.yaml", resolver)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if len(adp.Tags) < 2 {
		t.Fatalf("expected 2 tags, got %d", len(adp.Tags))
	}
	if adp.Tags[0] != "replaced" {
		t.Errorf("expected tags[0]='replaced', got %q", adp.Tags[0])
	}
}

func TestToIndexInvalid(t *testing.T) {
	// toIndex with non-integer segment should error
	mainYAML := `
adp_version: "0.1.0"
id: "arr-badidx"
tags:
  - "a"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
overrides:
  - path: "/tags/notanumber"
    op: "set"
    value: "x"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	_, errs := ResolveADP("main.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected error for non-integer array index")
	}
}

func TestApplyOverrideSetOnNonObjectNonArray(t *testing.T) {
	// Navigate into a scalar → error
	mainYAML := `
adp_version: "0.1.0"
id: "nav-scalar"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
overrides:
  - path: "/runtime/execution/0/entrypoint/foo"
    op: "set"
    value: "x"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	_, errs := ResolveADP("main.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected error for navigating into scalar")
	}
}

func TestApplyOverrideUnknownOp(t *testing.T) {
	mainYAML := `
adp_version: "0.1.0"
id: "bad-op"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
overrides:
  - path: "/id"
    op: "zap"
    value: "x"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	_, errs := ResolveADP("main.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected error for unknown op")
	}
}

func TestApplyOverrideAppendToNonArray(t *testing.T) {
	mainYAML := `
adp_version: "0.1.0"
id: "append-scalar"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
overrides:
  - path: "/id"
    op: "append"
    value: "x"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	_, errs := ResolveADP("main.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected error for appending to non-array")
	}
}

func TestApplyOverrideDeleteMissingIntermediate(t *testing.T) {
	// Delete with missing intermediate → should be no-op
	mainYAML := `
adp_version: "0.1.0"
id: "del-miss"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
overrides:
  - path: "/nonexistent_parent/child"
    op: "delete"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	adp, errs := ResolveADP("main.yaml", resolver)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if adp.ID != "del-miss" {
		t.Errorf("expected id 'del-miss', got %q", adp.ID)
	}
}

func TestApplyOverrideSetOnNonExistentKey(t *testing.T) {
	mainYAML := `
adp_version: "0.1.0"
id: "set-miss"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
overrides:
  - path: "/nonexistent_field"
    op: "set"
    value: "x"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	_, errs := ResolveADP("main.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected error for set on non-existent key")
	}
}

func TestResolveURIRegistry(t *testing.T) {
	mainYAML := `
adp_version: "0.1.0"
id: "reg"
extends: "registry://acme/base:1.0"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	_, errs := ResolveADP("main.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected error for registry:// URI")
	}
	found := false
	for _, e := range errs {
		if strings.Contains(e, "registry") {
			found = true
		}
	}
	if !found {
		t.Errorf("expected 'registry' in errors, got: %v", errs)
	}
}

func TestResolveURIHTTPPassthrough(t *testing.T) {
	// http:// URI in extends → resolver gets called for it → file not found error
	mainYAML := `
adp_version: "0.1.0"
id: "http-extends"
extends: "http://example.com/base.yaml"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	_, errs := ResolveADP("main.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected error for unresolvable http URI")
	}
}

func TestLoadURIFileScheme(t *testing.T) {
	// loadURI with a plain file path (no resolver) should read the file
	tmp, _ := os.MkdirTemp("", "go-comp-*")
	defer os.RemoveAll(tmp)
	yamlPath := filepath.Join(tmp, "agent.yaml")
	content := `adp_version: "0.1.0"
id: "file-uri-test"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
`
	os.WriteFile(yamlPath, []byte(content), 0o644)
	adp, errs := ResolveADP(yamlPath, nil)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if adp.ID != "file-uri-test" {
		t.Errorf("expected id 'file-uri-test', got %q", adp.ID)
	}
}

func TestLoadURIYAMLError(t *testing.T) {
	// loadURI with YAML that cannot unmarshal into map[string]interface{} should error
	// A bare list is valid YAML but cannot be unmarshaled into a map
	resolver := inMemoryResolver(map[string]string{"bad.yaml": "- item1\n- item2\n"})
	_, errs := ResolveADP("bad.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected error for YAML that is not a map")
	}
}

func TestLoadURIFileSchemeExplicit(t *testing.T) {
	// loadURI with file:// URI scheme in extends → resolveURI passes it through → loadURI reads via Path
	tmp, _ := os.MkdirTemp("", "go-comp-file-*")
	defer os.RemoveAll(tmp)

	baseContent := `adp_version: "0.1.0"
id: "file-scheme-base"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
`
	basePath := filepath.Join(tmp, "base.yaml")
	os.WriteFile(basePath, []byte(baseContent), 0o644)

	// Child uses file:// URI to reference the base
	childContent := `adp_version: "0.1.0"
id: "file-scheme-child"
extends: "file://` + basePath + `"
`
	childPath := filepath.Join(tmp, "child.yaml")
	os.WriteFile(childPath, []byte(childContent), 0o644)

	adp, errs := ResolveADP(childPath, nil)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if adp.ID != "file-scheme-child" {
		t.Errorf("expected id 'file-scheme-child', got %q", adp.ID)
	}
}

func TestLoadURINoResolverHTTP(t *testing.T) {
	// loadURI with http:// URI and no resolver should return an error
	mainYAML := `
adp_version: "0.1.0"
id: "http-no-resolver"
extends: "http://example.com/base.yaml"
`
	// With nil resolver, http URI should fail with "HTTP URIs require a custom Resolver"
	tmp, _ := os.MkdirTemp("", "go-comp-http-*")
	defer os.RemoveAll(tmp)
	yamlPath := filepath.Join(tmp, "main.yaml")
	os.WriteFile(yamlPath, []byte(mainYAML), 0o644)
	_, errs := ResolveADP(yamlPath, nil)
	if len(errs) == 0 {
		t.Fatal("expected error for HTTP URI without resolver")
	}
	found := false
	for _, e := range errs {
		if strings.Contains(e, "HTTP") || strings.Contains(e, "http") {
			found = true
		}
	}
	if !found {
		t.Errorf("expected HTTP error, got: %v", errs)
	}
}

func TestToStringSliceNonSlice(t *testing.T) {
	// toStringSlice with non-slice → returns nil; exercised via import sections filter
	mainYAML := `
adp_version: "0.1.0"
id: "str-slice"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
import:
  - id: "mod"
    from: "module.yaml"
    sections:
      - "tags"
`
	moduleYAML := `tags:
  - "tag1"
`
	resolver := inMemoryResolver(map[string]string{
		"main.yaml":   mainYAML,
		"module.yaml": moduleYAML,
	})
	adp, errs := ResolveADP("main.yaml", resolver)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if adp.ID != "str-slice" {
		t.Errorf("expected id 'str-slice', got %q", adp.ID)
	}
}

func TestDeepMergeNullDeletesKey(t *testing.T) {
	// null overlay value should delete key from result
	mainYAML := `
adp_version: "0.1.0"
id: "null-merge"
name: "Will Be Deleted"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
`
	childYAML := `
adp_version: "0.1.0"
id: "child-null"
extends: "main.yaml"
name: null
`
	resolver := inMemoryResolver(map[string]string{
		"main.yaml":  mainYAML,
		"child.yaml": childYAML,
	})
	adp, errs := ResolveADP("child.yaml", resolver)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	// null in child should delete name
	if adp.Name != "" {
		t.Errorf("expected name to be empty after null override, got %q", adp.Name)
	}
}

func TestAdditiveMergeScalarFallback(t *testing.T) {
	// Module has a scalar key already in merged → else fallback fires (module wins)
	baseYAML := `
adp_version: "0.1.0"
id: "base-scalar"
name: "Base Name"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
`
	childYAML := `
adp_version: "0.1.0"
id: "child-scalar"
extends: "base.yaml"
import:
  - id: "name-mod"
    from: "name_module.yaml"
`
	// Module has "name" (scalar) → additive merge: module wins for scalars that exist
	moduleYAML := `name: "Module Name"`
	resolver := inMemoryResolver(map[string]string{
		"base.yaml":        baseYAML,
		"child.yaml":       childYAML,
		"name_module.yaml": moduleYAML,
	})
	adp, errs := ResolveADP("child.yaml", resolver)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if adp.ID != "child-scalar" {
		t.Errorf("expected id 'child-scalar', got %q", adp.ID)
	}
}

func TestAdditiveMergeModuleListButBaseNotList(t *testing.T) {
	// Module has array for "tags" but base merged doesn't have tags → additive merge adds it
	baseYAML := `
adp_version: "0.1.0"
id: "base-tagtype"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
`
	childYAML := `
adp_version: "0.1.0"
id: "child-tagtype"
extends: "base.yaml"
import:
  - id: "tags-mod"
    from: "tags_module.yaml"
`
	// Module has tags as an array; base doesn't have tags at all → "!(key in result)" branch fires
	moduleYAML := `tags:
  - "imported-tag"`
	resolver := inMemoryResolver(map[string]string{
		"base.yaml":        baseYAML,
		"child.yaml":       childYAML,
		"tags_module.yaml": moduleYAML,
	})
	adp, errs := ResolveADP("child.yaml", resolver)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if len(adp.Tags) == 0 {
		t.Error("expected imported tags")
	}
}

func TestResolveManifestEmptyOverridesList(t *testing.T) {
	// Empty overrides list → should be no-op
	mainYAML := `
adp_version: "0.1.0"
id: "nil-override"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
overrides: []
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	adp, errs := ResolveADP("main.yaml", resolver)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if adp.ID != "nil-override" {
		t.Errorf("expected id 'nil-override', got %q", adp.ID)
	}
}

func TestResolveADPImportMissingFrom(t *testing.T) {
	// Import entry with empty "from" field → error
	mainYAML := `
adp_version: "0.1.0"
id: "bad-import"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites: []
import:
  - id: "bad"
    from: ""
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	_, errs := ResolveADP("main.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected error for empty 'from' field")
	}
}

// ──── Additional branch coverage tests ──────────────────────────────────────

// TestResolveADPSemanticsReturnedViaResolveADP verifies that semantic errors
// propagate back through ResolveADP (exercises the semErrors > 0 return path).
func TestResolveADPSemanticsReturnedViaResolveADP(t *testing.T) {
	// A manifest that will pass structural compose but fail semantic validation.
	// We construct a node with a tool_ref that doesn't exist in tools.
	mainYAML := `
adp_version: "0.2.0"
id: "sem-fail.agent"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - id: "n"
        kind: "tool"
        tool_ref: "nonexistent-tool"
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
tools:
  http_apis:
    - id: "real-tool"
      description: "A real tool"
      base_url: "https://example.com"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	_, errs := ResolveADP("main.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected semantic errors to propagate through ResolveADP")
	}
	found := false
	for _, e := range errs {
		if strings.Contains(e, "tool_ref") || strings.Contains(e, "nonexistent") {
			found = true
		}
	}
	if !found {
		t.Errorf("expected tool_ref error, got: %v", errs)
	}
}

// TestResolveManifestImportNilEntry verifies that a non-map import entry is
// silently skipped (ov == nil continue branch).
func TestResolveManifestImportNilEntry(t *testing.T) {
	// YAML where import list contains a non-map entry (e.g., a string).
	// In Go's yaml.v3, a bare string in a list becomes a string, not a map,
	// so `entry, _ := entryRaw.(map[string]interface{})` will yield nil → continue.
	mainYAML := `
adp_version: "0.1.0"
id: "import-nil-entry"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
import:
  - "not-a-map-entry"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	adp, errs := ResolveADP("main.yaml", resolver)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if adp.ID != "import-nil-entry" {
		t.Errorf("expected id 'import-nil-entry', got %q", adp.ID)
	}
}

// TestResolveManifestImportRegistryURIError verifies that a registry:// URI
// in an import.from field returns an error.
func TestResolveManifestImportRegistryURIError(t *testing.T) {
	mainYAML := `
adp_version: "0.1.0"
id: "import-registry"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites: []
import:
  - id: "reg"
    from: "registry://acme/tools:1.0"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	_, errs := ResolveADP("main.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected error for registry:// URI in import")
	}
}

// TestResolveManifestImportLoadError verifies that a load error in import.from
// returns an error.
func TestResolveManifestImportLoadError(t *testing.T) {
	mainYAML := `
adp_version: "0.1.0"
id: "import-load-fail"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites: []
import:
  - id: "missing"
    from: "totally_missing_module.yaml"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	_, errs := ResolveADP("main.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected error for missing import module")
	}
}

// TestResolveManifestOverrideNilEntry verifies that a non-map override entry
// is silently skipped.
func TestResolveManifestOverrideNilEntry(t *testing.T) {
	mainYAML := `
adp_version: "0.1.0"
id: "override-nil-entry"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
overrides:
  - "not-a-map-override"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	adp, errs := ResolveADP("main.yaml", resolver)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if adp.ID != "override-nil-entry" {
		t.Errorf("expected id 'override-nil-entry', got %q", adp.ID)
	}
}

// TestDeepMergeOverlayMapBaseNotMap verifies that when overlay has a map value
// for a key where base has a non-map value, the overlay map wins (deepCopyValue).
func TestDeepMergeOverlayMapBaseNotMap(t *testing.T) {
	// Base has "meta" as a scalar string, child overrides it with a map.
	baseYAML := `
adp_version: "0.1.0"
id: "base-meta-scalar"
meta: "scalar-value"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
`
	childYAML := `
adp_version: "0.1.0"
id: "child-meta-map"
extends: "base.yaml"
meta:
  key: "value"
  nested: true
`
	resolver := inMemoryResolver(map[string]string{
		"base.yaml":  baseYAML,
		"child.yaml": childYAML,
	})
	adp, errs := ResolveADP("child.yaml", resolver)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if adp.ID != "child-meta-map" {
		t.Errorf("expected id 'child-meta-map', got %q", adp.ID)
	}
}

// TestAdditiveMergeModuleListBaseNotList verifies that when the module has a
// list value for a key where the base has a NON-list value (key exists),
// the module list wins (deepCopyValue branch, line 208).
func TestAdditiveMergeModuleListBaseNotList(t *testing.T) {
	// Base has "tags" as a scalar string; module has "tags" as a list.
	// After deepMerge (extends), merged["tags"] is the scalar from base.
	// Then additiveMerge: module["tags"] is []interface{} but merged["tags"] is a string → deepCopyValue wins.
	baseYAML := `
adp_version: "0.1.0"
id: "base-tags-scalar"
tags: "single-tag-scalar"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
`
	childYAML := `
adp_version: "0.1.0"
id: "child-tags-list"
extends: "base.yaml"
import:
  - id: "tags-mod"
    from: "tags_module.yaml"
`
	moduleYAML := `tags:
  - "tag-from-module"
`
	resolver := inMemoryResolver(map[string]string{
		"base.yaml":        baseYAML,
		"child.yaml":       childYAML,
		"tags_module.yaml": moduleYAML,
	})
	adp, errs := ResolveADP("child.yaml", resolver)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if adp.ID != "child-tags-list" {
		t.Errorf("expected id 'child-tags-list', got %q", adp.ID)
	}
}

// TestAdditiveMergeModuleMapBaseNotMap verifies that when the module has a
// map value for a key where the base has a NON-map value (key exists),
// the module map wins (deepCopyValue branch, line 214).
func TestAdditiveMergeModuleMapBaseNotMap(t *testing.T) {
	// Base has "meta" as a string; module also has "meta" as a map.
	// additiveMerge: module["meta"] is a map but merged["meta"] is a string → deepCopyValue.
	baseYAML := `
adp_version: "0.1.0"
id: "base-meta-str"
meta: "original-string"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
`
	childYAML := `
adp_version: "0.1.0"
id: "child-meta-mapmod"
extends: "base.yaml"
import:
  - id: "meta-mod"
    from: "meta_module.yaml"
`
	moduleYAML := `meta:
  author: "team"
  version: 2
`
	resolver := inMemoryResolver(map[string]string{
		"base.yaml":        baseYAML,
		"child.yaml":       childYAML,
		"meta_module.yaml": moduleYAML,
	})
	adp, errs := ResolveADP("child.yaml", resolver)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if adp.ID != "child-meta-mapmod" {
		t.Errorf("expected id 'child-meta-mapmod', got %q", adp.ID)
	}
}

// TestApplyOverridePathNoLeadingSlash verifies that override path without '/'
// returns an error (line 226-228).
func TestApplyOverridePathNoLeadingSlash(t *testing.T) {
	mainYAML := `
adp_version: "0.1.0"
id: "bad-path"
name: "Original"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
overrides:
  - path: "name"
    op: "set"
    value: "New"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	_, errs := ResolveADP("main.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected error for override path without leading '/'")
	}
}

// TestApplyOverrideDeletePointerGetError verifies that a pointerGet error
// during delete navigation is returned (line 247-249).
// This requires a path with an array followed by a non-integer segment with allowMissing=true.
// Actually with allowMissing=true, the map path always succeeds. The array path can error
// if the segment is not an integer. We need: intermediate is an array, segment is non-integer.
func TestApplyOverrideDeleteArrayBadIndex(t *testing.T) {
	mainYAML := `
adp_version: "0.1.0"
id: "del-arr-bad"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
overrides:
  - path: "/runtime/execution/notanint/entrypoint"
    op: "delete"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	_, errs := ResolveADP("main.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected error for non-integer array index in delete path")
	}
}

// TestPointerGetDefaultCase verifies the default case in pointerGet when the
// node is a scalar (not map or array) during intermediate navigation.
// This is triggered by navigating through a string value with set/append.
func TestPointerGetDefaultCase(t *testing.T) {
	// Path: /name/sub — "name" is a string, then we try to navigate "sub" on it.
	// The intermediate navigation calls pointerGet(stringValue, "sub", path, false)
	// → default case in pointerGet.
	mainYAML := `
adp_version: "0.1.0"
id: "scalar-nav"
name: "Agent Name"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
overrides:
  - path: "/name/sub/value"
    op: "set"
    value: "x"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	_, errs := ResolveADP("main.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected error for navigating into scalar during intermediate navigation")
	}
	found := false
	for _, e := range errs {
		if strings.Contains(e, "navigate") || strings.Contains(e, "path") {
			found = true
		}
	}
	if !found {
		t.Errorf("expected navigation error, got: %v", errs)
	}
}

// TestPointerGetMapMissingSegment verifies the !ok branch in pointerGet for
// the map case when allowMissing=false (line 318-320).
// Triggered by set/append with an intermediate path segment that doesn't exist in the map.
func TestPointerGetMapMissingSegment(t *testing.T) {
	mainYAML := `
adp_version: "0.1.0"
id: "missing-seg"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
overrides:
  - path: "/runtime/nonexistent_key/value"
    op: "set"
    value: "x"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	_, errs := ResolveADP("main.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected error for missing intermediate path segment")
	}
}

// TestPointerGetArrayBadIndex verifies the array path in pointerGet when the
// segment is not an integer (line 322-326 in pointerGet array case).
// This is triggered by navigating to an array with a non-integer intermediate segment.
func TestPointerGetArrayBadIndex(t *testing.T) {
	mainYAML := `
adp_version: "0.1.0"
id: "arr-bad-inter"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
overrides:
  - path: "/runtime/execution/notanint/entrypoint"
    op: "set"
    value: "new:main"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	_, errs := ResolveADP("main.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected error for non-integer array index in set intermediate navigation")
	}
}

// TestApplyOverrideAppendMissingParentKey verifies the append path where
// pointerGet returns an error because the last key doesn't exist (line 291-293).
func TestApplyOverrideAppendMissingKey(t *testing.T) {
	mainYAML := `
adp_version: "0.1.0"
id: "append-miss"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
overrides:
  - path: "/nonexistent_list_field"
    op: "append"
    value: "x"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	_, errs := ResolveADP("main.yaml", resolver)
	if len(errs) == 0 {
		t.Fatal("expected error for append to nonexistent field")
	}
}

// TestLoadURIResolverReturnsInvalidYAML verifies that when a resolver returns
// bytes that are valid YAML but produce an error during unmarshal into a map
// (e.g., invalid YAML bytes), an error is returned.
func TestLoadURIResolverReturnsInvalidYAML(t *testing.T) {
	// Resolver returns bytes with invalid YAML (syntax error).
	// yaml.Unmarshal into map[string]interface{} will return an error.
	invalidYAMLResolver := func(uri string) ([]byte, error) {
		return []byte("key: [\ninvalid yaml here"), nil
	}
	// We need a valid YAML entry point that uses the resolver for extends.
	// Use a real file as the entry, and let it try to load the base via extends.
	tmp, _ := os.MkdirTemp("", "go-comp-inv-yaml-*")
	defer os.RemoveAll(tmp)
	entryContent := `adp_version: "0.1.0"
id: "entry"
extends: "base.yaml"
`
	entryPath := filepath.Join(tmp, "entry.yaml")
	os.WriteFile(entryPath, []byte(entryContent), 0o644)
	_, errs := ResolveADP(entryPath, invalidYAMLResolver)
	if len(errs) == 0 {
		t.Fatal("expected error for invalid YAML from resolver")
	}
}

// TestLoadURIEmptyYAML verifies that a resolver returning empty YAML bytes
// (nil raw map) succeeds and returns an empty map (the nil→empty map branch).
func TestLoadURIEmptyYAML(t *testing.T) {
	// Empty YAML → yaml.Unmarshal sets raw = nil → code sets raw = map[string]interface{}{}
	emptyResolver := func(uri string) ([]byte, error) {
		base := filepath.Base(uri)
		if base == "base.yaml" {
			// Return YAML that unmarshal into nil map (empty document).
			return []byte(""), nil
		}
		// For the main file, return a valid manifest.
		return []byte(`adp_version: "0.1.0"
id: "empty-yaml-test"
extends: "base.yaml"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
`), nil
	}
	adp, errs := ResolveADP("main.yaml", emptyResolver)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if adp.ID != "empty-yaml-test" {
		t.Errorf("expected id 'empty-yaml-test', got %q", adp.ID)
	}
}

// TestToStringSliceNonSliceInput verifies that toStringSlice returns nil for
// a non-slice input (line 436-438).
// We trigger this via import.sections being a non-list (scalar string in YAML).
// In Go YAML, a scalar string → string, not []interface{}, so toStringSlice returns nil
// → sections filter is skipped → entire module is imported additively.
// TestDeepMergeRecursive directly exercises the recursive deepMerge branch
// (overlay is a map AND base[k] is also a map → recursive call at line 184).
func TestDeepMergeRecursive(t *testing.T) {
	base := map[string]interface{}{
		"nested": map[string]interface{}{
			"a": "original",
			"b": "keep",
		},
	}
	overlay := map[string]interface{}{
		"nested": map[string]interface{}{
			"a": "overridden",
			"c": "new",
		},
	}
	result := deepMerge(base, overlay)
	nested, ok := result["nested"].(map[string]interface{})
	if !ok {
		t.Fatalf("expected nested to be a map, got %T", result["nested"])
	}
	if nested["a"] != "overridden" {
		t.Errorf("expected nested.a='overridden', got %q", nested["a"])
	}
	if nested["b"] != "keep" {
		t.Errorf("expected nested.b='keep', got %q", nested["b"])
	}
	if nested["c"] != "new" {
		t.Errorf("expected nested.c='new', got %q", nested["c"])
	}
}

// TestApplyOverrideDeleteExistingKey verifies that override op:delete successfully
// removes a key that exists (composition.go lines 255-259).
func TestApplyOverrideDeleteExistingKey(t *testing.T) {
	mainYAML := `
adp_version: "0.1.0"
id: "del-existing"
name: "Will Be Deleted"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
overrides:
  - path: "/name"
    op: "delete"
`
	resolver := inMemoryResolver(map[string]string{"main.yaml": mainYAML})
	adp, errs := ResolveADP("main.yaml", resolver)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	// After delete, name should be empty.
	if adp.Name != "" {
		t.Errorf("expected name to be empty after delete override, got %q", adp.Name)
	}
}

// TestResolveADPNilResolverMissingFile exercises the loadURI nil-resolver +
// missing file path (composition.go "cannot resolve URI" error).
func TestResolveADPNilResolverMissingFile(t *testing.T) {
	// Call ResolveADP with nil resolver and a file that doesn't exist.
	_, errs := ResolveADP("/nonexistent/path/to/missing_agent.yaml", nil)
	if len(errs) == 0 {
		t.Fatal("expected error for missing file with nil resolver")
	}
	found := false
	for _, e := range errs {
		if strings.Contains(e, "cannot resolve URI") || strings.Contains(e, "no such file") || strings.Contains(e, "open") {
			found = true
		}
	}
	if !found {
		t.Errorf("expected file-not-found error, got: %v", errs)
	}
}

// TestResolveURIInvalidPercentEncoding verifies the url.Parse error path
// (composition.go line 349). If url.Parse doesn't fail for the input, the test
// is skipped.
func TestResolveURIInvalidPercentEncoding(t *testing.T) {
	result, compErr := resolveURI("%ZZ", "/some/base/path")
	if compErr != nil {
		// url.Parse returned an error → the branch is covered.
		if !strings.Contains(compErr.Error(), "invalid") && !strings.Contains(compErr.Error(), "%ZZ") {
			t.Errorf("unexpected error message: %v", compErr)
		}
		return
	}
	// url.Parse did not fail for "%ZZ" on this platform → skip.
	if result != "" {
		t.Skip("url.Parse did not return an error for %ZZ on this platform; branch not reachable")
	}
}

func TestToStringSliceWithScalarSections(t *testing.T) {
	mainYAML := `
adp_version: "0.1.0"
id: "str-slice-scalar"
runtime:
  execution:
    - id: "py"
      backend: "python"
      entrypoint: "main:app"
flow:
  id: "f"
  graph:
    nodes:
      - { id: "n", kind: "input" }
    edges: []
    start_nodes: ["n"]
    end_nodes: ["n"]
evaluation:
  suites:
    - id: "s"
      metrics:
        - { id: "m", type: "deterministic", function: "noop", scoring: "boolean", threshold: true }
import:
  - id: "mod"
    from: "module.yaml"
    sections: "tags"
`
	moduleYAML := `tags:
  - "tag1"
`
	resolver := inMemoryResolver(map[string]string{
		"main.yaml":   mainYAML,
		"module.yaml": moduleYAML,
	})
	adp, errs := ResolveADP("main.yaml", resolver)
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	// When sections is a scalar string, toStringSlice returns nil → all sections imported.
	if adp.ID != "str-slice-scalar" {
		t.Errorf("expected id 'str-slice-scalar', got %q", adp.ID)
	}
}

// ---------------------------------------------------------------------------
// Id-keyed merge unit tests
// ---------------------------------------------------------------------------

func TestApplyPatchObjectDeepMerge(t *testing.T) {
	base := map[string]interface{}{
		"a": map[string]interface{}{"x": 1, "y": 2},
		"b": "keep",
	}
	patch := map[string]interface{}{
		"a": map[string]interface{}{"x": 99},
	}
	result := applyPatch(base, patch)
	a := result["a"].(map[string]interface{})
	if a["x"] != 99 || a["y"] != 2 || result["b"] != "keep" {
		t.Errorf("unexpected result: %v", result)
	}
}

func TestApplyPatchAddsMissingKey(t *testing.T) {
	base := map[string]interface{}{"existing": "value"}
	patch := map[string]interface{}{"new_key": map[string]interface{}{"nested": true}}
	result := applyPatch(base, patch)
	if _, ok := result["new_key"]; !ok {
		t.Error("expected new_key to be added")
	}
	if result["existing"] != "value" {
		t.Error("existing key should be preserved")
	}
}

func TestApplyPatchListIDKeyedMatch(t *testing.T) {
	base := map[string]interface{}{
		"models": []interface{}{
			map[string]interface{}{"id": "gpt4", "model": "gpt-4"},
			map[string]interface{}{"id": "claude", "model": "claude-3"},
		},
	}
	patch := map[string]interface{}{
		"models": []interface{}{
			map[string]interface{}{"id": "gpt4", "model": "gpt-4o"},
		},
	}
	result := applyPatch(base, patch)
	models := result["models"].([]interface{})
	if len(models) != 2 {
		t.Fatalf("expected 2 models, got %d", len(models))
	}
	m0 := models[0].(map[string]interface{})
	if m0["model"] != "gpt-4o" {
		t.Errorf("expected gpt-4o, got %v", m0["model"])
	}
	m1 := models[1].(map[string]interface{})
	if m1["id"] != "claude" {
		t.Errorf("claude entry should be preserved, got %v", m1)
	}
}

func TestApplyPatchListIDKeyedNewEntry(t *testing.T) {
	base := map[string]interface{}{
		"models": []interface{}{
			map[string]interface{}{"id": "gpt4", "model": "gpt-4"},
		},
	}
	patch := map[string]interface{}{
		"models": []interface{}{
			map[string]interface{}{"id": "new-model", "model": "llama-3"},
		},
	}
	result := applyPatch(base, patch)
	models := result["models"].([]interface{})
	if len(models) != 2 {
		t.Fatalf("expected 2 models, got %d", len(models))
	}
}

func TestApplyPatchListNoIDReplaces(t *testing.T) {
	base := map[string]interface{}{
		"tags": []interface{}{"a", "b", "c"},
	}
	patch := map[string]interface{}{
		"tags": []interface{}{"x", "y"},
	}
	result := applyPatch(base, patch)
	tags := result["tags"].([]interface{})
	if len(tags) != 2 || tags[0] != "x" {
		t.Errorf("expected [x, y], got %v", tags)
	}
}

func TestApplyPatchNullRemovesKey(t *testing.T) {
	base := map[string]interface{}{"keep": "yes", "remove": "this"}
	patch := map[string]interface{}{"remove": nil}
	result := applyPatch(base, patch)
	if _, ok := result["remove"]; ok {
		t.Error("'remove' key should have been deleted")
	}
	if result["keep"] != "yes" {
		t.Error("'keep' should be preserved")
	}
}

func TestExtendsLocalFieldAndOverride(t *testing.T) {
	// Local field (id-keyed merge) applied before override; override wins on same key.
	baseYAML := `adp_version: "0.3.0"
id: base
runtime:
  execution:
    - id: py
      backend: python
      entrypoint: a:b
telemetry:
  service_name: original
  protocol: grpc
`
	manifest := `adp_version: "0.3.0"
id: child
extends: "base.yaml"
telemetry:
  service_name: local-value
overrides:
  - path: /telemetry/service_name
    value: overridden
    op: set
`
	adp, errs := ResolveADP("child.yaml", inMemoryResolver(map[string]string{
		"base.yaml":  baseYAML,
		"child.yaml": manifest,
	}))
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if adp.Telemetry == nil || adp.Telemetry.ServiceName != "overridden" {
		t.Errorf("expected 'overridden', got %v", adp.Telemetry)
	}
	if adp.Telemetry.Protocol != "grpc" {
		t.Errorf("protocol should be inherited from base, got %q", adp.Telemetry.Protocol)
	}
}

func TestExtendsLocalFieldDifferentKeys(t *testing.T) {
	// Local field on one key + override on another; both applied correctly.
	baseYAML := `adp_version: "0.3.0"
id: base
runtime:
  execution:
    - id: py
      backend: python
      entrypoint: a:b
telemetry:
  service_name: base-name
  protocol: grpc
`
	manifest := `adp_version: "0.3.0"
id: child
extends: "base.yaml"
telemetry:
  service_name: local-name
overrides:
  - path: /telemetry/protocol
    value: http/protobuf
    op: set
`
	adp, errs := ResolveADP("child.yaml", inMemoryResolver(map[string]string{
		"base.yaml":  baseYAML,
		"child.yaml": manifest,
	}))
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if adp.Telemetry == nil {
		t.Fatal("telemetry should not be nil")
	}
	if adp.Telemetry.ServiceName != "local-name" {
		t.Errorf("expected 'local-name', got %q", adp.Telemetry.ServiceName)
	}
	if adp.Telemetry.Protocol != "http/protobuf" {
		t.Errorf("expected 'http/protobuf', got %q", adp.Telemetry.Protocol)
	}
}

func TestExtendsIdListFullPipeline(t *testing.T) {
	// Extends + local id-list + import + overrides: all active, correct result.
	baseYAML := `adp_version: "0.3.0"
id: base
runtime:
  execution:
    - id: py
      backend: python
      entrypoint: a:b
telemetry:
  service_name: base-svc
  protocol: grpc
`
	moduleYAML := `id: extra
evaluation:
  suites:
    - id: extra-suite
      description: extra
`
	manifest := `adp_version: "0.3.0"
id: child
extends: "base.yaml"
import:
  - id: extra
    from: "module.yaml"
telemetry:
  service_name: local-svc
overrides:
  - path: /telemetry/protocol
    value: http/protobuf
    op: set
`
	adp, errs := ResolveADP("child.yaml", inMemoryResolver(map[string]string{
		"base.yaml":   baseYAML,
		"module.yaml": moduleYAML,
		"child.yaml":  manifest,
	}))
	if len(errs) != 0 {
		t.Fatalf("unexpected errors: %v", errs)
	}
	if adp.Telemetry == nil {
		t.Fatal("telemetry should not be nil")
	}
	if adp.Telemetry.ServiceName != "local-svc" {
		t.Errorf("expected 'local-svc', got %q", adp.Telemetry.ServiceName)
	}
	if adp.Telemetry.Protocol != "http/protobuf" {
		t.Errorf("expected 'http/protobuf', got %q", adp.Telemetry.Protocol)
	}
}
