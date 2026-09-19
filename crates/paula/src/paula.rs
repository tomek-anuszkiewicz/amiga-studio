//! Paula (MOS 8364) Architecture & Subsystem Coordinator
//!
//! 4-channel DMA audio, floppy disk MFM controller, serial UART transceiver,
//! and central interrupt multiplexer (INTENA, INTREQ -> IPL 1..6).

pub use audio;
pub use interrupts;
pub use interrupts::InterruptController;
pub mod serial;
pub use serial::SerialPort;

use config::{stage_mutation, tick_mutations, DelayedMutation, MutationMode};
use serde::{Deserialize, Serialize};

/// Fixed-capacity in-flight register mutation buffer for Paula (covers audio, disk, uart, int)
const PAULA_MUTATION_CAPACITY: usize = 32;

/// Paula custom chip coordinator
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paula {
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
    pub mutations: [Option<DelayedMutation>; PAULA_MUTATION_CAPACITY],
}

pub const DSKBYTR_DMAON: u16 = 0x4000;
pub const DSKBYTR_DISKWRITE: u16 = 0x2000;
pub const DSKBYTR_DATA_MASK: u16 = 0x90FF;
pub const DSKLEN_WRITE_FLAG: u16 = 0x4000;
pub const DSKBYTR_DSKEN: u16 = 0x0010;

impl Default for Paula {
    fn default() -> Self {
        Self::new()
    }
}

impl Paula {
    /// Creates a new Paula instance
    pub fn new() -> Self {
        Self {
            audio: audio::Audio::new(),
            serial_port: serial::SerialPort::new(),
            interrupts: interrupts::InterruptController::new(),
            adkcon: 0,
            pot0dat: 0,
            pot1dat: 0,
            potgor: 0,
            potgo: 0,
            dskbytr: 0,
            dsklen: 0,
            dskdat: 0,
            dsksync: 0x4489,
            dma_armed: false,
            dma_active: false,
            dma_enables: 0,
            dma_master: false,
            mutations: [None; PAULA_MUTATION_CAPACITY],
        }
    }

    /// Resets Paula interrupt and audio registers to power-on defaults
    pub fn reset(&mut self) {
        self.audio.reset();
        self.serial_port.reset();
        self.interrupts.reset();
        self.adkcon = 0;
        self.pot0dat = 0;
        self.pot1dat = 0;
        self.potgor = 0;
        self.potgo = 0;
        self.dskbytr = 0;
        self.dsklen = 0;
        self.dskdat = 0;
        self.dsksync = 0x4489;
        self.dma_armed = false;
        self.dma_active = false;
        self.dma_enables = 0;
        self.dma_master = false;
        self.mutations = [None; PAULA_MUTATION_CAPACITY];
    }

    /// Advances Paula timers and processes in-flight register mutations by 1 Color Clock.
    /// Returns any register writes that matured and committed on this exact cycle.
    pub fn step_cck(&mut self) -> [Option<(u16, u16)>; 8] {
        self.audio.step_cck();
        for ch in 0..4 {
            if self.audio.poll_channel_irq(ch) {
                // Level 4 audio interrupt: bits 7..10 in INTREQ
                self.set_interrupt_request(1 << (7 + ch));
            }
        }
        self.serial_port.step_cck();
        let mut due = [None; 8];
        let mut due_count = 0;
        tick_mutations(&mut self.mutations, |reg, val| {
            if due_count < due.len() {
                due[due_count] = Some((reg, val));
                due_count += 1;
            }
        });
        for item in due.iter().flatten() {
            self.commit_register_write(item.0, item.1);
        }
        due
    }

