package adpkg

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func buildSource(dir string) error {
	adpDir := filepath.Join(dir, "adp")
	if err := os.MkdirAll(adpDir, 0o755); err != nil {
		return err
	}
	return os.WriteFile(filepath.Join(adpDir, "agent.yaml"), []byte(
		"adp_version: \"0.1.0\"\nid: \"agent.test\"\nruntime:\n  execution:\n    - backend: \"python\"\n      id: \"py\"\n      entrypoint: \"agent.main:app\"\nflow: {}\nevaluation: {}\n"), 0o644)
}

func buildSourceWithMetadata(dir string) error {
	if err := buildSource(dir); err != nil {
		return err
	}
	metadataDir := filepath.Join(dir, "metadata")
	if err := os.MkdirAll(metadataDir, 0o755); err != nil {
		return err
	}
	metadata := map[string]string{
		"agent_id":      "agent.test",
		"agent_version": "1.0.0",
		"spec_version":  "0.1.0",
		"build_timestamp": "2024-01-15T10:30:00Z",
	}
	metadataJSON, _ := json.Marshal(metadata)
	return os.WriteFile(filepath.Join(metadataDir, "version.json"), metadataJSON, 0o644)
}

func TestCreateAndValidateOCI(t *testing.T) {
	tmp, err := os.MkdirTemp("", "go-adpkg-*")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(tmp)
	if err := buildSource(tmp); err != nil {
		t.Fatal(err)
	}
	out := filepath.Join(tmp, "oci")
	if err := CreateADPKG(tmp, out); err != nil {
		t.Fatalf("create failed: %v", err)
	}
	if _, err := os.Stat(filepath.Join(out, "index.json")); err != nil {
		t.Fatalf("missing index.json: %v", err)
	}
	if _, err := os.Stat(filepath.Join(out, "oci-layout")); err != nil {
		t.Fatalf("missing oci-layout: %v", err)
	}
	if _, err := os.Stat(filepath.Join(out, "blobs", "sha256")); err != nil {
		t.Fatalf("missing blobs/sha256 directory: %v", err)
	}
}

func TestOCILayoutStructure(t *testing.T) {
	tmp, err := os.MkdirTemp("", "go-adpkg-*")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(tmp)
	if err := buildSource(tmp); err != nil {
		t.Fatal(err)
	}
	out := filepath.Join(tmp, "oci")
	if err := CreateADPKG(tmp, out); err != nil {
		t.Fatalf("create failed: %v", err)
	}
	
	// Verify oci-layout content
	layoutPath := filepath.Join(out, "oci-layout")
	layoutData, err := os.ReadFile(layoutPath)
	if err != nil {
		t.Fatalf("failed to read oci-layout: %v", err)
	}
	var layout map[string]interface{}
	if err := json.Unmarshal(layoutData, &layout); err != nil {
		t.Fatalf("failed to parse oci-layout: %v", err)
	}
	if layout["imageLayoutVersion"] != "1.0.0" {
		t.Errorf("expected imageLayoutVersion '1.0.0', got '%v'", layout["imageLayoutVersion"])
	}
	
	// Verify index.json structure
	indexPath := filepath.Join(out, "index.json")
	indexData, err := os.ReadFile(indexPath)
	if err != nil {
		t.Fatalf("failed to read index.json: %v", err)
	}
	var index map[string]interface{}
	if err := json.Unmarshal(indexData, &index); err != nil {
		t.Fatalf("failed to parse index.json: %v", err)
	}
	manifests, ok := index["manifests"].([]interface{})
	if !ok {
		t.Fatal("index.json should contain manifests array")
	}
	if len(manifests) != 1 {
		t.Errorf("expected 1 manifest, got %d", len(manifests))
	}
	manifest := manifests[0].(map[string]interface{})
	if manifest["mediaType"] != "application/vnd.oci.image.manifest.v1+json" {
		t.Errorf("expected correct media type, got '%v'", manifest["mediaType"])
	}
}

func TestPackageContainsRequiredFiles(t *testing.T) {
	tmp, err := os.MkdirTemp("", "go-adpkg-*")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(tmp)
	if err := buildSource(tmp); err != nil {
		t.Fatal(err)
	}
	out := filepath.Join(tmp, "oci")
	if err := CreateADPKG(tmp, out); err != nil {
		t.Fatalf("create failed: %v", err)
	}
	
	// Read index and manifest
	indexPath := filepath.Join(out, "index.json")
	indexData, _ := os.ReadFile(indexPath)
	var index map[string]interface{}
	json.Unmarshal(indexData, &index)
	manifestDesc := index["manifests"].([]interface{})[0].(map[string]interface{})
	manifestDigest := manifestDesc["digest"].(string)
	
	// Extract digest hash (format: "sha256:hex" -> "sha256/hex")
	digestParts := strings.Split(manifestDigest, ":")
	if len(digestParts) != 2 {
		t.Fatalf("invalid digest format: %s", manifestDigest)
	}
	manifestPath := filepath.Join(out, "blobs", digestParts[0], digestParts[1])
	if _, err := os.Stat(manifestPath); err != nil {
		t.Fatalf("manifest blob should exist: %v", err)
	}
	
	// Read manifest
	manifestData, _ := os.ReadFile(manifestPath)
	var manifest map[string]interface{}
	json.Unmarshal(manifestData, &manifest)
	
	// Extract config digest hash
	configDesc := manifest["config"].(map[string]interface{})
	configDigest := configDesc["digest"].(string)
	configParts := strings.Split(configDigest, ":")
	if len(configParts) != 2 {
		t.Fatalf("invalid config digest format: %s", configDigest)
	}
	configPath := filepath.Join(out, "blobs", configParts[0], configParts[1])
	if _, err := os.Stat(configPath); err != nil {
		t.Fatalf("config blob should exist: %v", err)
	}
	
	// Verify config content
	configData, _ := os.ReadFile(configPath)
	var config map[string]interface{}
	json.Unmarshal(configData, &config)
	if config["agent_id"] != "agent.test" {
		t.Errorf("expected agent_id 'agent.test', got '%v'", config["agent_id"])
	}
	if config["adp_version"] != "0.1.0" {
		t.Errorf("expected adp_version '0.1.0', got '%v'", config["adp_version"])
	}
}

func TestPackageWithMetadata(t *testing.T) {
	tmp, err := os.MkdirTemp("", "go-adpkg-*")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(tmp)
	if err := buildSourceWithMetadata(tmp); err != nil {
		t.Fatal(err)
	}
	out := filepath.Join(tmp, "oci")
	if err := CreateADPKG(tmp, out); err != nil {
		t.Fatalf("create failed: %v", err)
	}
	
	// Verify package was created successfully
	if _, err := os.Stat(filepath.Join(out, "index.json")); err != nil {
		t.Fatalf("missing index.json: %v", err)
	}
}

func TestPackageErrorHandling(t *testing.T) {
	tmp, err := os.MkdirTemp("", "go-adpkg-missing-*")
	if err != nil {
		t.Fatal(err)
	}
	defer os.RemoveAll(tmp)
	
	// Missing adp/agent.yaml
	out := filepath.Join(tmp, "oci")
	if err := CreateADPKG(tmp, out); err == nil {
		t.Fatal("expected error for missing agent.yaml")
	}
}
