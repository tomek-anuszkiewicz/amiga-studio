//! Breakpoint, Watchpoint, and Conditional Evaluation Management
//!
//! Supports PC execution breakpoints, memory range watchpoints (Read/Write/Any),
//! and register-based conditional expressions for targeted hardware debugging.

use cpu::CpuState;

/// Watchpoint memory access type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchAccess {
    Read,
    Write,
    Any,
}

/// Target register for conditional evaluation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionRegister {
    D(u8), // Data registers D0 - D7
    A(u8), // Address registers A0 - A7
    PC,    // Program Counter
    SR,    // Status Register
    CCR,   // Condition Code Register (lower 5 bits of SR)
}

impl ConditionRegister {
    /// Extracts the register's 32-bit value from the CPU state
    #[inline]
    pub fn get_value(&self, state: &CpuState) -> u32 {
        match *self {
            ConditionRegister::D(idx) => state.d_long(idx as usize),
            ConditionRegister::A(idx) => state.a_long(idx as usize),
            ConditionRegister::PC => state.instruction_pc & 0x00FF_FFFF,
            ConditionRegister::SR => state.sr() as u32,
            ConditionRegister::CCR => state.ccr() as u32,
        }
    }
}

/// Comparison operator for conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionOp {
    Eq,     // ==
    Ne,     // !=
    Lt,     // <
    Gt,     // >
    Lte,    // <=
    Gte,    // >=
    MaskEq, // (reg & mask) == value
}

/// A simple conditional filter attached to a breakpoint
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakpointCondition {
    pub register: ConditionRegister,
    pub op: ConditionOp,
    pub value: u32,
    pub mask: Option<u32>,
}

impl BreakpointCondition {
    /// Creates a standard equality condition: `reg == val`
    pub fn eq(reg: ConditionRegister, value: u32) -> Self {
        Self {
            register: reg,
            op: ConditionOp::Eq,
            value,
            mask: None,
        }
    }

    /// Evaluates the condition against the current CPU state
    pub fn evaluate(&self, state: &CpuState) -> bool {
        let reg_val = self.register.get_value(state);
        match self.op {
            ConditionOp::Eq => reg_val == self.value,
            ConditionOp::Ne => reg_val != self.value,
            ConditionOp::Lt => reg_val < self.value,
            ConditionOp::Gt => reg_val > self.value,
            ConditionOp::Lte => reg_val <= self.value,
            ConditionOp::Gte => reg_val >= self.value,
            ConditionOp::MaskEq => {
                let m = self.mask.unwrap_or(0xFFFF_FFFF);
                (reg_val & m) == (self.value & m)
            }
        }
    }

    /// Formats the condition as human-readable text
    pub fn format(&self) -> String {
        let reg_str = match self.register {
            ConditionRegister::D(i) => format!("D{}", i),
            ConditionRegister::A(i) => format!("A{}", i),
            ConditionRegister::PC => "PC".to_string(),
            ConditionRegister::SR => "SR".to_string(),
            ConditionRegister::CCR => "CCR".to_string(),
        };
        let op_str = match self.op {
            ConditionOp::Eq => "==",
            ConditionOp::Ne => "!=",
            ConditionOp::Lt => "<",
            ConditionOp::Gt => ">",
            ConditionOp::Lte => "<=",
            ConditionOp::Gte => ">=",
            ConditionOp::MaskEq => "&",
        };
        if self.op == ConditionOp::MaskEq {
            let m = self.mask.unwrap_or(0xFFFF_FFFF);
            format!("({} & ${:X}) == ${:X}", reg_str, m, self.value)
        } else {
            format!("{} {} ${:X}", reg_str, op_str, self.value)
        }
    }
}

/// A PC execution breakpoint with optional condition and enable switch
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcBreakpoint {
    pub addr: u32,
    pub enabled: bool,
    pub condition: Option<BreakpointCondition>,
}

/// A memory address range watchpoint
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryWatchpoint {
    pub start: u32,
    pub end: u32,
    pub access: WatchAccess,
    pub enabled: bool,
}

/// Breakpoint and Watchpoint Manager
#[derive(Debug, Clone, Default)]
pub struct BreakpointManager {
    /// Detailed PC execution breakpoints
    pub pc_breakpoints: Vec<PcBreakpoint>,
    /// Address watchpoints (range start..=end, access type)
    pub watchpoints: Vec<MemoryWatchpoint>,
}