    /// Reads a Paula register by offset ($000..$1FE).
    /// Write-only registers return floating open bus $FFFF.
    #[inline]
    pub fn read_register(&self, offset: u16) -> u16 {
        match offset & 0x1FE {
            0x010 => self.adkcon,
            0x012 => self.pot0dat,
            0x014 => self.pot1dat,
            0x016 => self.potgor,
            0x018 => self.serial_port.serdatr,
            0x01A => self.peek_dskbytr(),
            0x01C => self.interrupts.read_intenar(),
            0x01E => self.interrupts.read_intreqr(),
            _ => 0xFFFF,
        }
    }

    /// Action method: writes DSKLEN register following the 2-write arming sequence
    pub fn write_dsklen(&mut self, val: u16) {
        self.dsklen = val;
        let dmaen = (val & 0x8000) != 0;
        if !dmaen {
            self.dma_armed = false;
            self.dma_active = false;
        } else if !self.dma_armed {
            self.dma_armed = true;
        } else {
            self.dma_active = true;
        }
    }

    /// Returns true if disk DMA is active in DSKLEN
    #[inline]
    pub fn is_dsk_dma_active(&self) -> bool {
        self.dma_active
    }

    /// Returns composite DSKBYTR status with live flags without side effects (for debuggers and peek inspection)
    #[inline]
    pub fn peek_dskbytr(&self) -> u16 {
        self.assemble_dskbytr(self.dskbytr)
    }

    /// Reads composite live DSKBYTR with Clear-on-Read hardware side-effects (clears bit 15 DSKBYT)
    #[inline]
    pub fn read_dskbytr(&mut self) -> u16 {
        let val = self.peek_dskbytr();
        self.dskbytr &= !0x8000;
        val
    }

    /// Action method: latches a new deserialized MFM byte from the floppy drive bitstream
    #[inline]
    pub fn set_disk_byte(&mut self, byte: u8, sync_matched: bool) {
        let sync_bit = if sync_matched { 0x1000 } else { 0 };
        self.dskbytr = 0x8000 | sync_bit | (byte as u16);
    }

    /// Assembles composite live DSKBYTR status from raw read word, DSKLEN, and Paula DMA enables.
    #[inline]
    pub fn assemble_dskbytr(&self, floppy_dskbytr: u16) -> u16 {
        let dmaon = if self.dma_master && (self.dma_enables & DSKBYTR_DSKEN) != 0 {
            DSKBYTR_DMAON
        } else {
            0
        };
        let diskwrite = if (self.dsklen & DSKLEN_WRITE_FLAG) != 0 {
            DSKBYTR_DISKWRITE
        } else {
            0
        };
        (floppy_dskbytr & DSKBYTR_DATA_MASK) | dmaon | diskwrite
    }

    /// Schedules a staged register write with appropriate propagation delay and overwrite mode.
    /// Returns `Some((offset, val))` if committed immediately, or `None` if staged in pipeline.
    pub fn write_register(&mut self, offset: u16, val: u16) -> Option<(u16, u16)> {
        let offset = offset & 0x1FE;
        let (delay, mode) = match offset {
            0x09A | 0x09C => (1, MutationMode::OverwritePending), // INTENA, INTREQ (1 CCK)
            0x09E => (2, MutationMode::OverwritePending),         // ADKCON (2 CCK)
            0x096 => (2, MutationMode::OverwritePending),         // DMACON broadcast (2 CCK)
            0x030 => (1, MutationMode::Pipeline),                 // SERDAT (1 CCK)
            0x032 => (2, MutationMode::OverwritePending),         // SERPER (2 CCK)
            0x034 => (2, MutationMode::OverwritePending),         // POTGO (2 CCK)
            0x024 => (2, MutationMode::OverwritePending),         // DSKLEN (2 CCK)
            0x07E => (2, MutationMode::OverwritePending),         // DSKSYNC (2 CCK)
            0x0A0 | 0x0A2 | 0x0B0 | 0x0B2 | 0x0C0 | 0x0C2 | 0x0D0 | 0x0D2 => {
                (2, MutationMode::OverwritePending)
            } // AUDxLCH / AUDxLCL
            0x0A4 | 0x0B4 | 0x0C4 | 0x0D4 => (2, MutationMode::OverwritePending), // AUDxLEN
            0x0A6 | 0x0B6 | 0x0C6 | 0x0D6 => (2, MutationMode::OverwritePending), // AUDxPER
            0x0A8 | 0x0B8 | 0x0C8 | 0x0D8 => (2, MutationMode::OverwritePending), // AUDxVOL
            0x0AA | 0x0BA | 0x0CA | 0x0DA => (1, MutationMode::Pipeline), // AUDxDAT
            _ => (2, MutationMode::OverwritePending),
        };

        if !stage_mutation(&mut self.mutations, offset, val, delay, mode) {
            self.commit_register_write(offset, val);
            Some((offset, val))
        } else {
            None
        }
    }

