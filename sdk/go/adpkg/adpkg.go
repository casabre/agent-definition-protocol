package adpkg

import (
    "archive/tar"
    "crypto/sha256"
    "encoding/hex"
    "encoding/json"
    "fmt"
    "os"
    "path/filepath"
    "strings"
    "time"

    "github.com/casabre/adp-sdk/adp"
)

type ADPKG struct {
	Path string
}

func OpenADPKG(path string) (*ADPKG, error) {
	return &ADPKG{Path: path}, nil
}

func CreateADPKG(srcDir, outPath string) error {
    return CreateADPKGWithProvenance(srcDir, outPath, "", "", "")
}

func CreateADPKGWithProvenance(srcDir, outPath, builderID, sourceRepo, sourceRef string) error {
	adpPath := filepath.Join(srcDir, "adp", "agent.yaml")
	agent, err := adp.LoadADP(adpPath)
	if err != nil {
		return err
	}
	if err := adp.ValidateADP(agent); err != nil {
		return err
	}

	if err := os.MkdirAll(filepath.Join(outPath, "blobs", "sha256"), 0o755); err != nil {
		return err
	}

	// Config blob with provenance fields
	configMap := map[string]string{
		"agent_id":        agent.ID,
		"adp_version":     agent.ADPVersion,
		"builder.id":      builderID,
		"source.repo":     sourceRepo,
		"source.ref":      sourceRef,
		"build_timestamp": time.Now().UTC().Format(time.RFC3339),
	}
	config, err := json.Marshal(configMap)
	if err != nil {
		return err
	}
	configDigest := sha256Bytes(config)
	if err := writeBlob(outPath, configDigest, config); err != nil {
		return err
	}

	// Layer tar
	layerTar := filepath.Join(outPath, "layer.tar")
	if err := createTar(layerTar, srcDir); err != nil {
		return err
	}
	layerBytes, err := os.ReadFile(layerTar)
	if err != nil {
		return err
	}
	layerDigest := sha256Bytes(layerBytes)
	if err := writeBlob(outPath, layerDigest, layerBytes); err != nil {
		return err
	}
	_ = os.Remove(layerTar)

	manifest := map[string]interface{}{
		"schemaVersion": 2,
		"mediaType":     "application/vnd.oci.image.manifest.v1+json",
		"config": map[string]interface{}{
			"mediaType": "application/vnd.adp.config.v1+json",
			"digest":    configDigest,
			"size":      len(config),
		},
		"layers": []map[string]interface{}{
			{
				"mediaType": "application/vnd.adp.package.v1+tar",
				"digest":    layerDigest,
				"size":      len(layerBytes),
			},
		},
	}
	manifestBytes, err := json.MarshalIndent(manifest, "", "  ")
	if err != nil {
		return err
	}
	manifestDigest := sha256Bytes(manifestBytes)
	if err := writeBlob(outPath, manifestDigest, manifestBytes); err != nil {
		return err
	}

	index := map[string]interface{}{
		"schemaVersion": 2,
		"manifests": []map[string]interface{}{
			{
				"mediaType": "application/vnd.oci.image.manifest.v1+json",
				"digest":    manifestDigest,
				"size":      len(manifestBytes),
				"annotations": map[string]string{
					"org.opencontainers.image.title": agent.ID,
				},
			},
		},
	}
	indexBytes, err := json.MarshalIndent(index, "", "  ")
	if err != nil {
		return err
	}
	if err := os.WriteFile(filepath.Join(outPath, "index.json"), indexBytes, 0o644); err != nil {
		return err
	}
	if err := os.WriteFile(filepath.Join(outPath, "oci-layout"), []byte(`{"imageLayoutVersion":"1.0.0"}`), 0o644); err != nil {
		return err
	}
	return nil
}

// InspectResult holds structured metadata from an ADPKG.
type InspectResult struct {
    AgentID    string                 `json:"agent_id"`
    ADPVersion string                 `json:"adp_version"`
    LayerCount int                    `json:"layer_count"`
    Config     map[string]interface{} `json:"config"`
}

// VerifyResult holds the outcome of digest verification.
type VerifyResult struct {
    Passed   bool     `json:"passed"`
    Failures []string `json:"failures"`
}

func blobPath(root, digest string) string {
    parts := strings.Split(digest, ":")
    return filepath.Join(root, "blobs", parts[0], parts[1])
}

