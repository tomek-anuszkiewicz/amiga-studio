//! Headless Temporal History and Time-Travel Debugging Engine
//!
//! Provides a high-capacity circular execution history ring buffer storing snapshots
//! of cycle-exact CPU states with zero heap allocations during recording.
//! Supports dynamic capacity scaling (>=1s PAL execution, ~250,000+ frames),
//! on/off recording toggles, multi-granularity navigation, and CCK cycle search.

use cpu::CpuState;

/// Single temporal snapshot entry in the history ring buffer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalFrame {
    pub cck: u64,
    pub pc: u32,
    pub ir: u16,
    pub state: CpuState,
}

/// Default buffer capacity: 25,000 instructions (~lean default, ~7.5 MB)
pub const DEFAULT_TEMPORAL_CAPACITY: usize = 25_000;

/// Standard Amiga PAL video frame duration in Color Clocks: 312 lines * 227 CCKs = 70,824 CCKs (~20 ms)
pub const PAL_FRAME_CCK: u64 = 70_824;

/// Circular history ring buffer for time-travel debugging
#[derive(Debug, Clone)]
pub struct TemporalHistory {
    capacity: usize,
    buffer: Vec<TemporalFrame>,
    write_pos: usize,
    total_recorded: usize,
    /// Whether recording is actively enabled
    recording_enabled: bool,
    /// Currently selected history offset (None = at live head)
    pub scrub_cursor: Option<usize>,
}

impl Default for TemporalHistory {
    fn default() -> Self {
        Self::new(DEFAULT_TEMPORAL_CAPACITY)
    }
}

