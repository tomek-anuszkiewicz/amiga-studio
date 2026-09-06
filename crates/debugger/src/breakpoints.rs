//! Breakpoint and Watchpoint management


/// Watchpoint access type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchAccess {
    Read,
    Write,
    Any,
}

/// Breakpoint and Watchpoint Manager
#[derive(Debug, Clone, Default)]
pub struct BreakpointManager {
    /// PC execution breakpoints (target address -> enabled)
    pub pc_breakpoints: Vec<u32>,
    /// Address watchpoints (range start..=end, access type)
    pub watchpoints: Vec<(u32, u32, WatchAccess)>,
}

impl BreakpointManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a PC execution breakpoint
    pub fn add_pc_breakpoint(&mut self, addr: u32) {
        let addr = addr & 0x00FF_FFFF;
        if !self.pc_breakpoints.contains(&addr) {
            self.pc_breakpoints.push(addr);
        }
    }

    /// Removes a PC execution breakpoint
    pub fn remove_pc_breakpoint(&mut self, addr: u32) {
        let addr = addr & 0x00FF_FFFF;
        self.pc_breakpoints.retain(|&a| a != addr);
    }

    /// Checks if a PC breakpoint is hit
    #[inline]
    pub fn check_pc(&self, pc: u32) -> bool {
        let pc = pc & 0x00FF_FFFF;
        self.pc_breakpoints.contains(&pc)
    }

    /// Adds a memory watchpoint
    pub fn add_watchpoint(&mut self, start: u32, end: u32, access: WatchAccess) {
        let start = start & 0x00FF_FFFF;
        let end = end & 0x00FF_FFFF;
        self.watchpoints.push((start, end, access));
    }

    /// Checks if a memory access triggers a watchpoint
    pub fn check_watchpoint(&self, addr: u32, is_write: bool) -> bool {
        let addr = addr & 0x00FF_FFFF;
        for &(start, end, access) in &self.watchpoints {
            if addr >= start && addr <= end {
                match access {
                    WatchAccess::Any => return true,
                    WatchAccess::Read if !is_write => return true,
                    WatchAccess::Write if is_write => return true,
                    _ => {}
                }
            }
        }
        false
    }
}
