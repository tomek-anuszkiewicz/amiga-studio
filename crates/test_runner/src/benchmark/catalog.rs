//! M68000 Instruction Benchmark Catalog
//!
//! Exhaustive catalog of representative instruction variants, sizes, addressing modes,
//! expected Amiga CCK cycles, and category tags per Obsidian/Amiga/Design/CPU Instruction Benchmark Catalog.md.

use serde::{Deserialize, Serialize};

use super::catalog_data_a::CATALOG_DATA_A;
use super::catalog_data_b::CATALOG_DATA_B;

/// Addressing mode evaluated by the benchmark
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddressingMode {
    Implied,
    DataRegDirect,
    AddrRegDirect,
    AddrIndirect,
    PostIncrement,
    PreDecrement,
    Displacement,
    Index,
    AbsoluteShort,
    AbsoluteLong,
    PcDisplacement,
    PcIndex,
    Immediate,
}

impl AddressingMode {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Implied => "Implied",
            Self::DataRegDirect => "DataRegDirect",
            Self::AddrRegDirect => "AddrRegDirect",
            Self::AddrIndirect => "AddrIndirect",
            Self::PostIncrement => "PostIncrement",
            Self::PreDecrement => "PreDecrement",
            Self::Displacement => "Displacement",
            Self::Index => "Index",
            Self::AbsoluteShort => "AbsoluteShort",
            Self::AbsoluteLong => "AbsoluteLong",
            Self::PcDisplacement => "PcDisplacement",
            Self::PcIndex => "PcIndex",
            Self::Immediate => "Immediate",
        }
    }
}

/// Broad instruction category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstructionCategory {
    Baseline,
    DataMovement,
    Arithmetic,
    Logic,
    ShiftRotate,
    Comparison,
    Bcd,
    ControlFlow,
    System,
}

impl InstructionCategory {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Baseline => "Baseline",
            Self::DataMovement => "DataMovement",
            Self::Arithmetic => "Arithmetic",
            Self::Logic => "Logic",
            Self::ShiftRotate => "ShiftRotate",
            Self::Comparison => "Comparison",
            Self::Bcd => "Bcd",
            Self::ControlFlow => "ControlFlow",
            Self::System => "System",
        }
    }
}

/// Execution strategy required for state invariance across 700 unrolled executions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BenchmarkStrategy {
    BaselineNop,
    DataRegRotate,
    AddrReg,
    CyclicMemPostInc,
    CyclicMemPreDec,
    CyclicMemIndirect,
    DisplacementMem,
    IndexMem,
    AbsoluteMem,
    PcMem,
    ImmediateStream,
    MemToMemPostInc,
    StackPush,
    StackFramePair,
    ArithRegRotate,
    ArithMemPostInc,
    ArithMemIndirect,
    ArithMemPreDec,
    MultiplyRotate,
    DivideValid,
    DivideOverflow,
    DivideZeroTrap,
    LogicMaskAlternate,
    LogicIdempotent,
    BitCycling,
    TestAndSet,
    BalancedShiftPair,
    RotateUnroll,
    CompareIdempotent,
    ComparePostInc,
    CheckInBounds,
    BcdReg,
    BcdMem,
    BranchTaken,
    BranchUntaken,
    SubroutineCall,
    CascadingReturnRts,
    CascadingReturnRtr,
    LoopDecrement,
    SystemCcr,
    SystemSr,
    SystemUsp,
    CascadingReturnRte,
    SystemTrap,
    SystemTrapvUntaken,
}

/// Formal benchmark specification for an instruction variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BenchmarkSpec {
    pub id: &'static str,
    pub mnemonic: &'static str,
    pub size_suffix: &'static str,
    pub addressing_mode: AddressingMode,
    pub representative_syntax: &'static str,
    pub opcode_words: &'static [u16],
    pub amiga_cck: u32,
    pub category: InstructionCategory,
    pub category_tag: &'static str,
    pub strategy: BenchmarkStrategy,
}

/// Iterates over all registered benchmark specifications across catalog parts A and B
pub fn all_benchmark_specs() -> impl Iterator<Item = &'static BenchmarkSpec> {
    CATALOG_DATA_A.iter().chain(CATALOG_DATA_B.iter())
}

/// Total count of all benchmark specifications in catalog
pub fn total_benchmark_specs_count() -> usize {
    CATALOG_DATA_A.len() + CATALOG_DATA_B.len()
}

/// Looks up a benchmark specification by its unique ID
pub fn find_spec_by_id(id: &str) -> Option<&'static BenchmarkSpec> {
    all_benchmark_specs().find(|s| s.id == id)
}

/// Filters benchmark specifications matching the query string against ID, mnemonic, category, or tag
pub fn filter_specs(query: &str) -> Vec<&'static BenchmarkSpec> {
    if query.is_empty() || query == "*" {
        return all_benchmark_specs().collect();
    }
    let parts: Vec<&str> = query
        .split('|')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    all_benchmark_specs()
        .filter(|spec| {
            parts.iter().any(|&part| {
                spec.id.eq_ignore_ascii_case(part)
                    || spec.mnemonic.eq_ignore_ascii_case(part)
                    || spec.category_tag.contains(part)
                    || spec.category.name().eq_ignore_ascii_case(part)
                    || spec
                        .representative_syntax
                        .to_ascii_lowercase()
                        .contains(&part.to_ascii_lowercase())
            })
        })
        .collect()
}