impl BreakpointManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a PC execution breakpoint
    pub fn add_pc_breakpoint(&mut self, addr: u32) {
        let addr = addr & 0x00FF_FFFF;
        if let Some(bp) = self.pc_breakpoints.iter_mut().find(|b| b.addr == addr) {
            bp.enabled = true;
        } else {
            self.pc_breakpoints.push(PcBreakpoint {
                addr,
                enabled: true,
                condition: None,
            });
        }
    }

    /// Adds a PC execution breakpoint with a conditional expression
    pub fn add_conditional_breakpoint(&mut self, addr: u32, condition: BreakpointCondition) {
        let addr = addr & 0x00FF_FFFF;
        if let Some(bp) = self.pc_breakpoints.iter_mut().find(|b| b.addr == addr) {
            bp.enabled = true;
            bp.condition = Some(condition);
        } else {
            self.pc_breakpoints.push(PcBreakpoint {
                addr,
                enabled: true,
                condition: Some(condition),
            });
        }
    }

    /// Removes a PC execution breakpoint
    pub fn remove_pc_breakpoint(&mut self, addr: u32) {
        let addr = addr & 0x00FF_FFFF;
        self.pc_breakpoints.retain(|b| b.addr != addr);
    }

    /// Toggles a PC execution breakpoint on/off
    pub fn toggle_pc_breakpoint(&mut self, addr: u32) {
        let addr = addr & 0x00FF_FFFF;
        if let Some(pos) = self.pc_breakpoints.iter().position(|b| b.addr == addr) {
            self.pc_breakpoints.remove(pos);
        } else {
            self.add_pc_breakpoint(addr);
        }
    }

    /// Checks if a PC breakpoint exists and is active for the specified address
    #[inline]
    pub fn has_pc_breakpoint(&self, addr: u32) -> bool {
        let addr = addr & 0x00FF_FFFF;
        self.pc_breakpoints
            .iter()
            .any(|b| b.addr == addr && b.enabled)
    }

    /// Checks if a PC breakpoint is hit at the given address, evaluating any conditions against `state`
    #[inline]
    pub fn check_pc_with_state(&self, pc: u32, state: &CpuState) -> bool {
        let pc = pc & 0x00FF_FFFF;
        for bp in &self.pc_breakpoints {
            if bp.enabled && bp.addr == pc {
                if let Some(ref cond) = bp.condition {
                    if cond.evaluate(state) {
                        return true;
                    }
                } else {
                    return true;
                }
            }
        }
        false
    }

    /// Checks if an unconditional PC breakpoint is hit at the given address
    #[inline]
    pub fn check_pc(&self, pc: u32) -> bool {
        let pc = pc & 0x00FF_FFFF;
        self.pc_breakpoints
            .iter()
            .any(|b| b.addr == pc && b.enabled && b.condition.is_none())
    }

    /// Adds a memory watchpoint
    pub fn add_watchpoint(&mut self, start: u32, end: u32, access: WatchAccess) {
        let start = start & 0x00FF_FFFF;
        let end = end & 0x00FF_FFFF;
        self.watchpoints.push(MemoryWatchpoint {
            start,
            end,
            access,
            enabled: true,
        });
    }

    /// Removes a memory watchpoint by index
    pub fn remove_watchpoint(&mut self, index: usize) {
        if index < self.watchpoints.len() {
            self.watchpoints.remove(index);
        }
    }

    /// Checks if a memory access triggers a watchpoint
    pub fn check_watchpoint(&self, addr: u32, is_write: bool) -> bool {
        let addr = addr & 0x00FF_FFFF;
        for wp in &self.watchpoints {
            if wp.enabled && addr >= wp.start && addr <= wp.end {
                match wp.access {
                    WatchAccess::Any => return true,
                    WatchAccess::Read if !is_write => return true,
                    WatchAccess::Write if is_write => return true,
                    _ => {}
                }
            }
        }
        false
    }

    /// Finds any active watchpoint covering the specified address
    pub fn find_watchpoint_at(&self, addr: u32) -> Option<&MemoryWatchpoint> {
        let addr = addr & 0x00FF_FFFF;
        self.watchpoints
            .iter()
            .find(|wp| wp.enabled && addr >= wp.start && addr <= wp.end)
    }

    /// Toggles a 1-byte watchpoint for the given address and access mode
    pub fn toggle_byte_watchpoint(&mut self, addr: u32, access: WatchAccess) {
        let addr = addr & 0x00FF_FFFF;
        if let Some(pos) = self
            .watchpoints
            .iter()
            .position(|wp| wp.start == addr && wp.end == addr)
        {
            self.watchpoints.remove(pos);
        } else {
            self.add_watchpoint(addr, addr, access);
        }
    }

    /// Removes all watchpoints that intersect or cover the given address
    pub fn remove_watchpoints_at(&mut self, addr: u32) {
        let addr = addr & 0x00FF_FFFF;
        self.watchpoints
            .retain(|wp| addr < wp.start || addr > wp.end);
    }

    /// Total count of all configured breakpoints and watchpoints
    #[inline]
    pub fn total_count(&self) -> usize {
        self.pc_breakpoints.len() + self.watchpoints.len()
    }

    /// Clears all breakpoints and watchpoints
    pub fn clear(&mut self) {
        self.pc_breakpoints.clear();
        self.watchpoints.clear();
    }
}