// Inspect returns structured metadata from the package without extracting the layer.
func Inspect(pkgDir string) (*InspectResult, error) {
    indexBytes, err := os.ReadFile(filepath.Join(pkgDir, "index.json"))
    if err != nil {
        return nil, err
    }
    var index map[string]interface{}
    if err := json.Unmarshal(indexBytes, &index); err != nil {
        return nil, err
    }
    manifestDesc := index["manifests"].([]interface{})[0].(map[string]interface{})
    manifestBytes, err := os.ReadFile(blobPath(pkgDir, manifestDesc["digest"].(string)))
    if err != nil {
        return nil, err
    }
    var manifest map[string]interface{}
    if err := json.Unmarshal(manifestBytes, &manifest); err != nil {
        return nil, err
    }
    configDigest := manifest["config"].(map[string]interface{})["digest"].(string)
    configBytes, err := os.ReadFile(blobPath(pkgDir, configDigest))
    if err != nil {
        return nil, err
    }
    var config map[string]interface{}
    if err := json.Unmarshal(configBytes, &config); err != nil {
        return nil, err
    }
    layerCount := 0
    if layers, ok := manifest["layers"].([]interface{}); ok {
        layerCount = len(layers)
    }
    return &InspectResult{
        AgentID:    fmt.Sprintf("%v", config["agent_id"]),
        ADPVersion: fmt.Sprintf("%v", config["adp_version"]),
        LayerCount: layerCount,
        Config:     config,
    }, nil
}

// Verify recomputes SHA-256 of each blob and compares it to the stored digest.
func Verify(pkgDir string) (*VerifyResult, error) {
    indexBytes, err := os.ReadFile(filepath.Join(pkgDir, "index.json"))
    if err != nil {
        return nil, err
    }
    var index map[string]interface{}
    if err := json.Unmarshal(indexBytes, &index); err != nil {
        return nil, err
    }
    var failures []string
    manifests, _ := index["manifests"].([]interface{})
    for _, entry := range manifests {
        desc := entry.(map[string]interface{})
        digest := desc["digest"].(string)
        verifyBlob(pkgDir, digest, &failures)
        manifestBytes, err := os.ReadFile(blobPath(pkgDir, digest))
        if err != nil {
            continue
        }
        var manifest map[string]interface{}
        if err := json.Unmarshal(manifestBytes, &manifest); err != nil {
            continue
        }
        if configDesc, ok := manifest["config"].(map[string]interface{}); ok {
            if d, ok := configDesc["digest"].(string); ok {
                verifyBlob(pkgDir, d, &failures)
            }
        }
        if layers, ok := manifest["layers"].([]interface{}); ok {
            for _, l := range layers {
                if layer, ok := l.(map[string]interface{}); ok {
                    if d, ok := layer["digest"].(string); ok {
                        verifyBlob(pkgDir, d, &failures)
                    }
                }
            }
        }
    }
    return &VerifyResult{Passed: len(failures) == 0, Failures: failures}, nil
}

func verifyBlob(pkgDir, digest string, failures *[]string) {
    data, err := os.ReadFile(blobPath(pkgDir, digest))
    if err != nil {
        *failures = append(*failures, digest+": blob file missing")
        return
    }
    actual := sha256Bytes(data)
    if actual != digest {
        *failures = append(*failures, digest+": digest mismatch (actual "+actual+")")
    }
}

func sha256Bytes(data []byte) string {
	h := sha256.Sum256(data)
	return "sha256:" + hex.EncodeToString(h[:])
}

func writeBlob(root, digest string, data []byte) error {
	parts := strings.Split(digest, ":")
	p := filepath.Join(root, "blobs", parts[0], parts[1])
	if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
		return err
	}
	return os.WriteFile(p, data, 0o644)
}

func createTar(dest, srcDir string) error {
	out, err := os.Create(dest)
	if err != nil {
		return err
	}
    defer out.Close()
    tw := tar.NewWriter(out)
    defer tw.Close()
    return filepath.Walk(srcDir, func(path string, info os.FileInfo, err error) error {
        if err != nil {
            return err
        }
        if info.IsDir() {
            return nil
        }
        rel, err := filepath.Rel(srcDir, path)
		if err != nil {
			return err
		}
		data, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		hdr, err := tar.FileInfoHeader(info, "")
		if err != nil {
			return err
		}
		hdr.Name = rel
		hdr.Size = int64(len(data))
		if err := tw.WriteHeader(hdr); err != nil {
			return err
		}
		_, err = tw.Write(data)
		return err
	})
}
