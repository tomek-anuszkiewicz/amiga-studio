//! HRM chapter 5 channel states, separate data/output latches and counters.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioState {
    #[default]
    Idle,
    FirstWord,
    SecondWord,
    HighByte,
    LowByte,
}

pub enum Attachment {
    None,
    Volume(u16),
    Period(u16),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioChannel {
    pub len: u16,
    pub per: u16,
    pub vol: u8,
    pub dat: u16,
    pub buffer: u16,
    pub words_remaining: u16,
    pub counter: u16,
    pub current_sample: i8,
    pub output_volume: u8,
    pub state: AudioState,
    pub dma_enabled: bool,
    pub dma_request: bool,
    pub restart_request: bool,
    pub irq_pending: bool,
    /// Length completion waits for the relevant sample/attachment transition.
    pub loop_irq: bool,
}

impl AudioChannel {
    pub fn set_dma_enabled(&mut self, enabled: bool) {
        self.dma_enabled = enabled;
        if enabled && self.state == AudioState::Idle {
            self.state = AudioState::FirstWord;
            self.words_remaining = self.len;
            self.dma_request = true;
            self.restart_request = false;
        } else if !enabled {
            self.dma_request = false;
            self.restart_request = false;
            if matches!(self.state, AudioState::FirstWord | AudioState::SecondWord) {
                self.state = AudioState::Idle;
            }
        }
    }

    pub(super) fn load_word(
        &mut self,
        word: u16,
        interrupt: bool,
        volume: bool,
        period: bool,
    ) -> Attachment {
        self.dat = word;
        self.dma_request = false;
        self.restart_request = false;
        if self.dma_enabled {
            match self.state {
                AudioState::Idle => self.set_dma_enabled(true),
                AudioState::FirstWord => {
                    self.state = AudioState::SecondWord;
                    self.dma_request = true;
                    self.restart_request = true;
                    self.irq_pending = true;
                    if self.words_remaining != 1 {
                        self.words_remaining = self.words_remaining.wrapping_sub(1);
                    }
                }
                AudioState::SecondWord => {
                    self.dma_request = !period || volume;
                    return self.begin_high(volume);
                }
                AudioState::HighByte | AudioState::LowByte => {
                    if self.words_remaining == 1 {
                        self.words_remaining = self.len;
                        self.restart_request = true;
                        self.loop_irq = true;
                    } else {
                        self.words_remaining = self.words_remaining.wrapping_sub(1);
                    }
                }
            }
        } else if self.state == AudioState::Idle && !interrupt {
            self.irq_pending = true;
            return self.begin_high(volume);
        }
        Attachment::None
    }

    fn begin_high(&mut self, volume: bool) -> Attachment {
        self.state = AudioState::HighByte;
        self.counter = self.per;
        self.output_volume = self.vol;
        if volume {
            Attachment::Volume(self.dat)
        } else {
            self.buffer = self.dat;
            self.current_sample = (self.buffer >> u8::BITS) as i8;
            Attachment::None
        }
    }

    fn request_next(&mut self) {
        if self.dma_enabled {
            self.dma_request = true;
            if self.loop_irq {
                self.irq_pending = true;
                self.loop_irq = false;
            }
        } else {
            self.irq_pending = true;
        }
    }

    pub(super) fn step(&mut self, interrupt: bool, volume: bool, period: bool) -> Attachment {
        if !matches!(self.state, AudioState::HighByte | AudioState::LowByte) {
            return Attachment::None;
        }
        // Zero represents the full 65536-CCK period through wrapping arithmetic.
        self.counter = self.counter.wrapping_sub(1);
        if self.counter != 0 {
            return Attachment::None;
        }
        self.counter = self.per;
        if self.state == AudioState::HighByte {
            self.state = AudioState::LowByte;
            self.current_sample = self.buffer as i8;
            if period {
                self.request_next();
                return Attachment::Period(self.dat);
            }
        } else if !self.dma_enabled && interrupt {
            self.state = AudioState::Idle;
            self.loop_irq = false;
        } else {
            if !period || volume {
                self.request_next();
            }
            return self.begin_high(volume);
        }
        Attachment::None
    }

    #[inline]
    pub fn output_level(&self) -> i16 {
        i16::from(self.current_sample) * i16::from(self.output_volume)
    }
}
