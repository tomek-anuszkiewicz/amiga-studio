//! Paula (MOS 8364) Architecture & Subsystem Coordinator
//!
//! 4-channel DMA audio, floppy disk MFM controller, serial UART transceiver,
//! and central interrupt multiplexer (INTENA, INTREQ -> IPL 1..6).

use config::{stage_mutation, tick_mutations, DelayedMutation, MutationMode};
use serde::{Deserialize, Serialize};

/// Fixed-capacity in-flight register mutation buffer for Paula (covers audio, disk, uart, int)
pub const PAULA_MUTATION_CAPACITY: usize = 32;

/// Paula custom chip coordinator
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paula {
    // --- Active Latched Registers (Read is NOW) ---
    /// Interrupt Enable register (INTENA / INTENAR at $DFF09A / $DFF01C)
    pub intena: u16,
    /// Interrupt Request register (INTREQ / INTREQR at $DFF09C / $DFF01E)
    pub intreq: u16,
    /// Audio / Disk / UART Control register (ADKCON / ADKCONR at $DFF09E / $DFF010)
    pub adkcon: u16,

    /// Potentiometer counters ($012, $014) and port direction ($016, $034)
    pub pot0dat: u16,
    pub pot1dat: u16,
    pub potgor: u16,
    pub potgo: u16,

    /// Serial UART data read ($018), write ($030), and period divisor ($032)
    pub serdatr: u16,
    pub serdat: u16,
    pub serper: u16,

    /// Floppy disk data byte read ($01A), length ($024), data ($026), and sync ($07E)
    pub dskbytr: u16,
    pub dsklen: u16,
    pub dskdat: u16,
    pub dsksync: u16,

    /// 4 Audio channels active state ($0A4-$0DA)
    pub audlen: [u16; 4],
    pub audper: [u16; 4],
    pub audvol: [u16; 4],
    pub auddat: [u16; 4],

    /// Paula-local DMA enables latched from DMACON ($096: AUD0..3EN, DSKEN)
    pub dma_enables: u16,

    /// Fixed inline in-flight mutation buffer (Zero-allocation)
    #[serde(with = "config::big_array")]
    pub mutations: [Option<DelayedMutation>; PAULA_MUTATION_CAPACITY],
}

impl Default for Paula {
    fn default() -> Self {
        Self::new()
    }
}

impl Paula {
    /// Creates a new Paula instance
    pub fn new() -> Self {
        Self {
            intena: 0,
            intreq: 0,
            adkcon: 0,
            pot0dat: 0,
            pot1dat: 0,
            potgor: 0,
            potgo: 0,
            serdatr: 0x3000, // TBE & TSRE set by default on power-on (TX empty)
            serdat: 0,
            serper: 0,
            dskbytr: 0,
            dsklen: 0,
            dskdat: 0,
            dsksync: 0x4489,
            audlen: [0; 4],
            audper: [0; 4],
            audvol: [0; 4],
            auddat: [0; 4],
            dma_enables: 0,
            mutations: [None; PAULA_MUTATION_CAPACITY],
        }
    }

    /// Resets Paula interrupt and audio registers to power-on defaults
    pub fn reset(&mut self) {
        self.intena = 0;
        self.intreq = 0;
        self.adkcon = 0;
        self.pot0dat = 0;
        self.pot1dat = 0;
        self.potgor = 0;
        self.potgo = 0;
        self.serdatr = 0x3000;
        self.serdat = 0;
        self.serper = 0;
        self.dskbytr = 0;
        self.dsklen = 0;
        self.dskdat = 0;
        self.dsksync = 0x4489;
        self.audlen.fill(0);
        self.audper.fill(0);
        self.audvol.fill(0);
        self.auddat.fill(0);
        self.dma_enables = 0;
        self.mutations = [None; PAULA_MUTATION_CAPACITY];
    }