    /// Commits a register write directly into active Paula silicon state
    pub fn commit_register_write(&mut self, offset: u16, val: u16) {
        match offset & 0x1FE {
            0x09A => self.write_intena(val),
            0x09C => self.write_intreq(val),
            0x09E => {
                // ADKCON SET/CLR logic
                if (val & 0x8000) != 0 {
                    self.adkcon |= val & 0x7FFF;
                } else {
                    self.adkcon &= !(val & 0x7FFF);
                }
                self.audio.set_adkcon(self.adkcon);
            }
            0x096 => {
                // DMACON broadcast: update Paula's local channel enables (AUD0..3EN, DSKEN, DMAEN)
                if (val & 0x8000) != 0 {
                    if (val & 0x0200) != 0 {
                        self.dma_master = true;
                    }
                    self.dma_enables |= val & 0x001F;
                } else {
                    if (val & 0x0200) != 0 {
                        self.dma_master = false;
                    }
                    self.dma_enables &= !(val & 0x001F);
                }
                self.audio
                    .set_dma_enables((self.dma_enables & 0x000F) as u8, self.dma_master);
            }
            0x030 => self.serial_port.write_serdat(val),
            0x032 => self.serial_port.write_serper(val),
            0x034 => self.potgo = val,
            0x024 => self.write_dsklen(val),
            0x026 => self.dskdat = val,
            0x07E => self.dsksync = val,

            // Audio channel 0 pointer
            0x0A0 => {
                let high = (val as u32) << 16;
                let low = self.audio.channels[0].lc & 0x0000FFFF;
                self.audio.set_loc(0, high | low);
            }
            0x0A2 => {
                let high = self.audio.channels[0].lc & 0xFFFF0000;
                let low = val as u32;
                self.audio.set_loc(0, high | low);
            }
            0x0A4 => self.audio.set_len(0, val),
            0x0A6 => self.audio.set_per(0, val),
            0x0A8 => self.audio.set_vol(0, (val & 0x007F) as u8),
            0x0AA => self.audio.set_dat(0, val),

            // Audio channel 1 pointer
            0x0B0 => {
                let high = (val as u32) << 16;
                let low = self.audio.channels[1].lc & 0x0000FFFF;
                self.audio.set_loc(1, high | low);
            }
            0x0B2 => {
                let high = self.audio.channels[1].lc & 0xFFFF0000;
                let low = val as u32;
                self.audio.set_loc(1, high | low);
            }
            0x0B4 => self.audio.set_len(1, val),
            0x0B6 => self.audio.set_per(1, val),
            0x0B8 => self.audio.set_vol(1, (val & 0x007F) as u8),
            0x0BA => self.audio.set_dat(1, val),

            // Audio channel 2 pointer
            0x0C0 => {
                let high = (val as u32) << 16;
                let low = self.audio.channels[2].lc & 0x0000FFFF;
                self.audio.set_loc(2, high | low);
            }
            0x0C2 => {
                let high = self.audio.channels[2].lc & 0xFFFF0000;
                let low = val as u32;
                self.audio.set_loc(2, high | low);
            }
            0x0C4 => self.audio.set_len(2, val),
            0x0C6 => self.audio.set_per(2, val),
            0x0C8 => self.audio.set_vol(2, (val & 0x007F) as u8),
            0x0CA => self.audio.set_dat(2, val),

            // Audio channel 3 pointer
            0x0D0 => {
                let high = (val as u32) << 16;
                let low = self.audio.channels[3].lc & 0x0000FFFF;
                self.audio.set_loc(3, high | low);
            }
            0x0D2 => {
                let high = self.audio.channels[3].lc & 0xFFFF0000;
                let low = val as u32;
                self.audio.set_loc(3, high | low);
            }
            0x0D4 => self.audio.set_len(3, val),
            0x0D6 => self.audio.set_per(3, val),
            0x0D8 => self.audio.set_vol(3, (val & 0x007F) as u8),
            0x0DA => self.audio.set_dat(3, val),

            _ => {}
        }
    }

