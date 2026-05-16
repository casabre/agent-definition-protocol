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

    // Conformance class enforcement
    adpMap, _ := json.Marshal(_adp)
    var adpRaw map[string]interface{}
    _ = json.Unmarshal(adpMap, &adpRaw)
    if cc, _ := adpRaw["conformance_class"].(string); cc == "full" {
        flow, _ := adpRaw["flow"].(map[string]interface{})
        if len(flow) == 0 {
            return fmt.Errorf("conformance_class 'full' declared but flow is empty")
        }
        eval, _ := adpRaw["evaluation"].(map[string]interface{})
        if len(eval) == 0 {
            return fmt.Errorf("conformance_class 'full' declared but evaluation is empty")
        }
    }

    schema, err := loadCompiledSchema()
    if err != nil {
        return fmt.Errorf("loading schema: %w", err)
    }
    var instance interface{}
    if err := json.Unmarshal(adpMap, &instance); err != nil {
        return fmt.Errorf("parsing adp json: %w", err)
    }
    if err := schema.Validate(instance); err != nil {
        return fmt.Errorf("schema validation failed: %w", err)
    }
    return nil
}

func ValidateADPSemantics(_adp *ADP) []string {
    var errors []string

    flowMap, ok := _adp.Flow.(map[string]interface{})
    if !ok {
        return errors
    }
    graphMap, ok := flowMap["graph"].(map[string]interface{})
    if !ok {
        return errors
    }

    nodesRaw, _ := graphMap["nodes"].([]interface{})
    edgesRaw, _ := graphMap["edges"].([]interface{})
    startNodesRaw, _ := graphMap["start_nodes"].([]interface{})
    endNodesRaw, _ := graphMap["end_nodes"].([]interface{})

    nodeIds := make(map[string]bool)
    for _, n := range nodesRaw {
        nodeMap, ok := n.(map[string]interface{})
        if !ok {
            continue
        }
        id, _ := nodeMap["id"].(string)
        if nodeIds[id] {
            errors = append(errors, fmt.Sprintf("duplicate node id '%s' in graph.nodes", id))
        }
        nodeIds[id] = true
    }

    for _, e := range edgesRaw {
        edgeMap, ok := e.(map[string]interface{})
        if !ok {
            continue
        }
        from, _ := edgeMap["from"].(string)
        to, _ := edgeMap["to"].(string)
        if !nodeIds[from] {
            errors = append(errors, fmt.Sprintf("edge from '%s' to '%s': node '%s' not found in graph.nodes", from, to, from))
        }
        if !nodeIds[to] {
            errors = append(errors, fmt.Sprintf("edge from '%s' to '%s': node '%s' not found in graph.nodes", from, to, to))
        }
    }

    for _, nid := range startNodesRaw {
        id, _ := nid.(string)
        if !nodeIds[id] {
            errors = append(errors, fmt.Sprintf("start_node '%s' not found in graph.nodes", id))
        }
    }
    for _, nid := range endNodesRaw {
        id, _ := nid.(string)
        if !nodeIds[id] {
            errors = append(errors, fmt.Sprintf("end_node '%s' not found in graph.nodes", id))
        }
    }

    evalMap, _ := _adp.Evaluation.(map[string]interface{})
    suitesRaw, _ := evalMap["suites"].([]interface{})
    suiteIds := make(map[string]bool)
    for _, s := range suitesRaw {
        sm, ok := s.(map[string]interface{})
        if !ok {
            continue
        }
        id, _ := sm["id"].(string)
        suiteIds[id] = true
    }

    modelIds := make(map[string]bool)
    for _, m := range _adp.Runtime.Models {
        modelIds[m.ID] = true
    }
    hasModels := len(_adp.Runtime.Models) > 0

    executionIds := make(map[string]bool)
    for _, e := range _adp.Runtime.Execution {
        executionIds[e.ID] = true
    }

    for _, n := range nodesRaw {
        nodeMap, ok := n.(map[string]interface{})
        if !ok {
            continue
        }
        nodeID, _ := nodeMap["id"].(string)

        if suiteRef, _ := nodeMap["suite_ref"].(string); suiteRef != "" {
            if !suiteIds[suiteRef] {
                errors = append(errors, fmt.Sprintf("node '%s' suite_ref '%s' not found in evaluation.suites", nodeID, suiteRef))
            }
        }
        if modelRef, _ := nodeMap["model_ref"].(string); modelRef != "" && hasModels {
            if !modelIds[modelRef] {
                errors = append(errors, fmt.Sprintf("node '%s' model_ref '%s' not found in runtime.models", nodeID, modelRef))
            }
        }
        if runtimeRef, _ := nodeMap["runtime_ref"].(string); runtimeRef != "" {
            if !executionIds[runtimeRef] {
                errors = append(errors, fmt.Sprintf("node '%s' runtime_ref '%s' not found in runtime.execution", nodeID, runtimeRef))
            }
        }
    }

    return errors
}
