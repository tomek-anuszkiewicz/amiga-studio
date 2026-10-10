//! Serde schema definitions for SingleStepTests (Tom Harte physical silicon vectors)

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SingleStepTest {
    pub name: String,
    pub initial: CpuTestState,
    #[serde(rename = "final")]
    pub final_state: CpuTestState,
    pub transactions: Vec<serde_json::Value>,
    pub length: u32,
}

#[derive(Debug, Deserialize)]
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
