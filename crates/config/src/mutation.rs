//! Electronic Propagation Delay & Register Mutation Pipeline
//!
//! Models physical circuit propagation latencies where register writes do not take
//! instantaneous effect, but travel down a provisional delay pipeline before committing
//! to active silicon latches.

use serde::{Deserialize, Serialize};

/// Register mutation propagation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationMode {
    /// Pipelined wave: subsequent writes to the same register coexist and commit in temporal FIFO order
    Pipeline,
    /// Overwrite pending: a subsequent write to the same register aborts any in-flight mutation and restarts the countdown
    OverwritePending,
}

/// Staged register write in the physical propagation latency pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelayedMutation {
    /// Chip-local queued destination (custom offset, interrupt input or CIA register)
    pub reg_offset: u16,
    /// Staged value to commit
    pub value: u16,
    /// Remaining clock cycles (CCK or E-Clock) before committing to active silicon state
    pub remaining_cck: u8,
    /// Mutation propagation mode
    pub mode: MutationMode,
}

/// Positive provisional delay for custom writes that previously bypassed the queue.
/// This setting establishes ordering, not measured silicon timing.
pub const PROVISIONAL_CUSTOM_WRITE_DELAY_CCK: u8 = 1;

/// Stages a write in issue order; zero delay and full capacity reject it.
/// Rejection never authorizes immediate application. The caller retains/retries it.
#[inline]
pub fn stage_mutation(
    buffer: &mut [Option<DelayedMutation>],
    reg_offset: u16,
    value: u16,
    delay_cck: u8,
    mode: MutationMode,
) -> bool {
    if delay_cck == 0 {
        return false;
    }
    // Replacing a pending value is a new issue event, after retained events.
    if mode == MutationMode::OverwritePending {
        if let Some(index) = buffer
            .iter()
            .position(|slot| slot.is_some_and(|m| m.reg_offset == reg_offset && m.mode == mode))
        {
            buffer[index..].rotate_left(1);
            if let Some(last) = buffer.last_mut() {
                *last = None;
            }
        }
    }
    if let Some(slot) = buffer.iter_mut().find(|slot| slot.is_none()) {
        *slot = Some(DelayedMutation {
            reg_offset,
            value,
            remaining_cck: delay_cck,
            mode,
        });
        true
    } else {
        false
    }
}

/// Advances one clock and commits every due entry in retained issue order.
/// Compaction prevents reuse of an earlier empty slot from reversing FIFO order.
#[inline]
pub fn tick_mutations<F>(buffer: &mut [Option<DelayedMutation>], mut commit_fn: F)
where
    F: FnMut(u16, u16),
{
    let mut retained = 0;
    for index in 0..buffer.len() {
        if let Some(mut item) = buffer[index].take() {
            item.remaining_cck = item.remaining_cck.saturating_sub(1);
            if item.remaining_cck == 0 {
                commit_fn(item.reg_offset, item.value);
            } else {
                buffer[retained] = Some(item);
                retained += 1;
            }
        }
    }
}