    /// Advances Paula timers and processes in-flight register mutations by 1 Color Clock
    pub fn step_cck(&mut self) {
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
            0x018 => self.serdatr,
            0x01A => self.dskbytr,
            0x01C => self.intena,
            0x01E => self.intreq,
            _ => 0xFFFF,
        }
    }

    /// Schedules a staged register write with appropriate propagation delay and overwrite mode
    pub fn write_register(&mut self, offset: u16, val: u16) {
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
            0x0A4 | 0x0B4 | 0x0C4 | 0x0D4 => (2, MutationMode::OverwritePending), // AUDxLEN
            0x0A6 | 0x0B6 | 0x0C6 | 0x0D6 => (2, MutationMode::OverwritePending), // AUDxPER
            0x0A8 | 0x0B8 | 0x0C8 | 0x0D8 => (2, MutationMode::OverwritePending), // AUDxVOL
            0x0AA | 0x0BA | 0x0CA | 0x0DA => (1, MutationMode::Pipeline), // AUDxDAT
            _ => (2, MutationMode::OverwritePending),
        };

        if !stage_mutation(&mut self.mutations, offset, val, delay, mode) {
            self.commit_register_write(offset, val);
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
            }
            0x096 => {
                // DMACON broadcast: update Paula's local channel enables (AUD0..3EN, DSKEN)
                if (val & 0x8000) != 0 {
                    self.dma_enables |= val & 0x001F; // DSKEN (bit 4) + AUD3..0EN (bits 3..0)
                } else {
                    self.dma_enables &= !(val & 0x001F);
                }
            }
            0x030 => self.serdat = val,
            0x032 => self.serper = val,
            0x034 => self.potgo = val,
            0x024 => self.dsklen = val,
            0x026 => self.dskdat = val,
            0x07E => self.dsksync = val,

            // Audio channel registers
            0x0A4 => self.audlen[0] = val,
            0x0A6 => self.audper[0] = val,
            0x0A8 => self.audvol[0] = val & 0x007F,
            0x0AA => self.auddat[0] = val,

            0x0B4 => self.audlen[1] = val,
            0x0B6 => self.audper[1] = val,
            0x0B8 => self.audvol[1] = val & 0x007F,
            0x0BA => self.auddat[1] = val,

            0x0C4 => self.audlen[2] = val,
            0x0C6 => self.audper[2] = val,
            0x0C8 => self.audvol[2] = val & 0x007F,
            0x0CA => self.auddat[2] = val,

            0x0D4 => self.audlen[3] = val,
            0x0D6 => self.audper[3] = val,
            0x0D8 => self.audvol[3] = val & 0x007F,
            0x0DA => self.auddat[3] = val,

            _ => {}
        }
    }

    /// Writes INTENA register following SET/CLR bit 15 logic
    pub fn write_intena(&mut self, val: u16) {
        if (val & 0x8000) != 0 {
            self.intena |= val & 0x7FFF;
        } else {
            self.intena &= !(val & 0x7FFF);
        }
    }

    /// Writes INTREQ register following SET/CLR bit 15 logic
    pub fn write_intreq(&mut self, val: u16) {
        if (val & 0x8000) != 0 {
            self.intreq |= val & 0x7FFF;
        } else {
            self.intreq &= !(val & 0x7FFF);
        }
    }

    /// Asserts interrupt request bits immediately
    #[inline]
    pub fn set_interrupt_request(&mut self, mask: u16) {
        self.intreq |= mask & 0x7FFF;
    }

    /// Clears interrupt request bits immediately
    #[inline]
    pub fn clear_interrupt_request(&mut self, mask: u16) {
        self.intreq &= !(mask & 0x7FFF);
    }

    /// Evaluates pending, enabled interrupt sources and returns the highest active IPL (0..6)
    pub fn pending_interrupt_level(&self) -> u8 {
        // Master interrupt enable bit (INTEN, bit 14)
        if (self.intena & 0x4000) == 0 {
            return 0;
        }

        let pending = self.intreq & self.intena & 0x3FFF;
        if pending == 0 {
            return 0;
        }

        // Level 6: External / CIA-B (bit 13)
        if (pending & 0x2000) != 0 {
            return 6;
        }
        // Level 5: Disk Sync (bit 12) or Serial Receive (bit 11)
        if (pending & 0x1800) != 0 {
            return 5;
        }
        // Level 4: Audio channels 0..3 (bits 10..7)
        if (pending & 0x0780) != 0 {
            return 4;
        }
        // Level 3: Copper (bit 4), VBlank (bit 5), or Blitter (bit 6)
        if (pending & 0x0070) != 0 {
            return 3;
        }
        // Level 2: Ports / CIA-A (bit 3)
        if (pending & 0x0008) != 0 {
            return 2;
        }
        // Level 1: Serial Transmit (bit 0), Disk Block (bit 1), Software (bit 2)
        if (pending & 0x0007) != 0 {
            return 1;
        }

        0
    }
}
