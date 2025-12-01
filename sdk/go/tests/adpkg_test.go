package tests

import (
	"archive/tar"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
	"strings"

	sdkadp "github.com/acme/adp-sdk/adp"
	"github.com/acme/adp-sdk/adpkg"
)

func buildSource(dir string) {
	os.MkdirAll(filepath.Join(dir, "adp"), 0o755)
	content := `adp_version: "0.1.0"
id: "agent.test"
runtime:
  execution:
    - backend: "python"
      id: "py"
      entrypoint: "agent.main:app"
flow: {}
evaluation:
  suites:
    - id: "s"
      metrics:
        - id: "m1"
          type: "deterministic"
          function: "noop"
          scoring: "boolean"
          threshold: "true"
`
	os.WriteFile(filepath.Join(dir, "adp", "agent.yaml"), []byte(content), 0o644)
}

func TestCreateOci(t *testing.T) {
	tmp, err := os.MkdirTemp("", "go-oci-*")
	if err != nil {
		t.Fatal(err)
	}
	buildSource(tmp)
	out := filepath.Join(tmp, "oci")
	if err := adpkg.CreateADPKG(tmp, out); err != nil {
		t.Fatal(err)
	}

	// index.json must exist and point to the manifest blob
	indexBytes, err := os.ReadFile(filepath.Join(out, "index.json"))
	if err != nil {
		t.Fatalf("missing index.json: %v", err)
	}
	var index map[string]interface{}
	if err := json.Unmarshal(indexBytes, &index); err != nil {
		t.Fatalf("invalid index: %v", err)
	}
	manifestDesc := index["manifests"].([]interface{})[0].(map[string]interface{})
	manifestDigest := manifestDesc["digest"].(string)
	manifestBytes, err := os.ReadFile(blobPath(out, manifestDigest))
	if err != nil {
		t.Fatalf("missing manifest blob: %v", err)
	}
	var manifest map[string]interface{}
	if err := json.Unmarshal(manifestBytes, &manifest); err != nil {
		t.Fatalf("invalid manifest: %v", err)
	}
	layerDesc := manifest["layers"].([]interface{})[0].(map[string]interface{})
	layerDigest := layerDesc["digest"].(string)
	layerFile, err := os.Open(blobPath(out, layerDigest))
	if err != nil {
		t.Fatalf("missing layer blob: %v", err)
	}
	tarReader := tar.NewReader(layerFile)
	foundAgent := false
	for {
		hdr, err := tarReader.Next()
		if err != nil {
			break
		}
		if hdr.Name == "adp/agent.yaml" {
			foundAgent = true
			break
		}
	}
	if !foundAgent {
		t.Fatal("agent.yaml not found in layer")
	}

	adpPath := filepath.Join(tmp, "adp", "agent.yaml")
	if _, err := sdkadp.LoadADP(adpPath); err != nil {
		t.Fatal(err)
	}
}

func TestCreateFailsWithoutAgent(t *testing.T) {
	tmp, _ := os.MkdirTemp("", "go-oci-missing-*")
	defer os.RemoveAll(tmp)
	out := filepath.Join(tmp, "oci")
	err := adpkg.CreateADPKG(tmp, out)
	if err == nil {
		t.Fatal("expected error when agent.yaml is missing")
	}
}

func TestValidateADPNegative(t *testing.T) {
	a := &sdkadp.ADP{ADPVersion: "x", Runtime: sdkadp.Runtime{Execution: []sdkadp.RuntimeEntry{}}}
	if err := sdkadp.ValidateADP(a); err == nil {
		t.Fatal("expected validation error")
	}
}

func TestLoadAndValidateADP(t *testing.T) {
	tmp, _ := os.MkdirTemp("", "go-adp-*")
	defer os.RemoveAll(tmp)
	buildSource(tmp)
	adpPath := filepath.Join(tmp, "adp", "agent.yaml")
	adpData, err := sdkadp.LoadADP(adpPath)
	if err != nil {
		t.Fatalf("failed to load: %v", err)
	}
	if adpData.ID != "agent.test" {
		t.Fatalf("unexpected id %s", adpData.ID)
	}
	if err := sdkadp.ValidateADP(adpData); err != nil {
		t.Fatalf("validate failed: %v", err)
	}
	_, err = sdkadp.LoadADP(filepath.Join(tmp, "missing.yaml"))
	if err == nil {
		t.Fatal("expected load error for missing file")
	}

	invalidPath := filepath.Join(tmp, "adp", "invalid.yaml")
	if err := os.WriteFile(invalidPath, []byte(":::not yaml"), 0o644); err != nil {
		t.Fatalf("write invalid yaml failed: %v", err)
	}
	if _, err := sdkadp.LoadADP(invalidPath); err == nil {
		t.Fatal("expected parse error for invalid yaml")
	}
}

// blobPath mirrors adpkg internal helper for test assertions
func blobPath(root, digest string) string {
	parts := strings.Split(digest, ":")
	return filepath.Join(root, "blobs", parts[0], parts[1])
}

func TestConfigDigestMatches(t *testing.T) {
	tmp, _ := os.MkdirTemp("", "go-oci-config-*")
	buildSource(tmp)
	out := filepath.Join(tmp, "oci")
	if err := adpkg.CreateADPKG(tmp, out); err != nil {
		t.Fatal(err)
	}
	cfg := `{"agent_id":"agent.test","adp_version":"0.1.0"}`
	sum := sha256.Sum256([]byte(cfg))
	expected := "sha256:" + hex.EncodeToString(sum[:])
	if _, err := os.Stat(blobPath(out, expected)); err != nil {
		t.Fatalf("expected config blob %s: %v", expected, err)
	}
}
