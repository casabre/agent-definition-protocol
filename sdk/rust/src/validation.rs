use crate::adp::Adp;

pub fn validate_adp(adp: &Adp) -> Result<(), Box<dyn std::error::Error>> {
    if adp.adp_version != "0.1.0" {
        return Err("adp_version must be 0.1.0".into());
    }
    if adp.runtime.execution.is_empty() {
        return Err("runtime.execution must not be empty".into());
    }
    Ok(())
}
