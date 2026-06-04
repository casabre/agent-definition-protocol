package adp

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
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
	for _, name := range []string{"adp", "flow", "runtime", "evaluation", "memory", "workspace", "sandbox", "artifacts", "observability"} {
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
	if _adp.Tools != nil {
		for _, api := range _adp.Tools.HTTPAPIs {
			toolIds[api.ID] = true
			if api.Auth != nil {
				schemeIsNone := api.Auth.Scheme == AuthSchemeNone || api.Auth.Scheme == ""
				if !schemeIsNone {
					if strings.TrimSpace(api.Auth.EnvVar) == "" {
						errors = append(errors, fmt.Sprintf("tool '%s': auth.env_var is required when scheme is '%s'", api.ID, api.Auth.Scheme))
					}
				}
			}
		}
		for _, mcp := range _adp.Tools.MCPservers {
			toolIds[mcp.ID] = true
			if mcp.Auth != nil {
				schemeIsNone := mcp.Auth.Scheme == AuthSchemeNone || mcp.Auth.Scheme == ""
				if !schemeIsNone {
					if strings.TrimSpace(mcp.Auth.EnvVar) == "" {
						errors = append(errors, fmt.Sprintf("tool '%s': auth.env_var is required when scheme is '%s'", mcp.ID, mcp.Auth.Scheme))
					}
				}
			}
		}
		for _, sql := range _adp.Tools.SQLFunctions {
			toolIds[sql.ID] = true
			if sql.Auth != nil {
				schemeIsNone := sql.Auth.Scheme == AuthSchemeNone || sql.Auth.Scheme == ""
				if !schemeIsNone {
					if strings.TrimSpace(sql.Auth.EnvVar) == "" {
						errors = append(errors, fmt.Sprintf("tool '%s': auth.env_var is required when scheme is '%s'", sql.ID, sql.Auth.Scheme))
					}
				}
			}
		}
	}

	// Check 10: compliance standard must be known or start with x_.
	// Note: compliance is in governance, not tools - skipping for now as governance uses interface{}

	// -----------------------------------------------------------------------
	// Build node ID set early — needed by both agentspec and flow checks.
	nodeIds := make(map[string]bool)
	if _adp.Flow != nil {
		for _, n := range _adp.Flow.Graph.Nodes {
			if nodeIds[n.ID] {
				errors = append(errors, fmt.Sprintf("duplicate node id '%s' in graph.nodes", n.ID))
			}
			nodeIds[n.ID] = true
		}
	}

	// --- AgentSpec Interop Checks (AS-1, AS-2) ---
	// Placed before the flow early-return so AS-2 (runtime-only) always runs.
	if _adp.Interop != nil && _adp.Interop.AgentSpec != nil {
		as := _adp.Interop.AgentSpec

		// Check AS-1: interop.agentspec.node_map keys must match node IDs in flow.graph.nodes
		if as.NodeMap != nil && _adp.Flow != nil {
			for mappedNodeID := range as.NodeMap {
				if !nodeIds[mappedNodeID] {
					errors = append(errors, fmt.Sprintf(
						"interop.agentspec.node_map: key '%s' does not match any node id in flow.graph.nodes",
						mappedNodeID,
					))
				}
			}
		}

		// Check AS-2: interop.agentspec.llm_map[].backend_id must match runtime.execution[].id
		if as.LLMMap != nil {
			backendIds := make(map[string]bool)
			for _, entry := range _adp.Runtime.Execution {
				backendIds[entry.ID] = true
			}
			for _, binding := range as.LLMMap {
				if binding.BackendID != "" && !backendIds[binding.BackendID] {
					errors = append(errors, fmt.Sprintf(
						"interop.agentspec.llm_map: backend_id '%s' does not match any id in runtime.execution",
						binding.BackendID,
					))
				}
			}
		}

		// Check AS-3: interop.agentspec.ref MUST NOT contain path traversal sequences
		if as.Ref != "" && strings.Contains(as.Ref, "..") {
			errors = append(errors, fmt.Sprintf(
				"interop.agentspec.ref '%s' MUST NOT contain path traversal sequences (..)",
				as.Ref,
			))
		}
	}

	// -----------------------------------------------------------------------
	// Graph / flow checks
	if _adp.Flow == nil {
		return errors
	}

	for _, e := range _adp.Flow.Graph.Edges {
		if !nodeIds[e.From] {
			errors = append(errors, fmt.Sprintf("edge from '%s' to '%s': node '%s' not found in graph.nodes", e.From, e.To, e.From))
		}
		if !nodeIds[e.To] {
			errors = append(errors, fmt.Sprintf("edge from '%s' to '%s': node '%s' not found in graph.nodes", e.From, e.To, e.To))
		}
	}

	for _, nid := range _adp.Flow.Graph.StartNodes {
		if !nodeIds[nid] {
			errors = append(errors, fmt.Sprintf("start_node '%s' not found in graph.nodes", nid))
		}
	}
	for _, nid := range _adp.Flow.Graph.EndNodes {
		if !nodeIds[nid] {
			errors = append(errors, fmt.Sprintf("end_node '%s' not found in graph.nodes", nid))
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

	for _, n := range _adp.Flow.Graph.Nodes {
		nodeID := n.ID

		if n.SuiteRef != "" {
			if !suiteIds[n.SuiteRef] {
				errors = append(errors, fmt.Sprintf("node '%s' suite_ref '%s' not found in evaluation.suites", nodeID, n.SuiteRef))
			}
		}
		if n.ModelRef != "" && hasModels {
			if !modelIds[n.ModelRef] {
				errors = append(errors, fmt.Sprintf("node '%s' model_ref '%s' not found in runtime.models", nodeID, n.ModelRef))
			}
		}
		if n.RuntimeRef != "" {
			if !executionIds[n.RuntimeRef] {
				errors = append(errors, fmt.Sprintf("node '%s' runtime_ref '%s' not found in runtime.execution", nodeID, n.RuntimeRef))
			}
		}

		// Check 11: tool_ref must reference an existing tool ID.
		if n.ToolRef != "" {
			if !toolIds[n.ToolRef] {
				errors = append(errors, fmt.Sprintf("node '%s' tool_ref '%s' not found in tools", nodeID, n.ToolRef))
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
	for _, n := range _adp.Flow.Graph.Nodes {
		if n.Kind != NodeKindSubflow {
			continue
		}
		nodeID := n.ID
		adpRef := n.AdpRef
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

	// =========================================================================
	// v0.3.0 Semantic Validation Checks (15-35b)
	// =========================================================================

	// --- Loop Checks (15-16) ---

	// Check 15: loop.body_nodes[] must reference known node IDs in flow.graph.nodes[]
	for _, n := range _adp.Flow.Graph.Nodes {
		if n.Kind != NodeKindLoop {
			continue
		}
		for _, bodyNodeID := range n.BodyNodes {
			if !nodeIds[bodyNodeID] {
				errors = append(errors, fmt.Sprintf(
					"loop node '%s': body_nodes references '%s' which is not found in graph.nodes",
					n.ID, bodyNodeID,
				))
			}
		}
	}

	// Check 15b: loop.body_nodes[] must contain at least 2 nodes connected by
	// at least one edge in flow.graph.edges[]
	for _, n := range _adp.Flow.Graph.Nodes {
		if n.Kind != NodeKindLoop {
			continue
		}
		bodyNodes := n.BodyNodes
		if len(bodyNodes) >= 2 {
			// Build adjacency from edges
			edgeMap := make(map[string]map[string]bool)
			for _, e := range _adp.Flow.Graph.Edges {
				if _, ok := edgeMap[e.From]; !ok {
					edgeMap[e.From] = make(map[string]bool)
				}
				edgeMap[e.From][e.To] = true
			}
			// Check if any body node connects to another body node
			hasConnection := false
			for _, nodeID := range bodyNodes {
				if targets, ok := edgeMap[nodeID]; ok {
					for target := range targets {
						if nodeIds[target] {
							for _, bn := range bodyNodes {
								if target == bn {
									hasConnection = true
									break
								}
							}
							if hasConnection {
								break
							}
						}
					}
					if hasConnection {
						break
					}
				}
			}
			if !hasConnection {
				errors = append(errors, fmt.Sprintf(
					"loop node '%s': body_nodes must contain at least 2 nodes connected by at least one edge",
					n.ID,
				))
			}
		}
	}

	// Check 16: Loop node MUST NOT reference itself (directly or transitively) in body_nodes
	for _, n := range _adp.Flow.Graph.Nodes {
		if n.Kind != NodeKindLoop {
			continue
		}
		loopID := n.ID
		for _, bodyNodeID := range n.BodyNodes {
			if bodyNodeID == loopID {
				errors = append(errors, fmt.Sprintf(
					"loop node '%s': body_nodes MUST NOT reference the loop node itself",
					loopID,
				))
			}
			// Check transitive: if body_node is a loop that references this loop
			for _, bodyNode := range _adp.Flow.Graph.Nodes {
				if bodyNode.ID != bodyNodeID {
					continue
				}
				if bodyNode.Kind == NodeKindLoop {
					for _, nestedBody := range bodyNode.BodyNodes {
						if nestedBody == loopID {
							errors = append(errors, fmt.Sprintf(
								"loop node '%s': circular loop reference detected with '%s'",
								loopID, bodyNodeID,
							))
						}
					}
				}
			}
		}
	}

	// --- Tools Policy Checks (17, 29) ---

	// Check 17: policy.cache.key_fields[] entries MUST use dot-path notation
	dotPathRe := regexp.MustCompile(`^[a-zA-Z_][a-zA-Z0-9_]*(\.[a-zA-Z_][a-zA-Z0-9_]*)*$`)
	if _adp.Tools != nil {
		checkToolCacheKeyFields := func(toolID string, policy *ToolPolicy) {
			if policy == nil || policy.Cache == nil {
				return
			}
			for _, field := range policy.Cache.KeyFields {
				if !dotPathRe.MatchString(field) {
					errors = append(errors, fmt.Sprintf(
						"tool '%s': cache.key_fields entry '%s' must use dot-path notation",
						toolID, field,
					))
				}
			}
		}
		for _, api := range _adp.Tools.HTTPAPIs {
			if api.Policy != nil {
				checkToolCacheKeyFields(api.ID, api.Policy)
			}
		}
		for _, sql := range _adp.Tools.SQLFunctions {
			if sql.Policy != nil {
				checkToolCacheKeyFields(sql.ID, sql.Policy)
			}
		}
		// Check global tools policy
		if _adp.Tools.Policy != nil {
			for _, field := range _adp.Tools.Policy.Cache.KeyFields {
				if !dotPathRe.MatchString(field) {
					errors = append(errors, fmt.Sprintf(
						"tools.policy.cache.key_fields entry '%s' must use dot-path notation",
						field,
					))
				}
			}
		}
	}

	// Check 29: Any tool with load_strategy: "on_demand" MUST have a non-empty description
	if _adp.Tools != nil {
		checkOnDemandDescription := func(toolID string, loadStrategy LoadStrategy, description string) {
			if loadStrategy == LoadStrategyOnDemand {
				if strings.TrimSpace(description) == "" {
					errors = append(errors, fmt.Sprintf(
						"tool '%s': load_strategy 'on_demand' requires a non-empty description",
						toolID,
					))
				}
			}
		}
		for _, api := range _adp.Tools.HTTPAPIs {
			var ls LoadStrategy
			if api.Policy != nil && api.Policy.LoadStrategy != "" {
				ls = api.Policy.LoadStrategy
			}
			checkOnDemandDescription(api.ID, ls, api.Description)
		}
		for _, sql := range _adp.Tools.SQLFunctions {
			var ls LoadStrategy
			if sql.Policy != nil && sql.Policy.LoadStrategy != "" {
				ls = sql.Policy.LoadStrategy
			}
			checkOnDemandDescription(sql.ID, ls, sql.Description)
		}
	}

	// --- Memory Checks (18-21c, 24) ---

	// Check 18: memory.stores[] IDs must be unique (post-composition)
	// Check 19: memory.operations[].store_ref must reference a known stores[].id
	// Check 20: memory.context_assembly.order[].store_ref must reference a known stores[].id
	// Check 24: memory.context_assembly.static_injection[].path (when source: "file")
	if _adp.Memory != nil && _adp.Memory.Structured != nil {
		mem := _adp.Memory.Structured
		if mem.Stores != nil {
			storeIds := make(map[string]bool)
			for _, store := range mem.Stores {
				if storeIds[store.ID] {
					errors = append(errors, fmt.Sprintf("memory: duplicate store id '%s'", store.ID))
				}
				storeIds[store.ID] = true
			}

			// Check 19
			for _, op := range mem.Operations {
				if op.StoreRef != "" && !storeIds[op.StoreRef] {
					errors = append(errors, fmt.Sprintf(
						"memory.operations: store_ref '%s' not found in memory.stores",
						op.StoreRef,
					))
				}
			}

			// Check 20: memory.context_assembly.order[].store_ref must reference a known stores[].id
			if mem.ContextAssembly != nil && mem.ContextAssembly.Order != nil {
				for _, item := range mem.ContextAssembly.Order {
					if item.Source == ContextAssemblySourceStore && item.StoreRef != "" && !storeIds[item.StoreRef] {
						errors = append(errors, fmt.Sprintf(
							"memory.context_assembly: store_ref '%s' not found in memory.stores",
							item.StoreRef,
						))
					}
				}
			}

			// Check 24: memory.context_assembly.static_injection[].path (when source: "file")
			// must be a relative path without .. traversal; must also reference a declared workspace
			if mem.ContextAssembly != nil && mem.ContextAssembly.StaticInjection != nil {
				hasWorkspace := _adp.Workspace != nil
				for _, si := range mem.ContextAssembly.StaticInjection {
					if si.Source == "file" && si.Path != "" {
						if strings.Contains(si.Path, "..") || strings.HasPrefix(si.Path, "/") {
							errors = append(errors, fmt.Sprintf(
								"memory.context_assembly.static_injection: path '%s' must be a relative path without .. traversal",
								si.Path,
							))
						}
						if !hasWorkspace {
							errors = append(errors, fmt.Sprintf(
								"memory.context_assembly.static_injection: path '%s' requires a workspace section to be declared",
								si.Path,
							))
						}
					}
				}
			}
		}
	}

	// Check 21: memory.working.summary_model_ref (when present) must reference a known runtime.models[].id
	// Check 21b: memory.working.summary_model_ref MUST be present when memory.working.strategy = "summary"
	// Check 21c: memory.working.compaction_threshold_tokens (when present) MUST be <= memory.working.max_tokens
	if _adp.Memory != nil && _adp.Memory.Structured != nil && _adp.Memory.Structured.Working != nil {
		w := _adp.Memory.Structured.Working
		// Check 21
		if w.SummaryModelRef != "" && hasModels && !modelIds[w.SummaryModelRef] {
			errors = append(errors, fmt.Sprintf(
				"memory.working.summary_model_ref '%s' not found in runtime.models",
				w.SummaryModelRef,
			))
		}
		// Check 21b
		if w.Strategy == "summary" && w.SummaryModelRef == "" {
			errors = append(errors, "memory.working: summary_model_ref MUST be present when strategy is 'summary'")
		}
		// Check 21c
		if w.CompactionThresholdTokens != nil && w.MaxTokens != nil {
			if *w.CompactionThresholdTokens > *w.MaxTokens {
				errors = append(errors, fmt.Sprintf(
					"memory.working: compaction_threshold_tokens (%d) MUST be <= max_tokens (%d)",
					*w.CompactionThresholdTokens, *w.MaxTokens,
				))
			}
		}
	}

	// --- Guardrails Checks (22-23, 30) ---

	if _adp.Guardrails != nil {
		// Collect all tool IDs
		allToolIdsAllTypes := make(map[string]bool)
		for k, v := range toolIds {
			allToolIdsAllTypes[k] = v
		}

		// Check 22: guardrails.interrupts[].tool_refs[] must reference known tool IDs
		// Check 22b: guardrails.interrupts[].execution_mode MUST NOT be set when mode: "pause_and_notify"
		if _adp.Guardrails.Interrupts != nil {
			for _, interrupt := range _adp.Guardrails.Interrupts {
				// Check 22
				for _, toolRef := range interrupt.ToolRefs {
					if !allToolIdsAllTypes[toolRef] {
						errors = append(errors, fmt.Sprintf(
							"guardrails.interrupts: tool_ref '%s' not found in tools",
							toolRef,
						))
					}
				}
				// Check 22b
				if interrupt.Mode == "pause_and_notify" && interrupt.ExecutionMode != "" {
					errors = append(errors, fmt.Sprintf(
						"guardrails.interrupts '%s': execution_mode MUST NOT be set when mode is 'pause_and_notify'",
						interrupt.ID,
					))
				}
			}
			interruptIds := make(map[string]bool)
			for _, interrupt := range _adp.Guardrails.Interrupts {
				interruptIds[interrupt.ID] = true
			}

			// Check 23: guardrails.cost.interrupt_ref (when present) must reference a known guardrails.interrupts[].id
			if _adp.Guardrails.Cost != nil && _adp.Guardrails.Cost.InterruptRef != "" {
				if !interruptIds[_adp.Guardrails.Cost.InterruptRef] {
					errors = append(errors, fmt.Sprintf(
						"guardrails.cost.interrupt_ref '%s' not found in guardrails.interrupts",
						_adp.Guardrails.Cost.InterruptRef,
					))
				}
			}

			// Check 30: guardrails.cost.downgrade_model_ref MUST be present when
			// on_threshold_exceeded: "downgrade"; it MUST reference a known runtime.models[].id
			if _adp.Guardrails.Cost != nil {
				cost := _adp.Guardrails.Cost
				if cost.OnThresholdExceeded == "downgrade" {
					if cost.DowngradeModelRef == "" {
						errors = append(errors, "guardrails.cost: downgrade_model_ref MUST be present when on_threshold_exceeded is 'downgrade'")
					} else if hasModels && !modelIds[cost.DowngradeModelRef] {
						errors = append(errors, fmt.Sprintf(
							"guardrails.cost.downgrade_model_ref '%s' not found in runtime.models",
							cost.DowngradeModelRef,
						))
					}
				}
			}
		}
	}

	// --- Workspace Checks (25-26, 31) ---

	if _adp.Workspace != nil {
		w := _adp.Workspace

		// Check 25: workspace.permissions.write[] paths MUST NOT escape workspace.root (no .. traversal)
		if w.Permissions != nil && w.Permissions.Write != nil {
			for _, path := range w.Permissions.Write {
				if strings.Contains(path, "..") {
					errors = append(errors, fmt.Sprintf(
						"workspace.permissions.write: path '%s' MUST NOT escape workspace.root",
						path,
					))
				}
			}
		}

		// Check 25b: Exactly one of workspace.root or workspace.root_env_var MUST be present
		if w.Root != "" && w.RootEnvVar != "" {
			errors = append(errors, "workspace: exactly one of 'root' or 'root_env_var' MUST be present, not both")
		}
		if w.Root == "" && w.RootEnvVar == "" {
			errors = append(errors, "workspace: exactly one of 'root' or 'root_env_var' MUST be present")
		}

		// Check 26: workspace.git.auto_commit: true requires workspace.git.enabled: true
		if w.Git != nil {
			if w.Git.AutoCommit != nil && *w.Git.AutoCommit {
				if w.Git.Enabled == nil || !*w.Git.Enabled {
					errors = append(errors, "workspace.git: auto_commit requires enabled to be true")
				}
			}
		}

		// Check 31: workspace.mounts[].id values must be unique;
		// workspace.mounts[].target paths MUST NOT escape workspace.root
		if w.Mounts != nil {
			mountIds := make(map[string]bool)
			for _, mount := range w.Mounts {
				if mountIds[mount.ID] {
					errors = append(errors, fmt.Sprintf("workspace.mounts: duplicate mount id '%s'", mount.ID))
				}
				mountIds[mount.ID] = true
				if strings.Contains(mount.Target, "..") {
					errors = append(errors, fmt.Sprintf(
						"workspace.mounts: target path '%s' MUST NOT escape workspace.root",
						mount.Target,
					))
				}
			}
		}
	}

	// --- Sandbox Checks (27-28, 32) ---

	if _adp.Sandbox != nil {
		s := _adp.Sandbox

		// Check 27: sandbox.policy.timeout_ms MUST be present (no unbounded sandbox execution)
		if s.Policy == nil {
			errors = append(errors, "sandbox.policy MUST be present (no unbounded sandbox execution)")
		} else if s.Policy.TimeoutMs == nil {
			errors = append(errors, "sandbox.policy.timeout_ms MUST be present (no unbounded sandbox execution)")
		}

		// Check 28: sandbox.mounts[].source: "workspace" requires a workspace section to be declared
		hasWorkspace := _adp.Workspace != nil
		if s.Mounts != nil {
			for _, mount := range s.Mounts {
				if mount.Source.Workspace != "" && !hasWorkspace {
					errors = append(errors, "sandbox.mounts: source 'workspace' requires a workspace section to be declared")
				}
			}
		}

		// Check 32: sandbox.snapshot.enabled: true with provider: "custom" emits a WARNING
		if s.Snapshot != nil && s.Snapshot.Enabled != nil && *s.Snapshot.Enabled && s.Provider == "custom" {
			errors = append(errors, "WARNING: sandbox: snapshot.enabled with provider 'custom' may not be supported")
		}
	}

	// --- Artifacts Checks (33-34) ---

	if _adp.Artifacts != nil {
		// Check 33: artifacts.stores[].id must be unique
		if _adp.Artifacts.Stores != nil {
			artifactStoreIds := make(map[string]bool)
			for _, store := range _adp.Artifacts.Stores {
				if artifactStoreIds[store.ID] {
					errors = append(errors, fmt.Sprintf("artifacts.stores: duplicate store id '%s'", store.ID))
				}
				artifactStoreIds[store.ID] = true
			}

			// Check 34: nodes[].params.artifact.store_ref must reference a known artifacts.stores[].id
			for _, n := range _adp.Flow.Graph.Nodes {
				if n.Params == nil {
					continue
				}
				paramsMap, ok := n.Params.(map[string]interface{})
				if !ok {
					continue
				}
				artifactRaw, ok := paramsMap["artifact"].(map[string]interface{})
				if !ok {
					continue
				}
				storeRef, _ := artifactRaw["store_ref"].(string)
				if storeRef != "" && !artifactStoreIds[storeRef] {
					errors = append(errors, fmt.Sprintf(
						"node '%s' params.artifact.store_ref '%s' not found in artifacts.stores",
						n.ID, storeRef,
					))
				}
			}
		}
	}

	// --- Observability Checks (35-35b) ---

	if _adp.Observability != nil {
		o := _adp.Observability

		// Check 35: observability.tracing.trace_events[] entries must be from the valid enum
		// Valid: model_request, tool_call, flow_node, loop_iteration, interrupt, cost_check, artifact_write
		validTraceEvents := map[string]bool{
			"model_request":  true,
			"tool_call":      true,
			"flow_node":      true,
			"loop_iteration": true,
			"interrupt":      true,
			"cost_check":     true,
			"artifact_write": true,
		}
		if o.Tracing != nil && o.Tracing.TraceEvents != nil {
			for _, event := range o.Tracing.TraceEvents {
				if !validTraceEvents[string(event)] {
					errors = append(errors, fmt.Sprintf(
						"observability.tracing.trace_events: '%s' is not a valid trace event",
						event,
					))
				}
			}
		}

		// Check 35b: observability.cost_reporting.model_refs[] (when present) must
		// reference known runtime.models[].id values
		if o.CostReporting != nil && o.CostReporting.ModelRefs != nil {
			for _, modelRef := range o.CostReporting.ModelRefs {
				if hasModels && !modelIds[modelRef] {
					errors = append(errors, fmt.Sprintf(
						"observability.cost_reporting.model_refs: '%s' not found in runtime.models",
						modelRef,
					))
				}
			}
		}
	}

	return errors
}
