//! Diagnostic failure logging and diff formatting for M68000 SingleStepTests

use serde::{Deserialize, Serialize};

/// Detailed condition code flag breakdown
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CcrBreakdown {
    pub t: bool,
    pub s: bool,
    pub i: u8,
    pub x: bool,
    pub n: bool,
    pub z: bool,
    pub v: bool,
    pub c: bool,
}

impl From<u16> for CcrBreakdown {
    fn from(sr: u16) -> Self {
        Self {
            t: (sr & 0x8000) != 0,
            s: (sr & 0x2000) != 0,
            i: ((sr >> 8) & 0x07) as u8,
            x: (sr & 0x0010) != 0,
            n: (sr & 0x0008) != 0,
            z: (sr & 0x0004) != 0,
            v: (sr & 0x0002) != 0,
            c: (sr & 0x0001) != 0,
        }
    }
}

impl CcrBreakdown {
    pub fn format_flags(&self) -> String {
        format!(
            "[T:{} S:{} I:{} X:{} N:{} Z:{} V:{} C:{}]",
            if self.t { 1 } else { 0 },
            if self.s { 1 } else { 0 },
            self.i,
            if self.x { 1 } else { 0 },
            if self.n { 1 } else { 0 },
            if self.z { 1 } else { 0 },
            if self.v { 1 } else { 0 },
            if self.c { 1 } else { 0 },
        )
    }
}

/// Formats a human-readable comparison between actual and expected Status Register (SR)
pub fn format_ccr_diff(actual_sr: u16, expected_sr: u16) -> (String, Vec<String>) {
    let actual = CcrBreakdown::from(actual_sr);
    let expected = CcrBreakdown::from(expected_sr);

    let mut diff_descriptions = Vec::new();

    if actual.t != expected.t {
        diff_descriptions.push(format!(
            "T flag: expected {}, got {}",
            expected.t as u8, actual.t as u8
        ));
    }
    if actual.s != expected.s {
        diff_descriptions.push(format!(
            "S flag: expected {}, got {}",
            expected.s as u8, actual.s as u8
        ));
    }
    if actual.i != expected.i {
        diff_descriptions.push(format!(
            "IPL mask: expected {}, got {}",
            expected.i, actual.i
        ));
    }
    if actual.x != expected.x {
        diff_descriptions.push(format!(
            "X flag: expected {}, got {} ({})",
            expected.x as u8,
            actual.x as u8,
            if actual.x {
                "unexpectedly SET"
            } else {
                "unexpectedly CLEARED"
            }
        ));
    }
    if actual.n != expected.n {
        diff_descriptions.push(format!(
            "N flag: expected {}, got {} ({})",
            expected.n as u8,
            actual.n as u8,
            if actual.n {
                "unexpectedly SET"
            } else {
                "unexpectedly CLEARED"
            }
        ));
    }
    if actual.z != expected.z {
        diff_descriptions.push(format!(
            "Z flag: expected {}, got {} ({})",
            expected.z as u8,
            actual.z as u8,
            if actual.z {
                "unexpectedly SET"
            } else {
                "unexpectedly CLEARED"
            }
        ));
    }
    if actual.v != expected.v {
        diff_descriptions.push(format!(
            "V flag: expected {}, got {} ({})",
            expected.v as u8,
            actual.v as u8,
            if actual.v {
                "unexpectedly SET"
            } else {
                "unexpectedly CLEARED"
            }
        ));
    }
    if actual.c != expected.c {
        diff_descriptions.push(format!(
            "C flag: expected {}, got {} ({})",
            expected.c as u8,
            actual.c as u8,
            if actual.c {
                "unexpectedly SET"
            } else {
                "unexpectedly CLEARED"
            }
        ));
    }

    let summary = format!(
        "Expected SR: 0x{:04X} {}\nActual SR:   0x{:04X} {}",
        expected_sr,
        expected.format_flags(),
        actual_sr,
        actual.format_flags()
    );

    (summary, diff_descriptions)
}

/// Specific difference between CPU actual state and expected test vector state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateDiff {
    DataRegister {
        reg: usize,
        actual: u32,
        expected: u32,
    },
    AddressRegister {
        reg: usize,
        actual: u32,
        expected: u32,
    },
    UserStackPointer {
        actual: u32,
        expected: u32,
    },
    SupervisorStackPointer {
        actual: u32,
        expected: u32,
    },
    StatusRegister {
        actual: u16,
        expected: u16,
        details: String,
        diverging_flags: Vec<String>,
    },
    ProgramCounter {
        actual: u32,
        expected: u32,
    },
    RamByte {
        address: u32,
        actual: u8,
        expected: u8,
    },
    CycleLength {
        actual: u32,
        expected: u32,
    },
    TransactionCountMismatch {
        actual: usize,
        expected: usize,
    },
    TransactionMismatch {
        details: String,
    },
}

