//! OKI MSM6242B Real-Time Clock & Calendar Emulation
//!
//! Emulates the battery-backed RTC chip found on the A501 expansion, A500+, and A2000.
//! Features 16 4-bit registers mapped at $DC0000..$DC003F on odd byte addresses.

use config::RtcModel;
use serde::{Deserialize, Serialize};

const LEAP_YEAR_PERIOD: u32 = 4;
const CENTURY_YEARS: u32 = 100;
const MONTH_GROUP_SIZE: u32 = 5;
const MONTH_GROUP_DAYS: u32 = 153;
const MONTH_ROUNDING_BIAS: u32 = 2;
const MARCH_TO_DECEMBER_MONTHS: u32 = 10;
const MARCH_MONTH_NUMBER: u32 = 3;
const JANUARY_FEBRUARY_OFFSET: u32 = 9;
const FEBRUARY_MONTH_NUMBER: u32 = 2;

const REGISTER_SECONDS_UNITS: usize = 0;
const REGISTER_SECONDS_TENS: usize = 1;
const REGISTER_MINUTES_UNITS: usize = 2;
const REGISTER_MINUTES_TENS: usize = 3;
const REGISTER_HOURS_UNITS: usize = 4;
const REGISTER_HOURS_TENS: usize = 5;
const REGISTER_DAY_UNITS: usize = 6;
const REGISTER_DAY_TENS: usize = 7;
const REGISTER_MONTH_UNITS: usize = 8;
const REGISTER_MONTH_TENS: usize = 9;
const REGISTER_YEAR_UNITS: usize = 10;
const REGISTER_YEAR_TENS: usize = 11;
const REGISTER_WEEKDAY: usize = 12;
const REGISTER_CONTROL_D: usize = 13;
const REGISTER_CONTROL_E: usize = 14;
const REGISTER_CONTROL_F: usize = 15;
const REGISTER_COUNT: usize = 16;
const BIT_REGISTER_ADDRESS_SHIFT: u32 = 2;
const MASK_REGISTER_SELECTOR: u32 = 0x0F;
const MASK_REGISTER_DATA: u8 = 0x0F;
const BIT_CONTROL_D_HOLD: u8 = 0x01;
const BIT_CONTROL_D_ADJUST: u8 = 0x08;
const MASK_CONTROL_D_READ: u8 = 0x0D;
const CONTROL_D_NOT_BUSY: u8 = 0;
const BIT_CONTROL_D_BUSY_SHIFT: u8 = 1;
const BIT_CONTROL_F_STOP: u8 = 0x02;
const BIT_CONTROL_F_24_HOUR: u8 = 0x04;
const BIT_HOURS_PM: u8 = 0x04;
const MASK_HOURS_24_TENS: u8 = 0x03;
const MASK_HOURS_12_TENS: u8 = 0x01;
const RESET_TIMESTAMP: i64 = 631152000;
const SECONDS_PER_MINUTE: i32 = 60;
const ADJUST_HALF_MINUTE_SECONDS: i32 = 30;
const HOURS_PER_HALF_DAY: u32 = 12;
const HOURS_PER_DAY: u32 = 24;
const DAYS_PER_WEEK: u32 = 7;
const MONTHS_PER_YEAR: u8 = 12;
const TWO_DIGIT_YEAR_MODULUS: i32 = 100;
const AMIGA_EPOCH_YEAR: i32 = 78;
const PREVIOUS_CENTURY: i32 = 1900;
const CURRENT_CENTURY: i32 = 2000;
const UNIX_EPOCH_WEEKDAY: i64 = 4;
const UNIX_EPOCH_MARCH_OFFSET: i64 = 719468;
const DAYS_PER_ERA: i64 = 146097;
const YEARS_PER_ERA: i64 = 400;
const DAYS_PER_YEAR: u32 = 365;
const YEAR_ESTIMATE_CORRECTION_DIVISOR: u32 = 1020;
const FOUR_YEAR_DAYS: u32 = 1460;
const CENTURY_DAYS: u32 = 36524;
const SECONDS_PER_HOUR: i64 = 3600;
const SECONDS_PER_DAY: i64 = 86400;

