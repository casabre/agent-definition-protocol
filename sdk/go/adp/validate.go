package adp

import "fmt"

func ValidateADP(_adp *ADP) error {
    // Allow both v0.1.0 and v0.2.0
    if _adp.ADPVersion != "0.1.0" && _adp.ADPVersion != "0.2.0" {
        return fmt.Errorf("adp_version must be 0.1.0 or 0.2.0, got %s", _adp.ADPVersion)
    }
    if len(_adp.Runtime.Execution) == 0 {
        return fmt.Errorf("runtime.execution must not be empty")
    }
    return nil
}
