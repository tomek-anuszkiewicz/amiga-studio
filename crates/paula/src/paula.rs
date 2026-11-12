//! Paula (MOS 8364) Architecture & Subsystem Coordinator
//!
//! 4-channel DMA audio, floppy disk MFM controller, serial UART transceiver,
//! and central interrupt multiplexer (INTENA, INTREQ -> IPL 1..6).

pub mod audio;
pub use audio::Audio;
pub mod interrupts;
pub use interrupts::InterruptController;
pub mod serial;
pub use serial::SerialPort;

use crate::register_bits::intreq;
use config::{tick_mutations, DelayedMutation};
use serde::{Deserialize, Serialize};

mod pins;
pub use pins::{PaulaInputs, PaulaPins, PaulaSync};

const DSKSYNC_RESET: u16 = 0x4489;
const MASK_DSKLEN_LENGTH: u16 = 0x3FFF;
const MASK_DSKLEN_CONTROL: u16 = 0xC000;

mod disk;
mod dmal;
pub use disk::DiskEngine;
mod register_bits;
mod registers;

/// Paula custom chip coordinator
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paula {
    /// Motherboard-facing signal state.
    pub pins: PaulaPins,
    /// Local phase since the last received horizontal synchronization strobe.
    pub sync: PaulaSync,
    /// 4-channel DMA audio subsystem
    pub audio: audio::Audio,
    /// RS-232 serial UART transceiver
    pub serial_port: serial::SerialPort,
    /// Central interrupt priority controller (INTENA / INTREQ / IPL 1..6)
    pub interrupts: interrupts::InterruptController,
    /// Audio / Disk / UART Control register (ADKCON / ADKCONR at $DFF09E / $DFF010)
    pub adkcon: u16,

    /// Potentiometer counters ($012, $014) and port direction ($016, $034)
    pub pot0dat: u16,
    pub pot1dat: u16,
    pub potgor: u16,
    pub potgo: u16,

    /// Floppy disk data byte read ($01A), length ($024), data ($026), and sync ($07E)
    pub dskbytr: u16,
    pub dsklen: u16,
    pub dskdat: u16,
    pub dsksync: u16,
    pub disk: DiskEngine,
    /// True if DSKLEN write 1 has armed the disk DMA sequence
    pub dma_armed: bool,
    /// True if DSKLEN write 2 has activated the disk DMA transfer
    pub dma_active: bool,

    /// Paula-local DMA enables latched from DMACON ($096: AUD0..3EN, DSKEN)
    pub dma_enables: u16,
    /// Master DMA enable flag latched from DMACON bit 9 (DMAEN)
    pub dma_master: bool,

    /// Fixed inline in-flight mutation buffer (Zero-allocation)
    #[serde(with = "config::big_array")]
    pub mutations: [Option<DelayedMutation>; Paula::MUTATION_CAPACITY],
}

pub const BIT_DSKLEN_WRITE_FLAG: u16 = 0x4000;

impl Default for Paula {
    fn default() -> Self {
        Self::new()
    }
}

impl Paula {
    /// Fixed-capacity in-flight register mutation buffer for Paula (covers audio, disk, uart, int)
    pub const MUTATION_CAPACITY: usize = 32;

    /// Creates a new Paula instance
    pub fn new() -> Self {
        Self {
            pins: PaulaPins::default(),
            sync: PaulaSync::default(),
            audio: audio::Audio::new(),
            serial_port: serial::SerialPort::new(),
            interrupts: interrupts::InterruptController::new(),
            adkcon: 0,
            pot0dat: 0,
            pot1dat: 0,
            potgor: config::mask::game_port::MASK_POT_DATA,
            potgo: 0,
            dskbytr: 0,
            dsklen: 0,
            dskdat: 0,
            dsksync: DSKSYNC_RESET,
            disk: DiskEngine::default(),
            dma_armed: false,
            dma_active: false,
            dma_enables: 0,
            dma_master: false,
            mutations: [None; Self::MUTATION_CAPACITY],
        }
    }

    /// Resets Paula interrupt and audio registers to power-on defaults
    pub fn reset(&mut self) {
        self.pins = PaulaPins::default();
        self.sync = PaulaSync::default();
        self.audio.reset();
        self.serial_port.reset();
        self.interrupts.reset();
        self.adkcon = 0;
        self.pot0dat = 0;
        self.pot1dat = 0;
        self.potgor = config::mask::game_port::MASK_POT_DATA;
        self.potgo = 0;
        self.dskbytr = 0;
        self.dsklen = 0;
        self.dskdat = 0;
        self.dsksync = DSKSYNC_RESET;
        self.disk = DiskEngine::default();
        self.dma_armed = false;
        self.dma_active = false;
        self.dma_enables = 0;
        self.dma_master = false;
        self.mutations = [None; Self::MUTATION_CAPACITY];
    }

    /// Advances Paula timers and processes in-flight register mutations by 1 Color Clock.
    /// Returns any register writes that matured and committed on this exact cycle.
    pub fn step_cck(&mut self) -> [Option<(u16, u16)>; Self::MUTATION_CAPACITY] {
        self.sync.cck_since_horizontal = self.sync.cck_since_horizontal.wrapping_add(1);
        let vertical_blank = self
            .pins
            .rga_strobe
            .take()
            .is_some_and(|address| self.receive_sync_strobe(address));
        self.audio
            .step_cck((self.interrupts.intreq >> intreq::BIT_AUD0.trailing_zeros()) as u8);
        self.step_disk();
        self.drive_dmal();
        let serial_irq = self
            .serial_port
            .step_cck(true, self.interrupts.intreq & intreq::BIT_RBF != 0);
        let mut due = [None; Self::MUTATION_CAPACITY];
        let mut due_count = 0;
        tick_mutations(&mut self.mutations, |reg, val| {
            if due_count < due.len() {
                due[due_count] = Some((reg, val));
                due_count += 1;
            }
        });
        for item in due.iter().flatten() {
            self.apply_register_write(item.0, item.1);
        }
        if vertical_blank {
            self.set_interrupt_request(intreq::BIT_VERTB);
        }
        for ch in 0..config::AUDIO_CHANNEL_COUNT {
            if self.audio.poll_channel_irq(ch) {
                // Level 4 audio interrupt: bits 7..10 in INTREQ
                self.set_interrupt_request(intreq::BIT_AUD0 << ch);
            }
        }
        self.interrupts
            .request(serial_irq | core::mem::take(&mut self.disk.irq_pending));
        self.sample_interrupt_pins();
        due
    }

    /// Returns true if DSKLEN is configured for write to disk (bit 14 set)
    #[inline]
    pub fn is_dsk_write(&self) -> bool {
        (self.dsklen & BIT_DSKLEN_WRITE_FLAG) != 0
    }

    /// Asserts interrupt request bits immediately
    #[inline]
    pub fn set_interrupt_request(&mut self, mask: u16) {
        self.interrupts.request(mask);
    }

    /// Evaluates pending, enabled interrupt sources and returns the highest active IPL (0..6)
    #[inline]
    pub fn pending_interrupt_level(&self) -> u8 {
        self.interrupts.pending_level()
    }
}
