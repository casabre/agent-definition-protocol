package adpkg

import (
    "os"
    "path/filepath"
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
}
