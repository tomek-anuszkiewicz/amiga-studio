//! Serde schema definitions for SingleStepTests (Tom Harte physical silicon vectors)

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct SingleStepTest {
    pub name: String,
    pub initial: CpuTestState,
    #[serde(rename = "final")]
    pub final_state: CpuTestState,
    pub transactions: Vec<serde_json::Value>,
    pub length: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CpuTestState {
    pub d0: u32,
    pub d1: u32,
    pub d2: u32,
    pub d3: u32,
    pub d4: u32,
    pub d5: u32,
    pub d6: u32,
    pub d7: u32,
    pub a0: u32,
    pub a1: u32,
    pub a2: u32,
    pub a3: u32,
    pub a4: u32,
    pub a5: u32,
    pub a6: u32,
    pub usp: u32,
    pub ssp: u32,
    pub sr: u16,
    pub pc: u32,
    pub prefetch: [u32; 2],
    pub ram: Vec<[u32; 2]>, // [address, byte]
}

impl SingleStepTest {
    /// Validate widths that the JSON container types cannot express themselves.
    pub fn validate(&self) -> Result<(), String> {
        for (name, state) in [("initial", &self.initial), ("final", &self.final_state)] {
            if state.prefetch.iter().any(|&word| word > u16::MAX as u32) {
                return Err(format!(
                    "{name}.prefetch contains a value outside the word range"
                ));
            }
            for (index, &[address, byte]) in state.ram.iter().enumerate() {
                if address > 0xFFFFFF || byte > u8::MAX as u32 {
                    return Err(format!(
                        "{name}.ram[{index}] exceeds the 24-bit address or byte range"
                    ));
                }
            }
        }
        Ok(())
    }
}
