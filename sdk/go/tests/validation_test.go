package tests

import (
    "testing"

    "github.com/acme/adp-sdk/adp"
)

func TestValidateFailsOnEmptyExecution(t *testing.T) {
    a := &adp.ADP{ADPVersion: "0.1.0", ID: "x", Runtime: adp.Runtime{Execution: []adp.RuntimeEntry{}}}
    if err := adp.ValidateADP(a); err == nil {
        t.Fatal("expected validation error")
    }
}

func TestValidatePasses(t *testing.T) {
    a := &adp.ADP{ADPVersion: "0.1.0", ID: "x", Runtime: adp.Runtime{Execution: []adp.RuntimeEntry{{Backend: "python", ID: "py", Entrypoint: "agent.main:app"}}}}
    if err := adp.ValidateADP(a); err != nil {
        t.Fatal(err)
    }
}
