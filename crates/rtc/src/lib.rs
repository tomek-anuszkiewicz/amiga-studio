//! OKI MSM6242B Real-Time Clock & Calendar Emulation
//!
//! Emulates the battery-backed RTC chip found on the A501 expansion, A500+, and A2000.
//! Features 16 4-bit registers mapped at $DC0000..$DC003F on odd byte addresses.

use config::RtcModel;
use serde::{Deserialize, Serialize};

/// OKI MSM6242B Real-Time Clock state and register emulation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RtcMsm6242b {
    /// Active RTC hardware model (None vs Msm6242b)
    pub model: RtcModel,

    /// 16 4-bit register latches (positive BCD values)
    pub registers: [u8; 16],

    /// Control Register D ($D): Bit 0 = HOLD, Bit 1 = BUSY, Bit 2 = IRQ, Bit 3 = 30s ADJ
    pub control_d: u8,

    /// Control Register E ($E): Bit 0 = MASK, Bit 1 = ITRPT/STND, Bits 2-3 = t0, t1
    pub control_e: u8,

    /// Control Register F ($F): Bit 0 = RESET, Bit 1 = STOP, Bit 2 = 24/12, Bit 3 = TEST
    pub control_f: u8,

    /// Current simulated time in seconds since January 1, 1970 (Unix epoch)
    pub simulated_time: i64,

    /// Number of master Color Clock cycles elapsed since last 1-second step
    pub cck_accumulator: u64,
}

impl Default for RtcMsm6242b {
    fn default() -> Self {
        Self::new(RtcModel::Msm6242b)
    }
}

impl RtcMsm6242b {
    /// Creates a new RTC instance with default power-on register states
    pub fn new(model: RtcModel) -> Self {
        let mut rtc = Self {
            model,
            registers: [0; 16],
            // Hard reset defaults: HOLD = 1, 24-hour mode enabled
            control_d: 0x01,
            control_e: 0x00,
            control_f: 0x04,
            // Default baseline date: 1990-01-01 00:00:00 (Amiga Kickstart 1.3 / 2.0 era)
            simulated_time: 631152000,
            cck_accumulator: 0,
        };
        rtc.sync_time_to_registers();
        rtc
    }

    /// Sets the simulated RTC time to an explicit Unix timestamp (seconds since 1970-01-01)
    pub fn set_time(&mut self, timestamp: i64) {
        self.simulated_time = timestamp;
        self.sync_time_to_registers();
    }

    /// Advances the internal RTC clock by a given number of Color Clock (CCK) cycles (~3.54 MHz PAL)
    pub fn step_cck(&mut self, cck_cycles: u64) {
        if self.model == RtcModel::None || (self.control_f & 0x02) != 0 {
            // Clock stopped via STOP bit (Control F bit 1) or no RTC installed
            return;
        }

        // 3,546,895 Color Clocks per second on PAL A500
        const CCK_PER_SECOND: u64 = 3_546_895;

        self.cck_accumulator += cck_cycles;
        while self.cck_accumulator >= CCK_PER_SECOND {
            self.cck_accumulator -= CCK_PER_SECOND;
            self.simulated_time = self.simulated_time.wrapping_add(1);

            // Only update readable register latches if HOLD is cleared
            if (self.control_d & 0x01) == 0 {
                self.sync_time_to_registers();
            }
        }
    }

    /// Reads an 8-bit byte from the RTC address space ($DC0000..$DC003F)
    pub fn read_byte(&self, addr: u32) -> u8 {
        // Unmapped or even byte addresses (A0 = 0) return floating open bus
        if self.model == RtcModel::None || (addr & 1) == 0 {
            return 0xFF;
        }

        let reg = ((addr >> 2) & 0x0F) as usize;

        match reg {
            0x0..=0xC => self.registers[reg] & 0x0F,
            0xD => {
                // BUSY bit (Bit 1) is 0 when HOLD is set, else clear
                let busy = 0;
                (self.control_d & 0x0D) | (busy << 1)
            }
            0xE => self.control_e & 0x0F,
            0xF => self.control_f & 0x0F,
            _ => 0xFF,
        }
    }

    /// Writes an 8-bit byte to the RTC address space ($DC0000..$DC003F)
    pub fn write_byte(&mut self, addr: u32, val: u8) {
        if self.model == RtcModel::None || (addr & 1) == 0 {
            return;
        }

        let reg = ((addr >> 2) & 0x0F) as usize;
        let data = val & 0x0F;

        match reg {
            0x0..=0xC => {
                self.registers[reg] = data;
                self.sync_registers_to_time();
            }
            0xD => {
                // Bit 0 = HOLD, Bit 2 = IRQ FLAG (writing 0 clears), Bit 3 = 30s ADJ
                let old_hold = self.control_d & 0x01;
                let new_hold = data & 0x01;
                self.control_d = data;

                // When releasing HOLD (1 -> 0), sync current time to registers
                if old_hold == 1 && new_hold == 0 {
                    self.sync_time_to_registers();
                }

                // 30-second adjust: round seconds to nearest minute
                if (data & 0x08) != 0 {
                    let sec = (self.simulated_time % 60) as i32;
                    if sec < 30 {
                        self.simulated_time -= sec as i64;
                    } else {
                        self.simulated_time += (60 - sec) as i64;
                    }
                    self.sync_time_to_registers();
                }
            }
            0xE => {
                self.control_e = data;
            }
            0xF => {
                let old_f = self.control_f;
                self.control_f = data;

                // If 24/12 hour mode changed (Bit 2), re-encode registers
                if (old_f & 0x04) != (data & 0x04) {
                    self.sync_time_to_registers();
                }
            }
            _ => {}
        }
    }

