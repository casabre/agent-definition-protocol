package adp

import "fmt"

func ValidateADP(_adp *ADP) error {
    if _adp.ADPVersion != "0.1.0" {
        return fmt.Errorf("adp_version must be 0.1.0")
    }
    if len(_adp.Runtime.Execution) == 0 {
        return fmt.Errorf("runtime.execution must not be empty")
    }
    return nil
}