    /// Writes INTENA register following SET/CLR bit 15 logic
    #[inline]
    pub fn write_intena(&mut self, val: u16) {
        self.interrupts.write_intena(val);
    }

    /// Writes INTREQ register following SET/CLR bit 15 logic
    #[inline]
    pub fn write_intreq(&mut self, val: u16) {
        self.interrupts.write_intreq(val);
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

    /// Reads Audio/Disk Control register (ADKCONR at $DFF010)
    #[inline(always)]
    pub fn adkconr(&self) -> u16 {
        self.adkcon
    }

    /// Reads Audio/Disk Control register without side-effects for debugging
    #[inline(always)]
    pub fn adkconr_debug(&self) -> u16 {
        self.adkcon
    }

    /// Reads Potentiometer 0 data register (POT0DAT at $DFF012)
    #[inline(always)]
    pub fn pot0dat(&self) -> u16 {
        self.pot0dat
    }

    /// Reads Potentiometer 0 data register without side-effects for debugging
    #[inline(always)]
    pub fn pot0dat_debug(&self) -> u16 {
        self.pot0dat
    }

    /// Reads Potentiometer 1 data register (POT1DAT at $DFF014)
    #[inline(always)]
    pub fn pot1dat(&self) -> u16 {
        self.pot1dat
    }

    /// Reads Potentiometer 1 data register without side-effects for debugging
    #[inline(always)]
    pub fn pot1dat_debug(&self) -> u16 {
        self.pot1dat
    }

    /// Reads Potentiometer Port control/data register (POTGOR at $DFF016)
    #[inline(always)]
    pub fn potgor(&self) -> u16 {
        self.potgor
    }

    /// Reads Potentiometer Port control/data register without side-effects for debugging
    #[inline(always)]
    pub fn potgor_debug(&self) -> u16 {
        self.potgor
    }

    /// Reads Serial port data and status register (SERDATR at $DFF018)
    #[inline(always)]
    pub fn serdatr(&self) -> u16 {
        self.serial_port.serdatr
    }

    /// Reads Serial port data and status register without side-effects for debugging
    #[inline(always)]
    pub fn serdatr_debug(&self) -> u16 {
        self.serial_port.serdatr
    }

    /// Reads Interrupt enable register (INTENAR at $DFF01C)
    #[inline(always)]
    pub fn intenar(&self) -> u16 {
        self.interrupts.read_intenar()
    }

    /// Reads Interrupt enable register without side-effects for debugging
    #[inline(always)]
    pub fn intenar_debug(&self) -> u16 {
        self.interrupts.read_intenar()
    }

    /// Reads Interrupt request register (INTREQR at $DFF01E)
    #[inline(always)]
    pub fn intreqr(&self) -> u16 {
        self.interrupts.read_intreqr()
    }

    /// Reads Interrupt request register without side-effects for debugging
    #[inline(always)]
    pub fn intreqr_debug(&self) -> u16 {
        self.interrupts.read_intreqr()
    }
}