    /// Synchronizes the internal `simulated_time` timestamp into the 16 4-bit BCD registers
    pub fn sync_time_to_registers(&mut self) {
        let (year, month, day, hour, min, sec, wday) = decompose_timestamp(self.simulated_time);

        // Seconds
        self.registers[0x0] = (sec % 10) as u8;
        self.registers[0x1] = (sec / 10) as u8;

        // Minutes
        self.registers[0x2] = (min % 10) as u8;
        self.registers[0x3] = (min / 10) as u8;

        // Hours (24h vs 12h mode per Control F Bit 2)
        if (self.control_f & 0x04) != 0 {
            // 24-hour mode
            self.registers[0x4] = (hour % 10) as u8;
            self.registers[0x5] = (hour / 10) as u8;
        } else {
            // 12-hour mode: Bit 2 = PM (1) / AM (0)
            let pm_bit = if hour >= 12 { 0x04 } else { 0x00 };
            let h12 = if hour == 0 {
                12
            } else if hour > 12 {
                hour - 12
            } else {
                hour
            };
            self.registers[0x4] = (h12 % 10) as u8;
            self.registers[0x5] = ((h12 / 10) as u8) | pm_bit;
        }

        // Day of month
        self.registers[0x6] = (day % 10) as u8;
        self.registers[0x7] = (day / 10) as u8;

        // Month (1-12)
        self.registers[0x8] = (month % 10) as u8;
        self.registers[0x9] = (month / 10) as u8;

        // Year (last 2 digits)
        let yr2 = year % 100;
        self.registers[0xA] = (yr2 % 10) as u8;
        self.registers[0xB] = (yr2 / 10) as u8;

        // Day of week (0 = Sunday .. 6 = Saturday)
        self.registers[0xC] = (wday % 7) as u8;
    }

    /// Synchronizes user-written BCD registers back into `simulated_time`
    pub fn sync_registers_to_time(&mut self) {
        let sec = (self.registers[0x1] * 10 + self.registers[0x0]) as u32;
        let min = (self.registers[0x3] * 10 + self.registers[0x2]) as u32;

        let hour = if (self.control_f & 0x04) != 0 {
            // 24-hour mode
            (self.registers[0x5] & 0x03) * 10 + self.registers[0x4]
        } else {
            // 12-hour mode
            let is_pm = (self.registers[0x5] & 0x04) != 0;
            let mut h = (self.registers[0x5] & 0x01) * 10 + self.registers[0x4];
            if is_pm && h < 12 {
                h += 12;
            } else if !is_pm && h == 12 {
                h = 0;
            }
            h
        } as u32;

        let day = (self.registers[0x7] * 10 + self.registers[0x6]).max(1) as u32;
        let month = (self.registers[0x9] * 10 + self.registers[0x8]).clamp(1, 12) as u32;
        let year_2d = (self.registers[0xB] * 10 + self.registers[0xA]) as i32;
        // Interpret year >= 78 as 1978..1999, else 2000..2077
        let year = if year_2d >= 78 {
            1900 + year_2d
        } else {
            2000 + year_2d
        };

        self.simulated_time = compose_timestamp(year, month, day, hour, min, sec);
    }
}

// =============================================================================
// Standalone Calendar Arithmetic (no_std / WASM safe without external crates)
// =============================================================================

/// Decomposes a Unix timestamp (seconds since 1970-01-01) into (year, month, day, hour, min, sec, wday)
fn decompose_timestamp(mut ts: i64) -> (i32, u32, u32, u32, u32, u32, u32) {
    if ts < 0 {
        ts = 0;
    }
    let sec = (ts % 60) as u32;
    ts /= 60;
    let min = (ts % 60) as u32;
    ts /= 60;
    let hour = (ts % 24) as u32;
    let days = ts / 24;

    // Day of week (1970-01-01 was a Thursday = 4)
    let wday = ((days + 4) % 7) as u32;

    // Civil day algorithm (Howard Hinnant formula)
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1020 + doe / 1460 - doe / 36524) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    (y as i32, m, d, hour, min, sec, wday)
}

/// Composes a calendar date into seconds since 1970-01-01 (Unix epoch)
fn compose_timestamp(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year } as i64;
    let m = month as i64;
    let d = day as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = (yoe as i64) * 365 + (yoe as i64) / 4 - (yoe as i64) / 100 + doy;
    let days = era * 146097 + doe - 719468;

    days * 86400 + (hour as i64) * 3600 + (min as i64) * 60 + (sec as i64)
}
