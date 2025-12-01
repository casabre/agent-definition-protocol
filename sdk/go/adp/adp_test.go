package adp

import (
	"os"
	"path/filepath"
	"testing"
)

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
flow: {}
evaluation: {}
`)
	if _, err := LoadADP(path); err != nil {
		t.Fatalf("load failed: %v", err)
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
		Flow:       map[string]interface{}{},
		Evaluation: map[string]interface{}{},
	}
	if err := ValidateADP(valid); err != nil {
		t.Fatalf("unexpected validation error: %v", err)
	}
	invalid := &ADP{ADPVersion: "0.0.1", Runtime: Runtime{Execution: []RuntimeEntry{}}}
	if err := ValidateADP(invalid); err == nil {
		t.Fatal("expected validation error")
	}
}
