use crate::adp::Adp;

pub fn validate_adp(adp: &Adp) -> Result<(), Box<dyn std::error::Error>> {
    // Allow both v0.1.0 and v0.2.0
    if adp.adp_version != "0.1.0" && adp.adp_version != "0.2.0" {
        return Err(format!("adp_version must be 0.1.0 or 0.2.0, got {}", adp.adp_version).into());
    }
    if adp.runtime.execution.is_empty() {
        return Err("runtime.execution must not be empty".into());
    }
    Ok(())
}
