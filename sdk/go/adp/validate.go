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

// loadCompiledSchemaFn is the injectable function used by loadCompiledSchema.
// Tests can replace it to simulate schema load failures.
var loadCompiledSchemaFn = func() (*jsonschema.Schema, error) {
    return loadCompiledSchemaFromDir(schemaDir())
}

func loadCompiledSchemaFromDir(dir string) (*jsonschema.Schema, error) {
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
        _ = c.AddResource(id, strings.NewReader(string(data))) // cannot fail for valid JSON from strings.NewReader
    }
    adpID := "https://casabre.github.io/agent-definition-protocol/schemas/adp.schema.json"
    return c.Compile(adpID)
}

func loadCompiledSchema() (*jsonschema.Schema, error) {
    return loadCompiledSchemaFn()
}

func ValidateADP(_adp *ADP) error {
    if _adp.ADPVersion != "0.1.0" && _adp.ADPVersion != "0.2.0" && _adp.ADPVersion != "0.3.0" {
        return fmt.Errorf("adp_version must be 0.1.0, 0.2.0, or 0.3.0, got %s", _adp.ADPVersion)
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
    _ = json.Unmarshal(adpMap, &instance) // cannot fail: adpMap came from json.Marshal above
    if err := schema.Validate(instance); err != nil {
        return fmt.Errorf("schema validation failed: %w", err)
    }
    return nil
}

func ValidateADPSemantics(_adp *ADP) []string {
    var errors []string

    // Pre-composition guard: warn if unresolved composition directives are present.
    if _adp.Extends != "" || len(_adp.Imports) > 0 {
        errors = append(errors, "warning: manifest has unresolved composition directives (extends/import); consider running ResolveADP first")
    }

    // Check 7: guardrail policy_ref must be non-empty.
    if _adp.Guardrails != nil {
        for _, rail := range _adp.Guardrails.Input {
            if strings.TrimSpace(rail.PolicyRef) == "" {
                errors = append(errors, fmt.Sprintf("guardrail input '%s': policy_ref must not be empty", rail.ID))
            }
        }
        for _, rail := range _adp.Guardrails.Output {
            if strings.TrimSpace(rail.PolicyRef) == "" {
                errors = append(errors, fmt.Sprintf("guardrail output '%s': policy_ref must not be empty", rail.ID))
            }
        }
    }

    // Check 8: telemetry.required_attributes must match gen_ai.* or x_<vendor>.*
    if _adp.Telemetry != nil {
        for _, attr := range _adp.Telemetry.RequiredAttributes {
            if !strings.HasPrefix(attr, "gen_ai.") && !strings.HasPrefix(attr, "x_") {
                errors = append(errors, fmt.Sprintf("telemetry.required_attributes: %q must match 'gen_ai.*' or 'x_<vendor>.*'", attr))
            }
        }
    }

    // Check 9: tool auth env_var required when scheme != "none".
    toolIds := make(map[string]bool)
    if toolsMap, ok := _adp.Tools.(map[string]interface{}); ok {
        if httpAPIs, ok := toolsMap["http_apis"].([]interface{}); ok {
            for _, apiRaw := range httpAPIs {
                api, ok := apiRaw.(map[string]interface{})
                if !ok {
                    continue
                }
                toolID, _ := api["id"].(string)
                if toolID != "" {
                    toolIds[toolID] = true
                }
                auth, _ := api["auth"].(map[string]interface{})
                if auth == nil {
                    continue
                }
                scheme, _ := auth["scheme"].(string)
                if scheme != "" && scheme != "none" {
                    envVar, _ := auth["env_var"].(string)
                    if strings.TrimSpace(envVar) == "" {
                        errors = append(errors, fmt.Sprintf("tool '%s': auth.env_var is required when scheme is '%s'", toolID, scheme))
                    }
                }
            }
        }
    }

    // Check 10: compliance standard must be known or start with x_.
    knownCompliance := map[string]bool{
        "soc2": true, "iso27001": true, "hipaa": true, "gdpr": true, "pci_dss": true,
    }
    if complianceRaw, ok := _adp.Tools.(map[string]interface{}); ok {
        if stdRaw, ok := complianceRaw["compliance"].([]interface{}); ok {
            for _, s := range stdRaw {
                std, _ := s.(string)
                if !knownCompliance[std] && !strings.HasPrefix(std, "x_") {
                    errors = append(errors, fmt.Sprintf("unknown compliance standard %q (must be a known standard or start with 'x_')", std))
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Graph / flow checks
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

        // Check 11: tool_ref must reference an existing tool ID.
        if toolRef, _ := nodeMap["tool_ref"].(string); toolRef != "" {
            if !toolIds[toolRef] {
                errors = append(errors, fmt.Sprintf("node '%s' tool_ref '%s' not found in tools", nodeID, toolRef))
            }
        }
    }

    // Check 12: hooks[].node_filter entries must reference known flow node IDs.
    if hooksRaw, ok := _adp.Hooks.([]interface{}); ok {
        for _, h := range hooksRaw {
            hookMap, ok := h.(map[string]interface{})
            if !ok {
                continue
            }
            event, _ := hookMap["event"].(string)
            if event == "" {
                event = "?"
            }
            filterRaw, _ := hookMap["node_filter"].([]interface{})
            for _, f := range filterRaw {
                fid, _ := f.(string)
                if fid != "" && !nodeIds[fid] {
                    errors = append(errors, fmt.Sprintf(
                        "hook event '%s' node_filter '%s' does not reference a known flow node",
                        event, fid,
                    ))
                }
            }
        }
    }

    // Check 13: subflow node adp_ref (non-URI/path) must resolve to subagents[].id.
    subagentIds := make(map[string]bool)
    for _, s := range _adp.Subagents {
        subagentIds[s.ID] = true
    }
    for _, n := range nodesRaw {
        nodeMap, ok := n.(map[string]interface{})
        if !ok {
            continue
        }
        if kind, _ := nodeMap["kind"].(string); kind != "subflow" {
            continue
        }
        nodeID, _ := nodeMap["id"].(string)
        adpRef, _ := nodeMap["adp_ref"].(string)
        if adpRef == "" {
            continue
        }
        isURIOrPath := strings.Contains(adpRef, "://") ||
            strings.Contains(adpRef, "/") ||
            strings.HasSuffix(adpRef, ".yaml") ||
            strings.HasSuffix(adpRef, ".json")
        if !isURIOrPath && !subagentIds[adpRef] {
            errors = append(errors, fmt.Sprintf(
                "subflow node '%s' adp_ref '%s' does not resolve to a known subagents[] entry",
                nodeID, adpRef,
            ))
        }
    }

    // Check 14: evaluator_ref must resolve to known x_testing evaluator/judge ID.
    if _adp.XTesting != nil {
        testingEvaluatorIds := make(map[string]bool)
        if evsRaw, ok := _adp.XTesting["evaluators"].([]interface{}); ok {
            for _, ev := range evsRaw {
                evMap, ok := ev.(map[string]interface{})
                if !ok {
                    continue
                }
                if id, _ := evMap["id"].(string); id != "" {
                    testingEvaluatorIds[id] = true
                }
            }
        }
        if judgesRaw, ok := _adp.XTesting["judges"].([]interface{}); ok {
            for _, j := range judgesRaw {
                jMap, ok := j.(map[string]interface{})
                if !ok {
                    continue
                }
                if id, _ := jMap["id"].(string); id != "" {
                    testingEvaluatorIds[id] = true
                }
            }
        }
        if len(testingEvaluatorIds) > 0 {
            for _, s := range suitesRaw {
                sm, ok := s.(map[string]interface{})
                if !ok {
                    continue
                }
                metricsRaw, _ := sm["metrics"].([]interface{})
                for _, m := range metricsRaw {
                    mMap, ok := m.(map[string]interface{})
                    if !ok {
                        continue
                    }
                    evalRef, _ := mMap["evaluator_ref"].(string)
                    if evalRef == "" {
                        continue
                    }
                    if !testingEvaluatorIds[evalRef] {
                        metricID, _ := mMap["id"].(string)
                        if metricID == "" {
                            metricID = "?"
                        }
                        errors = append(errors, fmt.Sprintf(
                            "evaluator '%s' evaluator_ref '%s' does not resolve to a known x_testing evaluator",
                            metricID, evalRef,
                        ))
                    }
                }
            }
        }
        // Deprecation warning
        judgesSlice, _ := _adp.XTesting["judges"].([]interface{})
        evsSlice, _ := _adp.XTesting["evaluators"].([]interface{})
        if len(judgesSlice) > 0 && len(evsSlice) == 0 {
            errors = append(errors, "WARNING: x_testing.judges[] is deprecated; migrate to x_testing.evaluators[]")
        }
    }

    return errors
}
