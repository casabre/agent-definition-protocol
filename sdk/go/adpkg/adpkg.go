package adpkg

import (
    "encoding/json"
    "os"
    "path/filepath"
    "strings"

    "archive/tar"
    "crypto/sha256"
    "encoding/hex"

    "github.com/acme/adp-sdk/adp"
)

type ADPKG struct {
	Path string
}

func OpenADPKG(path string) (*ADPKG, error) {
	return &ADPKG{Path: path}, nil
}

func CreateADPKG(srcDir, outPath string) error {
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

	// Config blob
	config := []byte(`{"agent_id":"` + agent.ID + `","adp_version":"` + agent.ADPVersion + `"}`)
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
