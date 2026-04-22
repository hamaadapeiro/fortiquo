/// Gas configuration for EVM execution.
#[derive(Clone, Debug)]
pub struct GasConfig {
    /// Cost of STOP opcode
    pub gas_stop: u64,
    /// Cost of ADD opcode
    pub gas_add: u64,
    /// Cost of MUL opcode
    pub gas_mul: u64,
    /// Cost of SUB opcode
    pub gas_sub: u64,
    /// Cost of DIV opcode
    pub gas_div: u64,
    /// Cost of memory access
    pub gas_memory_byte: u64,
    /// Cost of SSTORE (storage write)
    pub gas_sstore: u64,
    /// Cost of SLOAD (storage read)
    pub gas_sload: u64,
    /// Cost of calling a contract
    pub gas_call: u64,
    /// Cost of CREATE opcode
    pub gas_create: u64,
    /// Cost of CREATE2 opcode
    pub gas_create2: u64,
    /// Cost of SELFDESTRUCT opcode
    pub gas_selfdestruct: u64,
    /// Cost of LOG opcode
    pub gas_log_base: u64,
    /// Cost per topic in LOG
    pub gas_log_topic: u64,
    /// Cost per byte of data in LOG
    pub gas_log_data_byte: u64,
}

impl Default for GasConfig {
    fn default() -> Self {
        GasConfig {
            gas_stop: 0,
            gas_add: 3,
            gas_mul: 5,
            gas_sub: 3,
            gas_div: 5,
            gas_memory_byte: 3,
            gas_sstore: 20_000,
            gas_sload: 200,
            gas_call: 700,
            gas_create: 32_000,
            gas_create2: 32_000,
            gas_selfdestruct: 5_000,
            gas_log_base: 375,
            gas_log_topic: 375,
            gas_log_data_byte: 8,
        }
    }
}

impl GasConfig {
    /// Get the cost of a memory expansion (approximation).
    pub fn gas_memory_expansion(new_size: u64, old_size: u64) -> u64 {
        if new_size <= old_size {
            return 0;
        }
        let expansion = new_size - old_size;
        // Simplified memory expansion gas calculation
        (expansion * 3) + ((expansion * expansion) / 512)
    }

    /// Validate gas against a limit.
    pub fn validate_gas(&self, used: u64, limit: u64) -> Result<(), String> {
        if used > limit {
            return Err(format!(
                "Gas limit exceeded: used {} > limit {}",
                used, limit
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_gas_config() {
        let config = GasConfig::default();
        assert_eq!(config.gas_add, 3);
        assert_eq!(config.gas_mul, 5);
        assert_eq!(config.gas_sstore, 20_000);
    }

    #[test]
    fn test_memory_expansion_zero() {
        assert_eq!(GasConfig::gas_memory_expansion(100, 100), 0);
        assert_eq!(GasConfig::gas_memory_expansion(50, 100), 0);
    }

    #[test]
    fn test_memory_expansion_cost() {
        let cost = GasConfig::gas_memory_expansion(100, 50);
        assert!(cost > 0, "Memory expansion should have cost");
    }

    #[test]
    fn test_validate_gas_success() {
        let config = GasConfig::default();
        assert!(config.validate_gas(50_000, 100_000).is_ok());
        assert!(config.validate_gas(100_000, 100_000).is_ok());
    }

    #[test]
    fn test_validate_gas_failure() {
        let config = GasConfig::default();
        assert!(config.validate_gas(150_000, 100_000).is_err());
    }
}
