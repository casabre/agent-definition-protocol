package adp

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"strings"
)

// EvaluationResult is the normalised output of any evaluator.
type EvaluationResult struct {
	Passed        bool                   `json:"passed"`
	Score         *float64               `json:"score"`
	Reason        string                 `json:"reason"`
	Metadata      map[string]interface{} `json:"metadata"`
	EvaluatorID   string                 `json:"evaluator_id"`
	EvaluatorType string                 `json:"evaluator_type"`
}

// EvaluatorError is returned when an evaluator cannot be loaded or run.
type EvaluatorError struct {
	Kind    string
	Message string
}

func (e *EvaluatorError) Error() string {
	return fmt.Sprintf("%s: %s", e.Kind, e.Message)
}

func unsupportedType(t string) *EvaluatorError {
	return &EvaluatorError{Kind: "UnsupportedEvaluatorType", Message: t}
}

// Evaluator runs a single evaluation against agent output.
type Evaluator interface {
	Evaluate(output, context map[string]interface{}) (*EvaluationResult, error)
}

// LoadEvaluator returns an Evaluator for the given config dict.
// Supported types: script (bash only).
// Deferred with helpful errors: deterministic, llm_judge, container.
func LoadEvaluator(config map[string]interface{}) (Evaluator, error) {
	evalType, _ := config["type"].(string)
	switch evalType {
	case "script":
		return newScriptEvaluator(config)
	case "deterministic":
		return nil, unsupportedType("deterministic: function_ref loading requires the Python or TypeScript SDK")
	case "llm_judge":
		return nil, unsupportedType("llm_judge: requires an LLM client; use the Python or TypeScript SDK")
	case "container":
		return nil, unsupportedType("container: deferred in the Go SDK; use the Python or TypeScript SDK")
	default:
		if evalType == "" {
			evalType = "(missing)"
		}
		return nil, unsupportedType(evalType)
	}
}

type scriptEvaluator struct {
	id        string
	runtime   string
	inline    string
	scriptRef string
}

func newScriptEvaluator(config map[string]interface{}) (*scriptEvaluator, error) {
	id, _ := config["id"].(string)
	runtime, _ := config["runtime"].(string)
	if runtime == "" {
		return nil, &EvaluatorError{Kind: "MissingField", Message: "runtime"}
	}
	if runtime != "bash" {
		return nil, unsupportedType(fmt.Sprintf("script runtime '%s': only 'bash' is supported in the Go SDK", runtime))
	}
	inline, _ := config["inline"].(string)
	scriptRef, _ := config["script_ref"].(string)
	if inline == "" && scriptRef == "" {
		return nil, &EvaluatorError{Kind: "MissingField", Message: "inline or script_ref"}
	}
	return &scriptEvaluator{id: id, runtime: runtime, inline: inline, scriptRef: scriptRef}, nil
}

func (s *scriptEvaluator) resolveScript() (string, error) {
	if s.inline != "" {
		return s.inline, nil
	}
	if strings.HasPrefix(s.scriptRef, "git+") {
		return "", unsupportedType("git-pinned script_ref is not supported in the Go SDK")
	}
	data, err := os.ReadFile(s.scriptRef)
	if err != nil {
		return "", &EvaluatorError{Kind: "RuntimeError", Message: err.Error()}
	}
	return string(data), nil
}

func (s *scriptEvaluator) Evaluate(output, context map[string]interface{}) (*EvaluationResult, error) {
	script, err := s.resolveScript()
	if err != nil {
		return nil, err
	}

	input, _ := json.Marshal(map[string]interface{}{"output": output, "context": context})

	cmd := exec.Command("/bin/bash", "-c", script)
	cmd.Stdin = bytes.NewReader(input)
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	if runErr := cmd.Run(); runErr != nil {
		return &EvaluationResult{
			Passed:        false,
			Reason:        fmt.Sprintf("bash error: %s", stderr.String()),
			Metadata:      map[string]interface{}{},
			EvaluatorID:   s.id,
			EvaluatorType: "script",
		}, nil
	}

	var raw interface{}
	if jsonErr := json.Unmarshal(bytes.TrimSpace(stdout.Bytes()), &raw); jsonErr != nil {
		return nil, &EvaluatorError{Kind: "RuntimeError", Message: fmt.Sprintf("failed to parse output: %v", jsonErr)}
	}
	return normalizeResult(raw, s.id, "script"), nil
}

func normalizeResult(raw interface{}, id, evalType string) *EvaluationResult {
	base := &EvaluationResult{
		Metadata:      map[string]interface{}{},
		EvaluatorID:   id,
		EvaluatorType: evalType,
	}
	switch v := raw.(type) {
	case bool:
		score := 0.0
		if v {
			score = 1.0
		}
		base.Passed = v
		base.Score = &score
	case map[string]interface{}:
		if p, ok := v["passed"].(bool); ok {
			base.Passed = p
		} else if s, ok := v["score"].(float64); ok {
			base.Passed = s >= 0.5
		}
		if s, ok := v["score"].(float64); ok {
			base.Score = &s
		}
		if r, ok := v["reason"].(string); ok {
			base.Reason = r
		}
	default:
		if b, ok := raw.(bool); ok {
			base.Passed = b
		}
	}
	return base
}
