package adp

import (
    "encoding/json"
    "fmt"
    "os"
    "path/filepath"
    "runtime"
    "strings"

    jsonschema "github.com/santhosh-tekuri/jsonschema/v5"
    _ "github.com/santhosh-tekuri/jsonschema/v5/httploader"
)

func schemaDir() string {
    _, file, _, _ := runtime.Caller(0)
    return filepath.Join(filepath.Dir(file), "..", "..", "..", "schemas")
}

func loadCompiledSchema() (*jsonschema.Schema, error) {
    dir := schemaDir()
    c := jsonschema.NewCompiler()
    for _, name := range []string{"adp", "flow", "runtime", "evaluation"} {
        data, err := os.ReadFile(filepath.Join(dir, name+".schema.json"))
        if err != nil {
            return nil, fmt.Errorf("reading %s schema: %w", name, err)
        }
        var raw interface{}
        if err := json.Unmarshal(data, &raw); err != nil {
            return nil, fmt.Errorf("parsing %s schema: %w", name, err)
        }
        schemaMap := raw.(map[string]interface{})
        id, _ := schemaMap["$id"].(string)
        if id == "" {
            id = "file://" + filepath.Join(dir, name+".schema.json")
        }
        if err := c.AddResource(id, strings.NewReader(string(data))); err != nil {
            return nil, fmt.Errorf("adding %s schema resource: %w", name, err)
        }
    }
    adpID := "https://casabre.github.io/agent-definition-protocol/schemas/adp.schema.json"
    return c.Compile(adpID)
}

func ValidateADP(_adp *ADP) error {
    if _adp.ADPVersion != "0.1.0" {
        return fmt.Errorf("adp_version must be 0.1.0, got %s", _adp.ADPVersion)
    }
    if _adp.ID == "" {
        return fmt.Errorf("id must not be empty")
    }
    if len(_adp.Runtime.Execution) == 0 {
        return fmt.Errorf("runtime.execution must not be empty")
    }
    schema, err := loadCompiledSchema()
    if err != nil {
        return fmt.Errorf("loading schema: %w", err)
    }
    adpJSON, err := json.Marshal(_adp)
    if err != nil {
        return fmt.Errorf("marshaling adp: %w", err)
    }
    var instance interface{}
    if err := json.Unmarshal(adpJSON, &instance); err != nil {
        return fmt.Errorf("parsing adp json: %w", err)
    }
    if err := schema.Validate(instance); err != nil {
        return fmt.Errorf("schema validation failed: %w", err)
    }
    return nil
}
