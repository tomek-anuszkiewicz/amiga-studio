//! Internal Paula audio, disk, serial and interrupt register fields.

pub mod audio_disk_control {
    pub const BIT_SET_CLR: u16 = 0x8000;
    pub const MASK_WRITE: u16 = 0x7FFF;
    pub const BIT_VOLUME_MODULATION_FIRST: u16 = 1;
    pub const BIT_PERIOD_MODULATION_FIRST: u16 = 1 << 4;
    pub const BIT_UART_BREAK: u16 = 0x0800;
}

pub mod dskbytr {
    pub const BIT_DSKBYTR_DSKBYT: u16 = 0x8000;
    pub const BIT_DSKBYTR_DMAON: u16 = 0x4000;
    pub const BIT_DSKBYTR_DISKWRITE: u16 = 0x2000;
    pub const BIT_DSKBYTR_WORDEQUAL: u16 = 0x1000;
    pub const MASK_DSKBYTR_DATA: u16 = 0x90FF;
}

pub mod intreq {
    /// Bit 15: Set/Clear control bit
    pub const BIT_SET_CLR: u16 = 0x8000;
    /// Bit 14: Master interrupt enable (INTEN)
    pub const BIT_INTEN: u16 = 0x4000;
    /// Bit 13: External interrupt (Level 6)
    pub const BIT_EXTER: u16 = 0x2000;
    /// Bit 12: Disk sync match (Level 5)
    pub const BIT_DSKSYN: u16 = 0x1000;
    /// Bit 11: Serial receive buffer full (Level 5)
    pub const BIT_RBF: u16 = 0x0800;
    /// Bit 10: Audio channel 3 finished (Level 4)
    pub const BIT_AUD3: u16 = 0x0400;
    /// Bit 9: Audio channel 2 finished (Level 4)
    pub const BIT_AUD2: u16 = 0x0200;
    /// Bit 8: Audio channel 1 finished (Level 4)
    pub const BIT_AUD1: u16 = 0x0100;
    /// Bit 7: Audio channel 0 finished (Level 4)
    pub const BIT_AUD0: u16 = 0x0080;
    /// Bit 6: Blitter finished (Level 3)
    pub const BIT_BLIT: u16 = 0x0040;
    /// Bit 5: Vertical blank interval start (Level 3)
    pub const BIT_VERTB: u16 = 0x0020;
    /// Bit 4: Copper instruction finished / trap (Level 3)
    pub const BIT_COPER: u16 = 0x0010;
    /// Bit 3: I/O Ports and timers (CIA-A, Level 2)
    pub const BIT_PORTS: u16 = 0x0008;
    /// Bit 2: Software interrupt (Level 1)
    pub const BIT_SOFT: u16 = 0x0004;
    /// Bit 1: Disk block transfer finished (Level 1)
    pub const BIT_DSKBLK: u16 = 0x0002;
    /// Bit 0: Serial transmit buffer empty (Level 1)
    pub const BIT_TBE: u16 = 0x0001;
}
