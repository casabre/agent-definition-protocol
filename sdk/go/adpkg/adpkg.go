package adpkg

import (
    "archive/zip"
    "io"
    "os"
    "path/filepath"

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
    out, err := os.Create(outPath)
    if err != nil {
        return err
    }
    defer out.Close()
    zipw := zip.NewWriter(out)
    defer zipw.Close()

    if err := addFile(zipw, adpPath, "adp/agent.yaml"); err != nil {
        return err
    }
    return nil
}

func addFile(zipw *zip.Writer, srcPath, dest string) error {
    f, err := os.Open(srcPath)
    if err != nil {
        return err
    }
    defer f.Close()
    w, err := zipw.Create(dest)
    if err != nil {
        return err
    }
    _, err = io.Copy(w, f)
    return err
}
