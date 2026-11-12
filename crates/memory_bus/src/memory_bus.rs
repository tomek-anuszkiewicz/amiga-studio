//! Motherboard MemoryBus Router & Subsystem Dispatch
//!
//! Provides zero-cost stack-allocated 24-bit physical address routing across
//! physical storage (`PhysicalMemory`), custom chips (`Agnus`, `Denise`, `Paula`),
//! peripheral controllers (`CIA-A`, `CIA-B`), and expansion hardware (`RTC`).

use agnus::Agnus;
use cia::Cia;
use config::address_map::{CIA_BANK_FIRST, CIA_BANK_LAST, CUSTOM_BANK_FIRST, CUSTOM_BANK_LAST};
use config::OPEN_BUS_BYTE;
use denise::Denise;
use paula::Paula;
use physical_memory::{AddressBus, BusResult, MemoryBank, PhysicalMemory};
use rtc::RtcMsm6242b;

pub use physical_memory;

mod cia_registers;
mod custom_registers;

/// Zero-cost stack-allocated router implementing `AddressBus` across all subsystems
#[derive(Debug)]
pub struct MemoryBus<'a> {
    pub mem: &'a mut PhysicalMemory,
    pub agnus: &'a mut Agnus,
    pub denise: &'a mut Denise,
    pub paula: &'a mut Paula,
    pub cia_a: &'a mut Cia,
    pub cia_b: &'a mut Cia,
    pub rtc: &'a mut RtcMsm6242b,
}

impl<'a> MemoryBus<'a> {
    /// Physical memory map constants decoded by Gary logic
    pub const BANK_RTC: u8 = 0xDC;

    pub const RTC_START: u32 = 0xDC0000;
    pub const RTC_END: u32 = 0xDC003F;

    const BIT_CIAA_PA_OVL: u8 = 0x01;

    /// Writes an arbitrary byte slice directly to physical memory for debugger/test injection
    #[inline(always)]
    pub fn write_bytes_debug(
        &mut self,
        addr: u32,
        data: &[u8],
    ) -> Result<usize, physical_memory::KickstartSizeError> {
        self.mem.write_bytes_debug(addr, data)
    }
}

impl<'a> AddressBus for MemoryBus<'a> {
    #[inline(always)]
    fn read_byte(&mut self, addr: u32) -> BusResult<u8> {
        let addr = addr & physical_memory::MASK_PHYSICAL_ADDRESS;
        let bank = (addr >> 16) as u8;
        let (result, shared_bus_wait) = match bank {
            CUSTOM_BANK_FIRST..=CUSTOM_BANK_LAST => {
                if self.mem.chip_ram_blocked {
                    (BusResult::WaitState, true)
                } else {
                    let word = self
                        .read_custom_word((addr & config::MASK_CUSTOM_REG_OFFSET as u32) as u16);
                    let byte = if (addr & 1) == 0 {
                        (word >> 8) as u8
                    } else {
                        (word & 0xFF) as u8
                    };
                    (BusResult::Ready(byte), false)
                }
            }
            CIA_BANK_FIRST..=CIA_BANK_LAST => (self.read_cia_byte(addr), false),
            Self::BANK_RTC => {
                let byte = if (Self::RTC_START..=Self::RTC_END).contains(&addr) {
                    self.rtc.read_byte(addr)
                } else {
                    OPEN_BUS_BYTE
                };
                (BusResult::Ready(byte), false)
            }
            _ => {
                // Storage handlers enforce contention only for Chip/Slow RAM.
                let result = self.mem.read_byte(addr);
                (result, result.is_wait())
            }
        };
        self.agnus.dma.receive_cpu_bus_wait(shared_bus_wait);
        result
    }

    #[inline(always)]
    fn read_word(&mut self, addr: u32) -> BusResult<u16> {
        let addr = addr & physical_memory::MASK_PHYSICAL_ADDRESS;
        let bank = (addr >> 16) as u8;
        let (result, shared_bus_wait) = match bank {
            CUSTOM_BANK_FIRST..=CUSTOM_BANK_LAST => {
                if self.mem.chip_ram_blocked {
                    (BusResult::WaitState, true)
                } else {
                    (
                        BusResult::Ready(self.read_custom_word(
                            (addr & config::MASK_CUSTOM_REG_OFFSET as u32) as u16,
                        )),
                        false,
                    )
                }
            }
            CIA_BANK_FIRST..=CIA_BANK_LAST => (self.read_cia_word(addr), false),
            Self::BANK_RTC => {
                let b0 = if (Self::RTC_START..=Self::RTC_END).contains(&addr) {
                    self.rtc.read_byte(addr)
                } else {
                    OPEN_BUS_BYTE
                };
                let next_addr = addr.wrapping_add(1);
                let b1 = if (Self::RTC_START..=Self::RTC_END).contains(&next_addr) {
                    self.rtc.read_byte(next_addr)
                } else {
                    OPEN_BUS_BYTE
                };
                (BusResult::Ready(u16::from_be_bytes([b0, b1])), false)
            }
            _ => {
                let result = self.mem.read_word(addr);
                (result, result.is_wait())
            }
        };
        self.agnus.dma.receive_cpu_bus_wait(shared_bus_wait);
        result
    }

