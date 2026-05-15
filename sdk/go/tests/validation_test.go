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
        Flow: map[string]interface{}{
            "id": "f",
            "graph": map[string]interface{}{
                "nodes":       []interface{}{map[string]interface{}{"id": "n", "kind": "input"}},
                "edges":       []interface{}{},
                "start_nodes": []interface{}{"n"},
                "end_nodes":   []interface{}{"n"},
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
