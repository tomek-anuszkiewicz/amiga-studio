//! Electronic Propagation Delay & Register Mutation Pipeline
//!
//! Models physical circuit propagation latencies where register writes do not take
//! instantaneous effect, but travel down a calibrated delay pipeline before committing
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
    /// Register address offset (e.g. $000..$1FE for custom chips, or $0..$F for CIAs)
    pub reg_offset: u16,
    /// Staged value to commit
    pub value: u16,
    /// Remaining clock cycles (CCK or E-Clock) before committing to active silicon state
    pub remaining_cck: u8,
    /// Mutation propagation mode
    pub mode: MutationMode,
}

/// Stages a register write into an inline fixed-capacity mutation buffer.
///
/// Returns `true` if successfully queued into the delay pipeline, or `false` if
/// immediate commit is required (when `delay_cck == 0` or upon buffer overflow fallback).
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

    // In OverwritePending mode, check if this exact register already has an in-flight mutation
    if mode == MutationMode::OverwritePending {
        for slot in buffer.iter_mut() {
            if let Some(m) = slot {
                if m.reg_offset == reg_offset {
                    m.value = value;
                    m.remaining_cck = delay_cck;
                    return true;
                }
            }
        }
    }

    // Insert into the first available empty slot
    for slot in buffer.iter_mut() {
        if slot.is_none() {
            *slot = Some(DelayedMutation {
                reg_offset,
                value,
                remaining_cck: delay_cck,
                mode,
            });
            return true;
        }
    }

    // Overflow protection: buffer capacity exhausted.
    // Return false so caller falls back to immediate commit, preventing data loss and host panics.
    false
}

/// Advances the delay countdown of all in-flight mutations by 1 clock cycle.
///
/// For any mutations that reach 0, clears the slot and invokes `commit_fn(reg_offset, value)`.
#[inline]
pub fn tick_mutations<F>(buffer: &mut [Option<DelayedMutation>], mut commit_fn: F)
where
    F: FnMut(u16, u16),
{
    for slot in buffer.iter_mut() {
        if let Some(m) = slot {
            if m.remaining_cck > 0 {
                m.remaining_cck -= 1;
            }
            if m.remaining_cck == 0 {
                let reg = m.reg_offset;
                let val = m.value;
                *slot = None;
                commit_fn(reg, val);
            }
        }
    }
}