impl TemporalHistory {
    /// Creates a new temporal history buffer with the given capacity
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.max(1);
        Self {
            capacity: cap,
            buffer: Vec::with_capacity(cap.min(10_000)), // lazily grow up to capacity
            write_pos: 0,
            total_recorded: 0,
            recording_enabled: true,
            scrub_cursor: None,
        }
    }

    /// Queries whether temporal recording is currently active
    #[inline]
    pub fn is_recording(&self) -> bool {
        self.recording_enabled
    }

    /// Toggles temporal recording state and returns the new state
    pub fn toggle_recording(&mut self) -> bool {
        self.recording_enabled = !self.recording_enabled;
        self.recording_enabled
    }

    /// Explicitly sets temporal recording active state
    #[inline]
    pub fn set_recording(&mut self, enabled: bool) {
        self.recording_enabled = enabled;
    }

    /// Dynamically resizes the capacity while preserving chronological history
    pub fn set_capacity(&mut self, new_cap: usize) {
        let new_cap = new_cap.max(1);
        if new_cap == self.capacity {
            return;
        }

        let current_count = self.buffer.len();
        if current_count == 0 {
            self.capacity = new_cap;
            return;
        }

        // Linearize current entries in chronological order
        let mut linearized = Vec::with_capacity(new_cap.min(current_count));
        let start_idx = if current_count > new_cap {
            current_count - new_cap
        } else {
            0
        };

        for i in start_idx..current_count {
            if let Some(frame) = self.get_chronological(i) {
                linearized.push(frame.clone());
            }
        }

        self.capacity = new_cap;
        self.write_pos = if linearized.len() >= new_cap {
            0
        } else {
            linearized.len()
        };
        self.buffer = linearized;
        self.scrub_cursor = None;
    }

    /// Records a new snapshot at the current live execution point if recording is enabled
    pub fn record(&mut self, cck: u64, pc: u32, ir: u16, state: CpuState) {
        if !self.recording_enabled {
            return;
        }

        let frame = TemporalFrame { cck, pc, ir, state };

        if self.buffer.len() < self.capacity {
            self.buffer.push(frame);
        } else {
            self.buffer[self.write_pos] = frame;
        }

        self.write_pos = (self.write_pos + 1) % self.capacity;
        self.total_recorded = self.total_recorded.saturating_add(1);
        self.scrub_cursor = None; // Reset to live head upon new execution
    }

    /// Number of frames currently held in the ring buffer
    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Maximum number of frames that can be held
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Queries whether the history buffer is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Returns a chronological frame by 0-indexed position (0 = oldest, len - 1 = most recent)
    pub fn get_chronological(&self, index: usize) -> Option<&TemporalFrame> {
        let count = self.buffer.len();
        if index >= count {
            return None;
        }

        if count < self.capacity {
            self.buffer.get(index)
        } else {
            let start = self.write_pos;
            let physical_idx = (start + index) % self.capacity;
            self.buffer.get(physical_idx)
        }
    }

    /// Total number of frames recorded since machine initialization
    #[inline]
    pub fn total_recorded(&self) -> usize {
        self.total_recorded
    }

    /// Clears all recorded history snapshots
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.write_pos = 0;
        self.total_recorded = 0;
        self.scrub_cursor = None;
    }

    /// Current chronological index being scrubbed, or live head (len - 1)
    #[inline]
    fn current_cursor_or_head(&self) -> usize {
        self.scrub_cursor
            .unwrap_or_else(|| self.len().saturating_sub(1))
    }

    /// Steps backward by `delta` instructions in history
    pub fn step_back_n(&mut self, delta: usize) -> Option<usize> {
        let count = self.len();
        if count == 0 {
            return None;
        }
        let cur = match self.scrub_cursor {
            Some(idx) => idx,
            None => count.saturating_sub(1),
        };
        let target = cur.saturating_sub(delta);
        self.scrub_cursor = Some(target);
        Some(target)
    }

    /// Steps forward by `delta` instructions toward live head
    pub fn step_forward_n(&mut self, delta: usize) -> Option<usize> {
        let count = self.len();
        if count == 0 {
            return None;
        }
        match self.scrub_cursor {
            Some(idx) => {
                let target = idx.saturating_add(delta);
                if target + 1 >= count {
                    self.scrub_cursor = None; // Reached live head
                    None
                } else {
                    self.scrub_cursor = Some(target);
                    Some(target)
                }
            }
            None => None, // Already at live head
        }
    }

    /// Jumps backward by ~1 video frame in CCKs (PAL default 70,824 CCKs)
    pub fn step_frame_back(&mut self, frame_cck: u64) -> Option<usize> {
        let count = self.len();
        if count == 0 {
            return None;
        }
        let cur_idx = self.current_cursor_or_head();
        let cur_frame = self.get_chronological(cur_idx)?;
        let target_cck = cur_frame.cck.saturating_sub(frame_cck);
        let target_idx = self.find_closest_cck(target_cck)?;
        self.scrub_cursor = Some(target_idx);
        Some(target_idx)
    }

    /// Jumps forward by ~1 video frame in CCKs
    pub fn step_frame_forward(&mut self, frame_cck: u64) -> Option<usize> {
        let count = self.len();
        if count == 0 {
            return None;
        }
        let cur_idx = self.current_cursor_or_head();
        let cur_frame = self.get_chronological(cur_idx)?;
        let target_cck = cur_frame.cck.saturating_add(frame_cck);
        let target_idx = self.find_closest_cck(target_cck)?;
        if target_idx + 1 >= count {
            self.scrub_cursor = None; // Reached live head
            None
        } else {
            self.scrub_cursor = Some(target_idx);
            Some(target_idx)
        }
    }

    /// Binary searches the chronological timeline to find the frame index closest to `target_cck`
    pub fn find_closest_cck(&self, target_cck: u64) -> Option<usize> {
        let count = self.len();
        if count == 0 {
            return None;
        }

        let first = self.get_chronological(0)?;
        if target_cck <= first.cck {
            return Some(0);
        }

        let last = self.get_chronological(count - 1)?;
        if target_cck >= last.cck {
            return Some(count - 1);
        }

        let mut low = 0;
        let mut high = count - 1;

        while low <= high {
            let mid = low + (high - low) / 2;
            let mid_cck = self.get_chronological(mid)?.cck;

            if mid_cck == target_cck {
                return Some(mid);
            } else if mid_cck < target_cck {
                low = mid + 1;
            } else {
                if mid == 0 {
                    return Some(0);
                }
                high = mid - 1;
            }
        }

        // Compare low and high to find whichever is closer
        let dist_low = if low < count {
            let cck = self.get_chronological(low)?.cck;
            cck.abs_diff(target_cck)
        } else {
            u64::MAX
        };

        let dist_high = if high < count {
            let cck = self.get_chronological(high)?.cck;
            cck.abs_diff(target_cck)
        } else {
            u64::MAX
        };

        if dist_low <= dist_high && low < count {
            Some(low)
        } else {
            Some(high)
        }
    }

    /// Sets the scrub cursor to a specific chronological index and returns the target frame
    pub fn scrub_to_index(&mut self, index: usize) -> Option<&TemporalFrame> {
        let count = self.len();
        if index < count {
            self.scrub_cursor = Some(index);
            self.get_chronological(index)
        } else {
            None
        }
    }

    /// Finds chronological indexes and CCK timestamps for historical passes of `target_pc`
    pub fn find_matches_by_pc(&self, target_pc: u32, max_matches: usize) -> Vec<(usize, u64)> {
        let count = self.len();
        let mut matches = Vec::new();
        for idx in 0..count {
            if let Some(frame) = self.get_chronological(idx) {
                if frame.pc == target_pc {
                    matches.push((idx, frame.cck));
                    if matches.len() >= max_matches {
                        break;
                    }
                }
            }
        }
        matches
    }
}
