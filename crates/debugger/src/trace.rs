//! Fixed-size 1024-entry execution trace ring buffer

use cpu::CpuState;
use serde::{Deserialize, Serialize};

pub const TRACE_BUFFER_SIZE: usize = 1024;

/// A single snapshot in the CPU execution trace history
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEntry {
    pub cck: u64,
    pub pc: u32,
    pub opcode: u16,
    pub disassembly: String,
    pub registers: CpuState,
}

/// Fixed-size ring buffer for post-mortem crash inspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceRingBuffer {
    buffer: Vec<Option<TraceEntry>>,
    head: usize,
    count: usize,
}

impl Default for TraceRingBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl TraceRingBuffer {
    pub fn new() -> Self {
        Self {
            buffer: vec![None; TRACE_BUFFER_SIZE],
            head: 0,
            count: 0,
        }
    }

    /// Appends a new execution trace entry
    pub fn record(
        &mut self,
        cck: u64,
        pc: u32,
        opcode: u16,
        disassembly: String,
        registers: CpuState,
    ) {
        self.buffer[self.head] = Some(TraceEntry {
            cck,
            pc,
            opcode,
            disassembly,
            registers,
        });
        self.head = (self.head + 1) % TRACE_BUFFER_SIZE;
        if self.count < TRACE_BUFFER_SIZE {
            self.count += 1;
        }
    }

    /// Returns the recorded entries in chronological order
    pub fn entries(&self) -> Vec<TraceEntry> {
        let mut res = Vec::with_capacity(self.count);
        if self.count == 0 {
            return res;
        }

        let start = if self.count < TRACE_BUFFER_SIZE {
            0
        } else {
            self.head
        };

        for i in 0..self.count {
            let idx = (start + i) % TRACE_BUFFER_SIZE;
            if let Some(entry) = &self.buffer[idx] {
                res.push(entry.clone());
            }
        }

        res
    }

    /// Returns a reference to an entry in chronological order without allocation
    #[inline]
    pub fn get(&self, chronological_idx: usize) -> Option<&TraceEntry> {
        if chronological_idx >= self.count {
            return None;
        }
        let start = if self.count < TRACE_BUFFER_SIZE {
            0
        } else {
            self.head
        };
        let idx = (start + chronological_idx) % TRACE_BUFFER_SIZE;
        self.buffer[idx].as_ref()
    }

    /// Clears the ring buffer
    pub fn clear(&mut self) {
        self.buffer.fill(None);
        self.head = 0;
        self.count = 0;
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}