/// OKI MSM6242B Real-Time Clock state and register emulation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RtcMsm6242b {
    /// Active RTC hardware model (None vs Msm6242b)
    pub model: RtcModel,

    /// 16 4-bit register latches (positive BCD values)
    pub registers: [u8; REGISTER_COUNT],

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
            registers: [0; REGISTER_COUNT],
            // Hard reset defaults: HOLD = 1, 24-hour mode enabled
            control_d: BIT_CONTROL_D_HOLD,
            control_e: 0x00,
            control_f: BIT_CONTROL_F_24_HOUR,
            // Default baseline date: 1990-01-01 00:00:00 (Amiga Kickstart 1.3 / 2.0 era)
            simulated_time: RESET_TIMESTAMP,
            cck_accumulator: 0,
        };
        rtc.sync_time_to_registers();
        rtc
    }

    /// Sets the simulated RTC time to an explicit Unix timestamp (seconds since 1970-01-01)
    #[inline]
    pub fn set_time(&mut self, timestamp: i64) {
        self.simulated_time = timestamp;
        self.sync_time_to_registers();
    }

    /// Advances the internal RTC clock by a given number of Color Clock (CCK) cycles (~3.54 MHz PAL)
    pub fn step_cck(&mut self, cck_cycles: u64) {
        if self.model == RtcModel::None || (self.control_f & BIT_CONTROL_F_STOP) != 0 {
            // Clock stopped via STOP bit (Control F bit 1) or no RTC installed
            return;
        }

        // 3,546,895 Color Clocks per second on PAL A500
        const CCK_PER_SECOND: u64 = 3_546_895;

        self.cck_accumulator += cck_cycles;
        if self.cck_accumulator >= CCK_PER_SECOND {
            let seconds = self.cck_accumulator / CCK_PER_SECOND;
            self.cck_accumulator %= CCK_PER_SECOND;
            self.simulated_time = self.simulated_time.wrapping_add(seconds as i64);

            // Only update readable register latches if HOLD is cleared
            if (self.control_d & BIT_CONTROL_D_HOLD) == 0 {
                self.sync_time_to_registers();
            }
        }
    }

    /// Reads an 8-bit byte from the RTC address space ($DC0000..$DC003F)
    pub fn read_byte(&self, addr: u32) -> u8 {
        // Unmapped or even byte addresses (A0 = 0) return floating open bus
        if self.model == RtcModel::None || (addr & 1) == 0 {
            return config::OPEN_BUS_BYTE;
        }

        let reg = ((addr >> BIT_REGISTER_ADDRESS_SHIFT) & MASK_REGISTER_SELECTOR) as usize;

        match reg {
            REGISTER_SECONDS_UNITS..=REGISTER_WEEKDAY => self.registers[reg] & MASK_REGISTER_DATA,
            REGISTER_CONTROL_D => {
                // BUSY bit (Bit 1) is 0 when HOLD is set, else clear
                let busy = CONTROL_D_NOT_BUSY;
                (self.control_d & MASK_CONTROL_D_READ) | (busy << BIT_CONTROL_D_BUSY_SHIFT)
            }
            REGISTER_CONTROL_E => self.control_e & MASK_REGISTER_DATA,
            REGISTER_CONTROL_F => self.control_f & MASK_REGISTER_DATA,
            _ => config::OPEN_BUS_BYTE,
        }
    }

    /// Writes an 8-bit byte to the RTC address space ($DC0000..$DC003F)
    pub fn write_byte(&mut self, addr: u32, val: u8) {
        if self.model == RtcModel::None || (addr & 1) == 0 {
            return;
        }

        let reg = ((addr >> BIT_REGISTER_ADDRESS_SHIFT) & MASK_REGISTER_SELECTOR) as usize;
        let data = val & MASK_REGISTER_DATA;

        match reg {
            REGISTER_SECONDS_UNITS..=REGISTER_WEEKDAY => {
                self.registers[reg] = data;
                self.sync_registers_to_time();
            }
            REGISTER_CONTROL_D => {
                // Bit 0 = HOLD, Bit 2 = IRQ FLAG (writing 0 clears), Bit 3 = 30s ADJ
                let old_hold = self.control_d & BIT_CONTROL_D_HOLD;
                let new_hold = data & BIT_CONTROL_D_HOLD;
                self.control_d = data;

                // When releasing HOLD (1 -> 0), sync current time to registers
                if old_hold == BIT_CONTROL_D_HOLD && new_hold == 0 {
                    self.sync_time_to_registers();
                }

                // 30-second adjust: round seconds to nearest minute
                if (data & BIT_CONTROL_D_ADJUST) != 0 {
                    let sec = (self.simulated_time % i64::from(SECONDS_PER_MINUTE)) as i32;
                    if sec < ADJUST_HALF_MINUTE_SECONDS {
                        self.simulated_time -= sec as i64;
                    } else {
                        self.simulated_time += (SECONDS_PER_MINUTE - sec) as i64;
                    }
                    self.sync_time_to_registers();
                }
            }
            REGISTER_CONTROL_E => {
                self.control_e = data;
            }
            REGISTER_CONTROL_F => {
                let old_f = self.control_f;
                self.control_f = data;

                // If 24/12 hour mode changed (Bit 2), re-encode registers
                if (old_f & BIT_CONTROL_F_24_HOUR) != (data & BIT_CONTROL_F_24_HOUR) {
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
        self.registers[REGISTER_SECONDS_UNITS] = (sec % 10) as u8;
        self.registers[REGISTER_SECONDS_TENS] = (sec / 10) as u8;

        // Minutes
        self.registers[REGISTER_MINUTES_UNITS] = (min % 10) as u8;
        self.registers[REGISTER_MINUTES_TENS] = (min / 10) as u8;

        // Hours (24h vs 12h mode per Control F Bit 2)
        if (self.control_f & BIT_CONTROL_F_24_HOUR) != 0 {
            // 24-hour mode
            self.registers[REGISTER_HOURS_UNITS] = (hour % 10) as u8;
            self.registers[REGISTER_HOURS_TENS] = (hour / 10) as u8;
        } else {
            // 12-hour mode: Bit 2 = PM (1) / AM (0)
            let pm_bit = if hour >= HOURS_PER_HALF_DAY {
                BIT_HOURS_PM
            } else {
                0x00
            };
            let h12 = if hour == 0 {
                HOURS_PER_HALF_DAY
            } else if hour > HOURS_PER_HALF_DAY {
                hour - HOURS_PER_HALF_DAY
            } else {
                hour
            };
            self.registers[REGISTER_HOURS_UNITS] = (h12 % 10) as u8;
            self.registers[REGISTER_HOURS_TENS] = ((h12 / 10) as u8) | pm_bit;
        }

        // Day of month
        self.registers[REGISTER_DAY_UNITS] = (day % 10) as u8;
        self.registers[REGISTER_DAY_TENS] = (day / 10) as u8;

        // Month (1-12)
        self.registers[REGISTER_MONTH_UNITS] = (month % 10) as u8;
        self.registers[REGISTER_MONTH_TENS] = (month / 10) as u8;

        // Year (last 2 digits)
        let yr2 = year % TWO_DIGIT_YEAR_MODULUS;
        self.registers[REGISTER_YEAR_UNITS] = (yr2 % 10) as u8;
        self.registers[REGISTER_YEAR_TENS] = (yr2 / 10) as u8;

        // Day of week (0 = Sunday .. 6 = Saturday)
        self.registers[REGISTER_WEEKDAY] = (wday % DAYS_PER_WEEK) as u8;
    }

    /// Synchronizes user-written BCD registers back into `simulated_time`
    pub fn sync_registers_to_time(&mut self) {
        let sec = (self.registers[REGISTER_SECONDS_TENS] * 10
            + self.registers[REGISTER_SECONDS_UNITS]) as u32;
        let min = (self.registers[REGISTER_MINUTES_TENS] * 10
            + self.registers[REGISTER_MINUTES_UNITS]) as u32;

        let hour = if (self.control_f & BIT_CONTROL_F_24_HOUR) != 0 {
            // 24-hour mode
            (self.registers[REGISTER_HOURS_TENS] & MASK_HOURS_24_TENS) * 10
                + self.registers[REGISTER_HOURS_UNITS]
        } else {
            // 12-hour mode
            let is_pm = (self.registers[REGISTER_HOURS_TENS] & BIT_HOURS_PM) != 0;
            let mut h = (self.registers[REGISTER_HOURS_TENS] & MASK_HOURS_12_TENS) * 10
                + self.registers[REGISTER_HOURS_UNITS];
            if is_pm && h < HOURS_PER_HALF_DAY as u8 {
                h += HOURS_PER_HALF_DAY as u8;
            } else if !is_pm && h == HOURS_PER_HALF_DAY as u8 {
                h = 0;
            }
            h
        } as u32;

        let day = (self.registers[REGISTER_DAY_TENS] * 10 + self.registers[REGISTER_DAY_UNITS])
            .max(1) as u32;
        let month = (self.registers[REGISTER_MONTH_TENS] * 10
            + self.registers[REGISTER_MONTH_UNITS])
            .clamp(1, MONTHS_PER_YEAR) as u32;
        let year_2d =
            (self.registers[REGISTER_YEAR_TENS] * 10 + self.registers[REGISTER_YEAR_UNITS]) as i32;
        // Interpret year >= 78 as 1978..1999, else 2000..2077
        let year = if year_2d >= AMIGA_EPOCH_YEAR {
            PREVIOUS_CENTURY + year_2d
        } else {
            CURRENT_CENTURY + year_2d
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
    let sec = (ts % i64::from(SECONDS_PER_MINUTE)) as u32;
    ts /= i64::from(SECONDS_PER_MINUTE);
    let min = (ts % i64::from(SECONDS_PER_MINUTE)) as u32;
    ts /= i64::from(SECONDS_PER_MINUTE);
    let hour = (ts % i64::from(HOURS_PER_DAY)) as u32;
    let days = ts / i64::from(HOURS_PER_DAY);

    // Day of week (1970-01-01 was a Thursday = 4)
    let wday = ((days + UNIX_EPOCH_WEEKDAY) % i64::from(DAYS_PER_WEEK)) as u32;

    // Civil day algorithm (Howard Hinnant formula)
    let z = days + UNIX_EPOCH_MARCH_OFFSET;
    let era = if z >= 0 { z } else { z - (DAYS_PER_ERA - 1) } / DAYS_PER_ERA;
    let doe = (z - era * DAYS_PER_ERA) as u32;
    let yoe = (doe - doe / YEAR_ESTIMATE_CORRECTION_DIVISOR + doe / FOUR_YEAR_DAYS
        - doe / CENTURY_DAYS)
        / DAYS_PER_YEAR;
    let y = (yoe as i64) + era * YEARS_PER_ERA;
    let doy = doe - (DAYS_PER_YEAR * yoe + yoe / LEAP_YEAR_PERIOD - yoe / CENTURY_YEARS);
    let mp = (MONTH_GROUP_SIZE * doy + MONTH_ROUNDING_BIAS) / MONTH_GROUP_DAYS;
    let d = doy - (MONTH_GROUP_DAYS * mp + MONTH_ROUNDING_BIAS) / MONTH_GROUP_SIZE + 1;
    let m = if mp < MARCH_TO_DECEMBER_MONTHS {
        mp + MARCH_MONTH_NUMBER
    } else {
        mp - JANUARY_FEBRUARY_OFFSET
    };
    let y = if m <= FEBRUARY_MONTH_NUMBER { y + 1 } else { y };

    (y as i32, m, d, hour, min, sec, wday)
}

/// Composes a calendar date into seconds since 1970-01-01 (Unix epoch)
fn compose_timestamp(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> i64 {
    let y = if month <= FEBRUARY_MONTH_NUMBER {
        year - 1
    } else {
        year
    } as i64;
    let m = month as i64;
    let d = day as i64;
    let era = if y >= 0 { y } else { y - (YEARS_PER_ERA - 1) } / YEARS_PER_ERA;
    let yoe = (y - era * YEARS_PER_ERA) as u32;
    let mp = if m > i64::from(FEBRUARY_MONTH_NUMBER) {
        m - i64::from(MARCH_MONTH_NUMBER)
    } else {
        m + i64::from(JANUARY_FEBRUARY_OFFSET)
    };
    let doy = (i64::from(MONTH_GROUP_DAYS) * mp + i64::from(MONTH_ROUNDING_BIAS))
        / i64::from(MONTH_GROUP_SIZE)
        + d
        - 1;
    let doe = (yoe as i64) * i64::from(DAYS_PER_YEAR) + (yoe as i64) / i64::from(LEAP_YEAR_PERIOD)
        - (yoe as i64) / i64::from(CENTURY_YEARS)
        + doy;
    let days = era * DAYS_PER_ERA + doe - UNIX_EPOCH_MARCH_OFFSET;

    days * SECONDS_PER_DAY
        + (hour as i64) * SECONDS_PER_HOUR
        + (min as i64) * i64::from(SECONDS_PER_MINUTE)
        + (sec as i64)
}