    #[inline(always)]
    fn write_byte(&mut self, addr: u32, val: u8) -> BusResult<()> {
        let addr = addr & physical_memory::MASK_PHYSICAL_ADDRESS;
        let bank = (addr >> 16) as u8;
        let (result, shared_bus_wait) = match bank {
            CUSTOM_BANK_FIRST..=CUSTOM_BANK_LAST => {
                let waiting = self.mem.chip_ram_blocked;
                // Queue backpressure stalls the CPU without requesting an Agnus slot.
                let result = if waiting || !self.write_custom_byte(addr, val) {
                    BusResult::WaitState
                } else {
                    BusResult::Ready(())
                };
                (result, waiting)
            }
            CIA_BANK_FIRST..=CIA_BANK_LAST => (self.write_cia_byte(addr, val), false),
            Self::BANK_RTC => {
                if (Self::RTC_START..=Self::RTC_END).contains(&addr) {
                    self.rtc.write_byte(addr, val);
                }
                (BusResult::Ready(()), false)
            }
            _ => {
                let result = self.mem.write_byte(addr, val);
                (result, result.is_wait())
            }
        };
        self.agnus.dma.receive_cpu_bus_wait(shared_bus_wait);
        result
    }

    #[inline]
    fn write_tas_byte(&mut self, addr: u32, val: u8) -> BusResult<()> {
        let addr = addr & physical_memory::MASK_PHYSICAL_ADDRESS;
        let bank = self.mem.bank_map[(addr >> 16) as usize].bank;
        if matches!(bank, MemoryBank::ChipRam | MemoryBank::SlowRam) {
            let waiting = self.mem.chip_ram_blocked;
            self.agnus.dma.receive_cpu_bus_wait(waiting);
            if waiting {
                BusResult::WaitState
            } else {
                BusResult::Ready(())
            }
        } else {
            self.write_byte(addr, val)
        }
    }

    #[inline(always)]
    fn write_word(&mut self, addr: u32, val: u16) -> BusResult<()> {
        let addr = addr & physical_memory::MASK_PHYSICAL_ADDRESS;
        let bank = (addr >> 16) as u8;
        let (result, shared_bus_wait) = match bank {
            CUSTOM_BANK_FIRST..=CUSTOM_BANK_LAST => {
                let offset = (addr & config::MASK_CUSTOM_REG_OFFSET as u32) as u16;
                let waiting = self.mem.chip_ram_blocked;
                let result = if waiting || !self.write_custom_word(offset, val) {
                    BusResult::WaitState
                } else {
                    BusResult::Ready(())
                };
                (result, waiting)
            }
            CIA_BANK_FIRST..=CIA_BANK_LAST => (self.write_cia_word(addr, val), false),
            Self::BANK_RTC => {
                let bytes = val.to_be_bytes();
                if (Self::RTC_START..=Self::RTC_END).contains(&addr) {
                    self.rtc.write_byte(addr, bytes[0]);
                }
                let next_addr = addr.wrapping_add(1);
                if (Self::RTC_START..=Self::RTC_END).contains(&next_addr) {
                    self.rtc.write_byte(next_addr, bytes[1]);
                }
                (BusResult::Ready(()), false)
            }
            _ => {
                let result = self.mem.write_word(addr, val);
                (result, result.is_wait())
            }
        };
        self.agnus.dma.receive_cpu_bus_wait(shared_bus_wait);
        result
    }

    #[inline(always)]
    fn read_word_debug(&self, addr: u32) -> u16 {
        let addr = addr & physical_memory::MASK_PHYSICAL_ADDRESS;
        let bank = (addr >> 16) as u8;
        match bank {
            CUSTOM_BANK_FIRST..=CUSTOM_BANK_LAST => {
                self.read_custom_word_debug((addr & config::MASK_CUSTOM_REG_OFFSET as u32) as u16)
            }
            CIA_BANK_FIRST..=CIA_BANK_LAST => self.read_cia_word_debug(addr),
            Self::BANK_RTC => {
                let b0 = if (Self::RTC_START..=Self::RTC_END).contains(&addr) {
                    self.rtc.read_byte(addr)
                } else {
                    OPEN_BUS_BYTE
                };
                let next_addr = addr.wrapping_add(1);
                let b1 = if (Self::RTC_START..=Self::RTC_END).contains(&next_addr) {
                    self.rtc.read_byte(next_addr)
                } else {
                    OPEN_BUS_BYTE
                };
                u16::from_be_bytes([b0, b1])
            }
            _ => self.mem.read_word_debug(addr),
        }
    }
}
