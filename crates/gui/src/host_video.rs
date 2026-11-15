//! Host image ownership exchange, independent of GUI and machine execution APIs.
//!
//! The machine owns its drawing allocation. This exchange owns the second raw
//! image allocation. A producer seeds and swaps those allocations before
//! publication; the consumer releases the published one after conversion.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, TryLockError};

const STATE_FREE: u8 = 0;
const STATE_WRITING: u8 = 1;
const STATE_READY: u8 = 2;
const STATE_READING: u8 = 3;

/// Geometry and timeline belonging to one complete or debugger-stopped image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameInfo {
    pub width: usize,
    pub height: usize,
    pub row_stride: usize,
    pub field_number: u64,
    pub publication: u64,
    pub cck: u64,
    pub generation: u64,
    pub interlaced: bool,
    pub field_odd: bool,
    pub complete: bool,
}

impl FrameInfo {
    fn visible_storage_len(self, capacity: usize) -> Option<usize> {
        if self.width == 0 || self.height == 0 || self.width > self.row_stride {
            return None;
        }
        self.height
            .checked_mul(self.row_stride)
            .filter(|length| *length <= capacity)
    }
}

#[derive(Debug)]
struct FrameStorage {
    pixels: Vec<u32>,
    info: Option<FrameInfo>,
}

/// A failed exchange is a host runtime error, not a guest execution exception.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoExchangeError {
    InvalidFrame,
    PoisonedStorage,
}

impl std::fmt::Display for VideoExchangeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidFrame => "Invalid host video frame or storage capacity",
            Self::PoisonedStorage => "Host video storage was poisoned",
        })
    }
}

impl std::error::Error for VideoExchangeError {}

/// Whether a consumer released storage, including an obsolete timeline image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoRead<T> {
    Pending,
    Discarded,
    Frame(T),
}

/// Single-producer/single-consumer handoff with one pending published image.
///
/// The atomic state owns the access interval. The mutex makes pixel/metadata
/// access safe Rust; both sides only attempt a nonblocking lock after a claim.
/// References stay inside callbacks, and the lock is dropped before publishing
/// the next state. No thread, browser handle or serialization lives here.
#[derive(Debug)]
pub struct VideoExchange {
    state: AtomicU8,
    storage: Mutex<FrameStorage>,
}

struct StateClaim<'a> {
    state: &'a AtomicU8,
    release_state: u8,
}

impl Drop for StateClaim<'_> {
    fn drop(&mut self) {
        self.state.store(self.release_state, Ordering::Release);
    }
}

impl VideoExchange {
    /// Takes the spare allocation; the machine retains the other allocation.
    pub fn new(spare: Vec<u32>) -> Self {
        Self {
            state: AtomicU8::new(STATE_FREE),
            storage: Mutex::new(FrameStorage {
                pixels: spare,
                info: None,
            }),
        }
    }

    fn claim(&self, current: u8, next: u8) -> Option<StateClaim<'_>> {
        self.state
            .compare_exchange(current, next, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| StateClaim {
                state: &self.state,
                release_state: current,
            })
    }

    /// Claims a free exchange buffer and publishes the producer's selected image.
    ///
    /// The producer callback seeds/swaps the spare with its drawing allocation
    /// and returns matching metadata. `Ok(false)` applies backpressure: neither
    /// READY nor READING storage is changed, and the callback is not called.
    /// A successful return tells the host to notify the GUI separately.
    pub fn try_publish(
        &self,
        prepare: impl FnOnce(&mut Vec<u32>) -> Option<FrameInfo>,
    ) -> Result<bool, VideoExchangeError> {
        let Some(mut claim) = self.claim(STATE_FREE, STATE_WRITING) else {
            return Ok(false);
        };
        let mut storage = match self.storage.try_lock() {
            Ok(storage) => storage,
            Err(TryLockError::WouldBlock) => return Ok(false),
            Err(TryLockError::Poisoned(_)) => return Err(VideoExchangeError::PoisonedStorage),
        };
        let info = prepare(&mut storage.pixels).ok_or(VideoExchangeError::InvalidFrame)?;
        if info.visible_storage_len(storage.pixels.len()).is_none() {
            return Err(VideoExchangeError::InvalidFrame);
        }
        storage.info = Some(info);
        drop(storage);
        claim.release_state = STATE_READY;
        Ok(true)
    }

    /// Reads one image of the current timeline, then releases its raw allocation.
    ///
    /// A stale-generation image is released without calling the reader. The
    /// reader converts into GUI-owned data before returning; neither its pixel
    /// slice nor any mutable machine reference can escape this access interval.
    /// The host can wake a waiting producer after a frame has been released.
    pub fn try_consume<T>(
        &self,
        generation: u64,
        read: impl FnOnce(FrameInfo, &[u32]) -> T,
    ) -> Result<VideoRead<T>, VideoExchangeError> {
        let Some(mut claim) = self.claim(STATE_READY, STATE_READING) else {
            return Ok(VideoRead::Pending);
        };
        let storage = match self.storage.try_lock() {
            Ok(storage) => storage,
            Err(TryLockError::WouldBlock) => return Ok(VideoRead::Pending),
            Err(TryLockError::Poisoned(_)) => return Err(VideoExchangeError::PoisonedStorage),
        };
        claim.release_state = STATE_FREE;
        let info = storage.info.ok_or(VideoExchangeError::InvalidFrame)?;
        let length = info
            .visible_storage_len(storage.pixels.len())
            .ok_or(VideoExchangeError::InvalidFrame)?;
        if info.generation != generation {
            return Ok(VideoRead::Discarded);
        }
        Ok(VideoRead::Frame(read(info, &storage.pixels[..length])))
    }
}
