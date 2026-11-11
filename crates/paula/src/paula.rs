//! Paula (MOS 8364) Architecture & Subsystem Coordinator
//!
//! 4-channel DMA audio, floppy disk MFM controller, serial UART transceiver,
//! and central interrupt multiplexer (INTENA, INTREQ -> IPL 1..6).

pub use audio;
pub use interrupts;
pub use interrupts::InterruptController;
pub mod serial;
pub use serial::SerialPort;

use config::{tick_mutations, DelayedMutation};
use serde::{Deserialize, Serialize};

mod pins;
pub use pins::{PaulaInputs, PaulaPins, PaulaSync};

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

pub const DSKBYTR_DSKBYT: u16 = 0x8000;
pub const DSKBYTR_DMAON: u16 = 0x4000;
pub const DSKBYTR_DISKWRITE: u16 = 0x2000;
pub const DSKBYTR_WORDEQUAL: u16 = 0x1000;
pub const DSKBYTR_DATA_BYTE: u16 = 0x00FF;
pub const DSKBYTR_DATA_MASK: u16 = 0x90FF;
pub const DSKLEN_WRITE_FLAG: u16 = 0x4000;
pub const DSKBYTR_DSKEN: u16 = 0x0010;

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
            potgor: config::mask::game_port::POT_DATA,
            potgo: 0,
            dskbytr: 0,
            dsklen: 0,
            dskdat: 0,
            dsksync: 0x4489,
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
        self.potgor = config::mask::game_port::POT_DATA;
        self.potgo = 0;
        self.dskbytr = 0;
        self.dsklen = 0;
        self.dskdat = 0;
        self.dsksync = 0x4489;
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
        if self.pins.rga_strobe.take() == Some(config::custom_reg::paula::STRHOR) {
            self.sync.cck_since_horizontal = 0;
        }
        self.audio.step_cck();
        for ch in 0..4 {
            if self.audio.poll_channel_irq(ch) {
                // Level 4 audio interrupt: bits 7..10 in INTREQ
                self.set_interrupt_request(1 << (7 + ch));
            }
        }
        self.serial_port.step_cck();
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
        self.sample_interrupt_pins();
        due
    }

    /// Returns true if disk DMA is active in DSKLEN
    #[inline]
    pub fn is_dsk_dma_active(&self) -> bool {
        self.dma_active
    }

    /// Returns true if DSKLEN is configured for write to disk (bit 14 set)
    #[inline]
    pub fn is_dsk_write(&self) -> bool {
        (self.dsklen & DSKLEN_WRITE_FLAG) != 0
    }

    /// Decrements DSKLEN word count after a DMA transfer slot, terminating DMA and returning true if finished
    pub fn decrement_dsklen(&mut self) -> bool {
        let len_words = self.dsklen & 0x3FFF;
        if len_words > 1 {
            self.dsklen = (self.dsklen & 0xC000) | (len_words - 1);
            false
        } else {
            self.dsklen &= 0xC000;
            self.dma_active = false;
            self.interrupts.request(interrupts::IRQ_DSKBLK);
            true
        }
    }

    /// Latches newly deserialized MFM data from the floppy drive into DSKBYTR,
    /// preserving Paula's internal DMAON and DISKWRITE status bits.
    #[inline]
    pub fn latch_floppy_dskbytr(&mut self, floppy_bits: u16) {
        self.dskbytr = (self.dskbytr & !DSKBYTR_DATA_MASK) | (floppy_bits & DSKBYTR_DATA_MASK);
    }

    /// Action method: latches a new deserialized MFM byte from the floppy drive bitstream
    #[inline]
    pub fn set_disk_byte(&mut self, byte: u8, sync_matched: bool) {
        let sync_bit = if sync_matched { DSKBYTR_WORDEQUAL } else { 0 };
        self.dskbytr = DSKBYTR_DSKBYT | sync_bit | (byte as u16);
    }

    /// Asserts interrupt request bits immediately
    #[inline]
    pub fn set_interrupt_request(&mut self, mask: u16) {
        self.interrupts.request(mask);
    }

    /// Polls and clears the audio DMA restart strobe (`AUDxDSR`) for channel `ch`
    #[inline]
    pub fn poll_audio_restart(&mut self, ch: usize) -> bool {
        self.audio.poll_restart_strobe(ch)
    }

    /// Evaluates pending, enabled interrupt sources and returns the highest active IPL (0..6)
    #[inline]
    pub fn pending_interrupt_level(&self) -> u8 {
        self.interrupts.pending_level()
    }
}
