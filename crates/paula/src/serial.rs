//! Paula UART holding/shift engines; the production connector remains disconnected.

use crate::register_bits::intreq;
use serde::{Deserialize, Serialize};

const BIT_OVERRUN: u16 = 0x8000;
const BIT_RBF: u16 = 0x4000;
const BIT_TBE: u16 = 0x2000;
const BIT_TSRE: u16 = 0x1000;
const BIT_RXD: u16 = 0x0800;
const MASK_RECEIVED_WORD: u16 = 0x03ff;
const BIT_LONG_RECEIVE: u16 = 0x8000;
const MASK_PERIOD: u16 = 0x7fff;
const SHORT_DATA_BITS: u8 = 8;
const LONG_DATA_BITS: u8 = 9;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
enum ReceivePhase {
    #[default]
    Idle,
    Start,
    Data,
    Stop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SerialPort {
    pub serdatr: u16,
    pub serdat: u16,
    pub serper: u16,
    tx_holding: u16,
    tx_shift: u32,
    tx_counter: u16,
    tx_bit_high: bool,
    rx_previous_high: bool,
    rx_phase: ReceivePhase,
    rx_counter: u16,
    rx_shift: u16,
    rx_bit: u8,
    rx_bits: u8,
}

impl Default for SerialPort {
    fn default() -> Self {
        Self::new()
    }
}

impl SerialPort {
    pub fn new() -> Self {
        Self {
            serdatr: BIT_TBE | BIT_TSRE | BIT_RXD,
            serdat: 0,
            serper: 0,
            tx_holding: 0,
            tx_shift: 0,
            tx_counter: 0,
            tx_bit_high: true,
            rx_previous_high: true,
            rx_phase: ReceivePhase::Idle,
            rx_counter: 0,
            rx_shift: 0,
            rx_bit: 0,
            rx_bits: SHORT_DATA_BITS,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn write_serdat(&mut self, value: u16) {
        self.serdat = value;
        self.tx_holding = value;
        if value == 0 {
            self.serdatr |= BIT_TBE;
        } else {
            self.serdatr &= !BIT_TBE;
        }
    }

    pub fn write_serper(&mut self, value: u16) {
        self.serper = value;
    }

    fn period(&self) -> u16 {
        (self.serper & MASK_PERIOD) + 1
    }

    pub fn transmit_high(&self, force_break: bool) -> bool {
        self.tx_bit_high && !force_break
    }

    pub fn read_status(&self, receive_full: bool) -> u16 {
        (self.serdatr & !BIT_RBF) | if receive_full { BIT_RBF } else { 0 }
    }

    pub fn acknowledge_receive(&mut self) {
        self.serdatr &= !BIT_OVERRUN;
    }

    fn start_transmit(&mut self) -> u16 {
        if self.tx_holding == 0 {
            self.tx_bit_high = true;
            self.serdatr |= BIT_TSRE;
            return 0;
        }
        self.tx_shift = u32::from(self.tx_holding) << 1;
        self.tx_holding = 0;
        self.tx_counter = self.period();
        self.tx_bit_high = false;
        self.serdatr = (self.serdatr | BIT_TBE) & !BIT_TSRE;
        intreq::BIT_TBE
    }

    fn step_transmit(&mut self) -> u16 {
        if self.tx_shift == 0 {
            return self.start_transmit();
        }
        self.tx_counter = self.tx_counter.wrapping_sub(1);
        if self.tx_counter != 0 {
            return 0;
        }
        self.tx_shift >>= 1;
        if self.tx_shift == 0 {
            return self.start_transmit();
        }
        self.tx_bit_high = self.tx_shift & 1 != 0;
        self.tx_counter = self.period();
        0
    }

    fn step_receive(&mut self, high: bool, full: bool) -> u16 {
        if self.rx_phase == ReceivePhase::Idle {
            if self.rx_previous_high && !high {
                self.rx_bits = if self.serper & BIT_LONG_RECEIVE == 0 {
                    SHORT_DATA_BITS
                } else {
                    LONG_DATA_BITS
                };
                self.rx_shift = 0;
                self.rx_bit = 0;
                if self.period() == 1 {
                    self.rx_phase = ReceivePhase::Data;
                    self.rx_counter = 1;
                } else {
                    self.rx_phase = ReceivePhase::Start;
                    self.rx_counter = self.period() / 2;
                }
            }
            return 0;
        }
        self.rx_counter = self.rx_counter.wrapping_sub(1);
        if self.rx_counter != 0 {
            return 0;
        }
        self.rx_counter = self.period();
        match self.rx_phase {
            ReceivePhase::Idle => {}
            ReceivePhase::Start => {
                self.rx_phase = if high {
                    ReceivePhase::Idle
                } else {
                    ReceivePhase::Data
                };
            }
            ReceivePhase::Data => {
                self.rx_shift |= u16::from(high) << self.rx_bit;
                self.rx_bit += 1;
                if self.rx_bit == self.rx_bits {
                    self.rx_phase = ReceivePhase::Stop;
                }
            }
            ReceivePhase::Stop => {
                self.rx_shift |= u16::from(high) << self.rx_bits;
                self.serdatr = (self.serdatr & !(MASK_RECEIVED_WORD | BIT_OVERRUN))
                    | self.rx_shift
                    | if full { BIT_OVERRUN } else { 0 };
                self.rx_phase = ReceivePhase::Idle;
                return intreq::BIT_RBF;
            }
        }
        0
    }

    /// Returns interrupt pulses. Production calls this with RXD high; component
    /// tests can provide explicit input levels without an attached connector.
    pub fn step_cck(&mut self, receive_high: bool, receive_full: bool) -> u16 {
        self.serdatr = (self.serdatr & !BIT_RXD) | if receive_high { BIT_RXD } else { 0 };
        let requests = self.step_transmit() | self.step_receive(receive_high, receive_full);
        self.rx_previous_high = receive_high;
        requests
    }
}
