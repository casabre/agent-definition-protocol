package tests

import (
	"testing"

	"github.com/casabre/adp-sdk/adp"
)

func TestValidateFailsOnEmptyExecution(t *testing.T) {
	a := &adp.ADP{ADPVersion: "0.1.0", ID: "x", Runtime: adp.Runtime{Execution: []adp.RuntimeEntry{}}}
	if err := adp.ValidateADP(a); err == nil {
		t.Fatal("expected validation error")
	}
}

func TestValidatePasses(t *testing.T) {
	a := &adp.ADP{
		ADPVersion: "0.1.0",
		ID:         "x",
		Runtime:    adp.Runtime{Execution: []adp.RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "agent.main:app"}}},
		Flow: &adp.Flow{
			ID: "f",
			Graph: adp.Graph{
				Nodes:      []adp.Node{{ID: "n", Kind: adp.NodeKindInput}},
				Edges:      []adp.Edge{},
				StartNodes: []string{"n"},
				EndNodes:   []string{"n"},
			},
		},
		Evaluation: map[string]interface{}{
			"suites": []interface{}{map[string]interface{}{
				"id": "s",
				"metrics": []interface{}{map[string]interface{}{
					"id": "m", "type": "deterministic", "function": "noop",
					"scoring": "boolean", "threshold": true,
				}},
			}},
		},
	}
	if err := adp.ValidateADP(a); err != nil {
		t.Fatal(err)
	}
}
