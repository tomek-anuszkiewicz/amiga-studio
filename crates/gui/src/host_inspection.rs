//! Coherent, owned inspector data transferred only at stopped machine boundaries.

use cpu::CpuState;
use debugger::{Debugger, DebuggerSession};
use physical_memory::PhysicalMemory;

#[derive(Debug, Clone, Copy)]
pub struct HistoryPoint {
    pub cck: u64,
    pub pc: u32,
}

/// Timeline navigation needs positions and PCs; CPU snapshots stay with the owner.
#[derive(Debug)]
pub struct HistoryInspection {
    points: Vec<HistoryPoint>,
    pub scrub_cursor: Option<usize>,
    capacity: usize,
    recording: bool,
}

impl HistoryInspection {
    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn is_recording(&self) -> bool {
        self.recording
    }

    pub fn get_chronological(&self, index: usize) -> Option<&HistoryPoint> {
        self.points.get(index)
    }

    pub fn find_matches_by_pc(&self, pc: u32, limit: usize) -> Vec<(usize, u64)> {
        self.points
            .iter()
            .enumerate()
            .filter(|(_, point)| point.pc == pc)
            .take(limit)
            .map(|(index, point)| (index, point.cck))
            .collect()
    }
}

#[derive(Debug)]
pub struct HostInspection {
    pub cpu: CpuState,
    pub memory: PhysicalMemory,
    pub debugger: Debugger,
    pub temporal: HistoryInspection,
    pub cck: u64,
    pub instructions_executed: u64,
    pub previous_cpu: Option<CpuState>,
    pub previous_bytes: [u8; debugger::session::MEMORY_DIFF_BYTES],
    pub previous_base: u32,
}

impl HostInspection {
    pub(crate) fn capture(session: &DebuggerSession) -> Self {
        let history = &session.temporal;
        Self {
            cpu: session.machine.cpu.state.clone(),
            memory: session.machine.physical_memory.clone(),
            debugger: session.debugger.clone(),
            temporal: HistoryInspection {
                points: (0..history.len())
                    .filter_map(|index| history.get_chronological(index))
                    .map(|frame| HistoryPoint {
                        cck: frame.cck,
                        pc: frame.pc,
                    })
                    .collect(),
                scrub_cursor: history.scrub_cursor,
                capacity: history.capacity(),
                recording: history.is_recording(),
            },
            cck: session.machine.cck,
            instructions_executed: session.instructions_executed,
            previous_cpu: session.prev_cpu_state.clone(),
            previous_bytes: session.prev_hex_bytes,
            previous_base: session.prev_hex_base,
        }
    }
}