/// Comprehensive failure diagnostic for a single test case
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestFailure {
    pub test_name: String,
    pub file_path: String,
    pub test_index: usize,
    /// Clock cycles specified in test vector
    pub expected_clocks: u32,
    /// Estimated CCK cycles (1 CCK = 2 clocks, or 4 clocks / bus cycle)
    pub expected_cck: u32,
    pub diffs: Vec<StateDiff>,
}

impl TestFailure {
    pub fn new(
        test_name: impl Into<String>,
        file_path: impl Into<String>,
        test_index: usize,
        expected_clocks: u32,
    ) -> Self {
        Self {
            test_name: test_name.into(),
            file_path: file_path.into(),
            test_index,
            expected_clocks,
            expected_cck: expected_clocks / 2,
            diffs: Vec::new(),
        }
    }

    /// Formats an actionable, human-readable failure report
    pub fn format_diagnostic(&self) -> String {
        let mut out = String::with_capacity(1024);
        out.push_str(
            "================================================================================\n",
        );
        out.push_str(&format!("❌ TEST FAILURE: \"{}\"\n", self.test_name));
        out.push_str(&format!(
            "   Location: {} [Test #{}]\n",
            self.file_path, self.test_index
        ));
        out.push_str(&format!(
            "   Cycle:    {} clock cycles (approx. {} CCK cycles)\n",
            self.expected_clocks, self.expected_cck
        ));
        out.push_str(
            "--------------------------------------------------------------------------------\n",
        );
        out.push_str("Differences detected:\n");

        for diff in &self.diffs {
            match diff {
                StateDiff::StatusRegister {
                    actual,
                    expected,
                    diverging_flags,
                    ..
                } => {
                    let actual_flags = CcrBreakdown::from(*actual).format_flags();
                    let expected_flags = CcrBreakdown::from(*expected).format_flags();
                    out.push_str("  • Status Register / CCR Mismatch:\n");
                    out.push_str(&format!(
                        "      Expected: 0x{:04X} {}\n",
                        expected, expected_flags
                    ));
                    out.push_str(&format!(
                        "      Actual:   0x{:04X} {}\n",
                        actual, actual_flags
                    ));
                    if !diverging_flags.is_empty() {
                        out.push_str(&format!("      Diff:     {}\n", diverging_flags.join(", ")));
                    }
                }
                StateDiff::DataRegister {
                    reg,
                    actual,
                    expected,
                } => {
                    out.push_str(&format!(
                        "  • Data Register D{}: Expected 0x{:08X}, got 0x{:08X} (diff: {})\n",
                        reg,
                        expected,
                        actual,
                        format_diff_offset(*actual, *expected)
                    ));
                }
                StateDiff::AddressRegister {
                    reg,
                    actual,
                    expected,
                } => {
                    out.push_str(&format!(
                        "  • Address Register A{}: Expected 0x{:08X}, got 0x{:08X} (diff: {})\n",
                        reg,
                        expected,
                        actual,
                        format_diff_offset(*actual, *expected)
                    ));
                }
                StateDiff::UserStackPointer { actual, expected } => {
                    out.push_str(&format!(
                        "  • User Stack Pointer (USP): Expected 0x{:08X}, got 0x{:08X}\n",
                        expected, actual
                    ));
                }
                StateDiff::SupervisorStackPointer { actual, expected } => {
                    out.push_str(&format!(
                        "  • Supervisor Stack Pointer (SSP): Expected 0x{:08X}, got 0x{:08X}\n",
                        expected, actual
                    ));
                }
                StateDiff::ProgramCounter { actual, expected } => {
                    out.push_str(&format!(
                        "  • Program Counter (PC): Expected 0x{:08X}, got 0x{:08X} (diff: {})\n",
                        expected,
                        actual,
                        format_diff_offset(*actual, *expected)
                    ));
                }
                StateDiff::RamByte {
                    address,
                    actual,
                    expected,
                } => {
                    out.push_str(&format!(
                        "  • RAM at $0x{:06X}: Expected 0x{:02X}, got 0x{:02X}\n",
                        address, expected, actual
                    ));
                }
                StateDiff::CycleLength { actual, expected } => {
                    out.push_str(&format!(
                        "  • Instruction Cycle Count: Expected {} clocks ({} CCKs), got {} clocks ({} CCKs)\n",
                        expected, expected / 2, actual, actual / 2
                    ));
                }
                StateDiff::TransactionCountMismatch { actual, expected } => {
                    out.push_str(&format!(
                        "  • Bus Transaction Count: Expected {} transactions, recorded {}\n",
                        expected, actual
                    ));
                }
                StateDiff::TransactionMismatch { details } => {
                    out.push_str(&format!("  • Bus Transaction Mismatch: {}\n", details));
                }
            }
        }
        out.push_str(
            "================================================================================\n",
        );
        out
    }
}

fn format_diff_offset(actual: u32, expected: u32) -> String {
    let diff = actual.wrapping_sub(expected) as i32;
    if diff > 0 {
        format!("+{}", diff)
    } else {
        format!("{}", diff)
    }
}

impl std::fmt::Display for TestFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format_diagnostic())
    }
}

impl std::error::Error for TestFailure {}
