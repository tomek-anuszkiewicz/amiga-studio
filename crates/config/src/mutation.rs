//! Electronic Propagation Delay & Register Mutation Pipeline
//!
//! Retains accepted register data until its modeled capture boundary.
//! Custom chips preserve operations in a pipeline; CIA retains the last pending
//! value per selector until its next falling E edge.

use core::fmt;
use serde::de::{Error, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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

/// Fallback delay for custom data writes whose timing remains unresolved.
/// Concrete reference timings remain separate; address strobes act immediately.
pub const PROVISIONAL_CUSTOM_WRITE_DELAY_CCK: u8 = 1;

/// Shared Agnus/Paula DMACON capture delay retained from the reference model.
pub const DMACON_WRITE_DELAY_CCK: u8 = 2;

/// Fixed storage with a private occupied prefix in retained issue order.
/// Only pending entries are logical state. The due buffer is reusable scratch:
/// it survives staging, is replaced by the next tick and is discarded on reset,
/// clone and deserialization. Serialization never stores unused entries/counts.
#[derive(Debug)]
pub struct MutationQueue<const N: usize> {
    entries: [DelayedMutation; N],
    len: usize,
    due: [(u16, u16); N],
    due_len: usize,
}

impl<const N: usize> Default for MutationQueue<N> {
    fn default() -> Self {
        Self {
            entries: [DelayedMutation {
                reg_offset: 0,
                value: 0,
                remaining_cck: 0,
                mode: MutationMode::Pipeline,
            }; N],
            len: 0,
            due: [(0, 0); N],
            due_len: 0,
        }
    }
}

impl<const N: usize> Clone for MutationQueue<N> {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries,
            len: self.len,
            ..Self::default()
        }
    }
}

impl<const N: usize> PartialEq for MutationQueue<N> {
    fn eq(&self, other: &Self) -> bool {
        self.pending() == other.pending()
    }
}

impl<const N: usize> Eq for MutationQueue<N> {}

impl<const N: usize> MutationQueue<N> {
    /// Pending writes; callers cannot invalidate the occupied prefix.
    #[inline]
    pub fn pending(&self) -> &[DelayedMutation] {
        &self.entries[..self.len]
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.len == N
    }

    /// Cancels pending writes and the previous tick's observation.
    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
        self.due_len = 0;
    }

    /// Stages a write in issue order; zero delay and full capacity reject it.
    /// An overwrite is a new issue event after all retained entries, even at capacity.
    pub fn stage(
        &mut self,
        reg_offset: u16,
        value: u16,
        delay_cck: u8,
        mode: MutationMode,
    ) -> bool {
        if delay_cck == 0 {
            return false;
        }
        if mode == MutationMode::OverwritePending {
            if let Some(index) = self
                .pending()
                .iter()
                .position(|m| m.reg_offset == reg_offset && m.mode == mode)
            {
                self.entries.copy_within(index + 1..self.len, index);
                self.len -= 1;
            }
        }
        if self.is_full() {
            return false;
        }
        self.entries[self.len] = DelayedMutation {
            reg_offset,
            value,
            remaining_cck: delay_cck,
            mode,
        };
        self.len += 1;
        true
    }

    /// Advances the occupied prefix once and freezes this clock's complete due set.
    /// Application happens afterward, so newly staged writes cannot join that set.
    pub fn tick(&mut self) {
        self.due_len = 0;
        let mut retained = 0;
        for index in 0..self.len {
            let mut item = self.entries[index];
            item.remaining_cck = item.remaining_cck.saturating_sub(1);
            if item.remaining_cck == 0 {
                self.due[self.due_len] = (item.reg_offset, item.value);
                self.due_len += 1;
            } else {
                self.entries[retained] = item;
                retained += 1;
            }
        }
        self.len = retained;
    }

    /// Every write matured by the last tick, in retained issue order.
    /// The borrow prevents mutation while observing the slice; staging afterward
    /// leaves it intact until the next tick or clear.
    #[inline]
    pub fn due(&self) -> &[(u16, u16)] {
        &self.due[..self.due_len]
    }
}

impl<const N: usize> Serialize for MutationQueue<N> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.pending().serialize(serializer)
    }
}

impl<'de, const N: usize> Deserialize<'de> for MutationQueue<N> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct QueueVisitor<const N: usize>;

        impl<'de, const N: usize> Visitor<'de> for QueueVisitor<N> {
            type Value = MutationQueue<N>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "at most {N} pending writes with nonzero delays")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut queue = MutationQueue::default();
                while let Some(item) = seq.next_element::<DelayedMutation>()? {
                    if queue.is_full() || item.remaining_cck == 0 {
                        return Err(A::Error::custom("invalid pending mutation queue"));
                    }
                    // Restore the retained sequence directly; staging would reissue
                    // OverwritePending entries instead of restoring their state.
                    queue.entries[queue.len] = item;
                    queue.len += 1;
                }
                Ok(queue)
            }
        }
        deserializer.deserialize_seq(QueueVisitor::<N>)
    }
}
