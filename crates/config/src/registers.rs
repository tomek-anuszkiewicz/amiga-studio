//! Amiga 500 register addresses and fields shared across chip boundaries
//!
//! Custom chip offsets follow the Commodore-Amiga Hardware Reference Manual (HRM),
//! Appendix A. CIA byte addresses follow Appendices D and F.

/// Word-aligned RGA register selection within the custom chip window.
pub const MASK_CUSTOM_REG_OFFSET: u16 = 0x01FE;
pub const AUDIO_CHANNEL_COUNT: usize = 4;
pub const OCS_BITPLANE_COUNT: usize = 6;
pub const SPRITE_COUNT: usize = 8;
pub const AUDIO_REGISTER_STRIDE: u16 = 0x10;
pub const BITPLANE_POINTER_STRIDE: u16 = 4;
pub const SPRITE_POINTER_STRIDE: u16 = 4;
pub const SPRITE_REGISTER_STRIDE: u16 = 8;
pub const BIT_POINTER_LOW_REGISTER: u16 = 2;

/// Vertical display-window encoding shared by Agnus and frame composition.
pub mod display_window {
    pub const BIT_VSTOP_EXTENSION_SOURCE: u16 = 0x8000;
    pub const BIT_VSTOP_EXTENSION: u16 = 0x0100;
}

/// Custom chip register offsets in the $DFF000 - $DFFFFE range.
///
/// Register offsets are word-aligned ($000 to $1FE).
/// Registers are organized by physical custom chip (Agnus, Denise, Paula),
/// with shared offsets declared in each chip namespace that decodes them.
pub mod custom_reg {
    /// Agnus (MOS 8370 / 8371 / 8372) Address Generator & DMA Coprocessor Registers
    pub mod agnus {
        /// Blitter destination early read (pseudo-register)
        pub const REGISTER_BLTDDAT: u16 = 0x000;
        /// DMA control read (channel status and blitter busy/zero flags)
        pub const REGISTER_DMACONR: u16 = 0x002;
        /// Vertical beam position high bit, chip revision ID, and LOF
        pub const REGISTER_VPOSR: u16 = 0x004;
        /// Vertical and horizontal beam position read (raster beam position)
        pub const REGISTER_VHPOSR: u16 = 0x006;
        /// Disk track buffer pointer high
        pub const REGISTER_DSKPTH: u16 = 0x020;
        /// Disk track buffer pointer low
        pub const REGISTER_DSKPTL: u16 = 0x022;
        /// DRAM refresh pointer (internal test)
        pub const REGISTER_REFPTR: u16 = 0x028;
        /// Vertical position write (high bits)
        pub const REGISTER_VPOSW: u16 = 0x02A;
        /// Vertical and horizontal position write
        pub const REGISTER_VHPOSW: u16 = 0x02C;
        /// Copper control register (CDANG bit enables blitter register writes)
        pub const REGISTER_COPCON: u16 = 0x02E;
        /// Blitter control register 0 (channels, minterms, A-shift)
        pub const REGISTER_BLTCON0: u16 = 0x040;
        /// Blitter control register 1 (B-shift, line mode, fill, desc)
        pub const REGISTER_BLTCON1: u16 = 0x042;
        /// Blitter first word mask for channel A
        pub const REGISTER_BLTAFWM: u16 = 0x044;
        /// Blitter last word mask for channel A
        pub const REGISTER_BLTALWM: u16 = 0x046;
        /// Blitter channel C pointer high
        pub const REGISTER_BLTCPTH: u16 = 0x048;
        /// Blitter channel C pointer low
        pub const REGISTER_BLTCPTL: u16 = 0x04A;
        /// Blitter channel B pointer high
        pub const REGISTER_BLTBPTH: u16 = 0x04C;
        /// Blitter channel B pointer low
        pub const REGISTER_BLTBPTL: u16 = 0x04E;
        /// Blitter channel A pointer high
        pub const REGISTER_BLTAPTH: u16 = 0x050;
        /// Blitter channel A pointer low
        pub const REGISTER_BLTAPTL: u16 = 0x052;
        /// Blitter channel D pointer high
        pub const REGISTER_BLTDPTH: u16 = 0x054;
        /// Blitter channel D pointer low
        pub const REGISTER_BLTDPTL: u16 = 0x056;
        /// Blitter start & window size (rows and word width)
        pub const REGISTER_BLTSIZE: u16 = 0x058;
        /// Blitter channel C modulo
        pub const REGISTER_BLTCMOD: u16 = 0x060;
        /// Blitter channel B modulo
        pub const REGISTER_BLTBMOD: u16 = 0x062;
        /// Blitter channel A modulo
        pub const REGISTER_BLTAMOD: u16 = 0x064;
        /// Blitter channel D modulo
        pub const REGISTER_BLTDMOD: u16 = 0x066;
        /// Blitter channel C data latch
        pub const REGISTER_BLTCDAT: u16 = 0x070;
        /// Blitter channel B data latch
        pub const REGISTER_BLTBDAT: u16 = 0x072;
        /// Blitter channel A data latch
        pub const REGISTER_BLTADAT: u16 = 0x074;
        /// Copper list 1 location high (bits 18..16)
        pub const REGISTER_COP1LCH: u16 = 0x080;
        /// Copper list 1 location low (bits 15..1)
        pub const REGISTER_COP1LCL: u16 = 0x082;
        /// Copper list 2 location high (bits 18..16)
        pub const REGISTER_COP2LCH: u16 = 0x084;
        /// Copper list 2 location low (bits 15..1)
        pub const REGISTER_COP2LCL: u16 = 0x086;
        /// Copper restart list 1 strobe
        pub const REGISTER_COPJMP1: u16 = 0x088;
        /// Copper restart list 2 strobe
        pub const REGISTER_COPJMP2: u16 = 0x08A;
        /// Copper instruction fetch strobe
        pub const REGISTER_COPINS: u16 = 0x08C;
        /// Display window start (vertical DMA boundary; horizontal bits also latched by Denise)
        pub const REGISTER_DIWSTRT: u16 = 0x08E;
        /// Display window stop (vertical DMA boundary; horizontal bits also latched by Denise)
        pub const REGISTER_DIWSTOP: u16 = 0x090;
        /// Display data fetch start (horizontal bitplane DMA window start)
        pub const REGISTER_DDFSTRT: u16 = 0x092;
        /// Display data fetch stop (horizontal bitplane DMA window stop)
        pub const REGISTER_DDFSTOP: u16 = 0x094;
        /// DMA control write (set/clear bits for DMA channels)
        pub const REGISTER_DMACON: u16 = 0x096;
        /// Audio channel 0 pointer high (Location High)
        pub const REGISTER_AUD0LCH: u16 = 0x0A0;
        /// Audio channel 0 pointer low (Location Low)
        pub const REGISTER_AUD0LCL: u16 = 0x0A2;
        /// Audio channel 1 pointer high (Location High)
        pub const REGISTER_AUD1LCH: u16 = 0x0B0;
        /// Audio channel 1 pointer low (Location Low)
        pub const REGISTER_AUD1LCL: u16 = 0x0B2;
        /// Audio channel 2 pointer high (Location High)
        pub const REGISTER_AUD2LCH: u16 = 0x0C0;
        /// Audio channel 2 pointer low (Location Low)
        pub const REGISTER_AUD2LCL: u16 = 0x0C2;
        /// Audio channel 3 pointer high (Location High)
        pub const REGISTER_AUD3LCH: u16 = 0x0D0;
        /// Audio channel 3 pointer low (Location Low)
        pub const REGISTER_AUD3LCL: u16 = 0x0D2;
        /// Bitplane 1 pointer high
        pub const REGISTER_BPL1PTH: u16 = 0x0E0;
        /// Bitplane 1 pointer low
        pub const REGISTER_BPL1PTL: u16 = 0x0E2;
        /// Bitplane 2 pointer high
        pub const REGISTER_BPL2PTH: u16 = 0x0E4;
        /// Bitplane 2 pointer low
        pub const REGISTER_BPL2PTL: u16 = 0x0E6;
        /// Bitplane 3 pointer high
        pub const REGISTER_BPL3PTH: u16 = 0x0E8;
        /// Bitplane 3 pointer low
        pub const REGISTER_BPL3PTL: u16 = 0x0EA;
        /// Bitplane 4 pointer high
        pub const REGISTER_BPL4PTH: u16 = 0x0EC;
        /// Bitplane 4 pointer low
        pub const REGISTER_BPL4PTL: u16 = 0x0EE;
        /// Bitplane 5 pointer high
        pub const REGISTER_BPL5PTH: u16 = 0x0F0;
        /// Bitplane 5 pointer low
        pub const REGISTER_BPL5PTL: u16 = 0x0F2;
        /// Bitplane 6 pointer high
        pub const REGISTER_BPL6PTH: u16 = 0x0F4;
        /// Bitplane 6 pointer low
        pub const REGISTER_BPL6PTL: u16 = 0x0F6;
        /// Bitplane control 0 (hires, bitplane count, interlace, color)
        pub const REGISTER_BPLCON0: u16 = 0x100;
        /// Bitplane modulo 1 (odd bitplanes 1, 3, 5)
        pub const REGISTER_BPL1MOD: u16 = 0x108;
        /// Bitplane modulo 2 (even bitplanes 2, 4, 6)
        pub const REGISTER_BPL2MOD: u16 = 0x10A;
        /// Sprite 0 pointer high
        pub const REGISTER_SPR0PTH: u16 = 0x120;
        /// Sprite 0 pointer low
        pub const REGISTER_SPR0PTL: u16 = 0x122;
        /// Sprite 1 pointer high
        pub const REGISTER_SPR1PTH: u16 = 0x124;
        /// Sprite 1 pointer low
        pub const REGISTER_SPR1PTL: u16 = 0x126;
        /// Sprite 2 pointer high
        pub const REGISTER_SPR2PTH: u16 = 0x128;
        /// Sprite 2 pointer low
        pub const REGISTER_SPR2PTL: u16 = 0x12A;
        /// Sprite 3 pointer high
        pub const REGISTER_SPR3PTH: u16 = 0x12C;
        /// Sprite 3 pointer low
        pub const REGISTER_SPR3PTL: u16 = 0x12E;
        /// Sprite 4 pointer high
        pub const REGISTER_SPR4PTH: u16 = 0x130;
        /// Sprite 4 pointer low
        pub const REGISTER_SPR4PTL: u16 = 0x132;
        /// Sprite 5 pointer high
        pub const REGISTER_SPR5PTH: u16 = 0x134;
        /// Sprite 5 pointer low
        pub const REGISTER_SPR5PTL: u16 = 0x136;
        /// Sprite 6 pointer high
        pub const REGISTER_SPR6PTH: u16 = 0x138;
        /// Sprite 6 pointer low
        pub const REGISTER_SPR6PTL: u16 = 0x13A;
        /// Sprite 7 pointer high
        pub const REGISTER_SPR7PTH: u16 = 0x13C;
        /// Sprite 7 pointer low
        pub const REGISTER_SPR7PTL: u16 = 0x13E;
        /// Sprite position registers also sampled by Agnus
        pub const REGISTER_SPR0POS: u16 = 0x140;
        pub const REGISTER_SPR0CTL: u16 = 0x142;
        pub const REGISTER_SPR1POS: u16 = 0x148;
        pub const REGISTER_SPR1CTL: u16 = 0x14A;
        pub const REGISTER_SPR2POS: u16 = 0x150;
        pub const REGISTER_SPR2CTL: u16 = 0x152;
        pub const REGISTER_SPR3POS: u16 = 0x158;
        pub const REGISTER_SPR3CTL: u16 = 0x15A;
        pub const REGISTER_SPR4POS: u16 = 0x160;
        pub const REGISTER_SPR4CTL: u16 = 0x162;
        pub const REGISTER_SPR5POS: u16 = 0x168;
        pub const REGISTER_SPR5CTL: u16 = 0x16A;
        pub const REGISTER_SPR6POS: u16 = 0x170;
        pub const REGISTER_SPR6CTL: u16 = 0x172;
        pub const REGISTER_SPR7POS: u16 = 0x178;
        pub const REGISTER_SPR7CTL: u16 = 0x17A;
    }

    /// Denise (MOS 8362 / 8373) Video Display & Serializer Registers
    pub mod denise {
        /// Joystick/mouse 0 counter data
        pub const REGISTER_JOY0DAT: u16 = 0x00A;
        /// Joystick/mouse 1 counter data
        pub const REGISTER_JOY1DAT: u16 = 0x00C;
        /// Collision detection data read
        pub const REGISTER_CLXDAT: u16 = 0x00E;
        /// Joy/mouse test register
        pub const REGISTER_JOYTEST: u16 = 0x036;
        /// Horizontal sync with vertical blank and equalization strobe
        pub const REGISTER_STREQU: u16 = 0x038;
        /// Horizontal sync with vertical blank strobe
        pub const REGISTER_STRVBL: u16 = 0x03A;
        /// Horizontal sync strobe
        pub const REGISTER_STRHOR: u16 = 0x03C;
        /// Long horizontal line identification strobe
        pub const REGISTER_STRLONG: u16 = 0x03E;
        /// Display window start (upper-left coordinates)
        pub const REGISTER_DIWSTRT: u16 = 0x08E;
        /// Display window stop (lower-right coordinates)
        pub const REGISTER_DIWSTOP: u16 = 0x090;
        /// Collision control
        pub const REGISTER_CLXCON: u16 = 0x098;
        /// Bitplane control 0 (hires, bitplane count, interlace, color)
        pub const REGISTER_BPLCON0: u16 = 0x100;
        /// Bitplane control 1 (horizontal scroll deltas)
        pub const REGISTER_BPLCON1: u16 = 0x102;
        /// Bitplane control 2 (playfield/sprite priority)
        pub const REGISTER_BPLCON2: u16 = 0x104;
        /// Bitplane 1 data buffer
        pub const REGISTER_BPL1DAT: u16 = 0x110;
        /// Bitplane 2 data buffer
        pub const REGISTER_BPL2DAT: u16 = 0x112;
        /// Bitplane 3 data buffer
        pub const REGISTER_BPL3DAT: u16 = 0x114;
        /// Bitplane 4 data buffer
        pub const REGISTER_BPL4DAT: u16 = 0x116;
        /// Bitplane 5 data buffer
        pub const REGISTER_BPL5DAT: u16 = 0x118;
        /// Bitplane 6 data buffer
        pub const REGISTER_BPL6DAT: u16 = 0x11A;
        /// Sprite 0 position data
        pub const REGISTER_SPR0POS: u16 = 0x140;
        /// Sprite 0 control data
        pub const REGISTER_SPR0CTL: u16 = 0x142;
        /// Sprite 0 image data A
        pub const REGISTER_SPR0DATA: u16 = 0x144;
        /// Sprite 0 image data B
        pub const REGISTER_SPR0DATB: u16 = 0x146;
        /// Sprite 1 position data
        pub const REGISTER_SPR1POS: u16 = 0x148;
        /// Sprite 1 control data
        pub const REGISTER_SPR1CTL: u16 = 0x14A;
        /// Sprite 1 image data A
        pub const REGISTER_SPR1DATA: u16 = 0x14C;
        /// Sprite 1 image data B
        pub const REGISTER_SPR1DATB: u16 = 0x14E;
        /// Sprite 2 position data
        pub const REGISTER_SPR2POS: u16 = 0x150;
        /// Sprite 2 control data
        pub const REGISTER_SPR2CTL: u16 = 0x152;
        /// Sprite 2 image data A
        pub const REGISTER_SPR2DATA: u16 = 0x154;
        /// Sprite 2 image data B
        pub const REGISTER_SPR2DATB: u16 = 0x156;
        /// Sprite 3 position data
        pub const REGISTER_SPR3POS: u16 = 0x158;
        /// Sprite 3 control data
        pub const REGISTER_SPR3CTL: u16 = 0x15A;
        /// Sprite 3 image data A
        pub const REGISTER_SPR3DATA: u16 = 0x15C;
        /// Sprite 3 image data B
        pub const REGISTER_SPR3DATB: u16 = 0x15E;
        /// Sprite 4 position data
        pub const REGISTER_SPR4POS: u16 = 0x160;
        /// Sprite 4 control data
        pub const REGISTER_SPR4CTL: u16 = 0x162;
        /// Sprite 4 image data A
        pub const REGISTER_SPR4DATA: u16 = 0x164;
        /// Sprite 4 image data B
        pub const REGISTER_SPR4DATB: u16 = 0x166;
        /// Sprite 5 position data
        pub const REGISTER_SPR5POS: u16 = 0x168;
        /// Sprite 5 control data
        pub const REGISTER_SPR5CTL: u16 = 0x16A;
        /// Sprite 5 image data A
        pub const REGISTER_SPR5DATA: u16 = 0x16C;
        /// Sprite 5 image data B
        pub const REGISTER_SPR5DATB: u16 = 0x16E;
        /// Sprite 6 position data
        pub const REGISTER_SPR6POS: u16 = 0x170;
        /// Sprite 6 control data
        pub const REGISTER_SPR6CTL: u16 = 0x172;
        /// Sprite 6 image data A
        pub const REGISTER_SPR6DATA: u16 = 0x174;
        /// Sprite 6 image data B
        pub const REGISTER_SPR6DATB: u16 = 0x176;
        /// Sprite 7 position data
        pub const REGISTER_SPR7POS: u16 = 0x178;
        /// Sprite 7 control data
        pub const REGISTER_SPR7CTL: u16 = 0x17A;
        /// Sprite 7 image data A
        pub const REGISTER_SPR7DATA: u16 = 0x17C;
        /// Sprite 7 image data B
        pub const REGISTER_SPR7DATB: u16 = 0x17E;
        /// Color register 00 (background color)
        pub const REGISTER_COLOR00: u16 = 0x180;
        /// Color register 01
        pub const REGISTER_COLOR01: u16 = 0x182;
        /// Color register 02
        pub const REGISTER_COLOR02: u16 = 0x184;
        /// Color register 03
        pub const REGISTER_COLOR03: u16 = 0x186;
        /// Color register 04
        pub const REGISTER_COLOR04: u16 = 0x188;
        /// Color register 05
        pub const REGISTER_COLOR05: u16 = 0x18A;
        /// Color register 06
        pub const REGISTER_COLOR06: u16 = 0x18C;
        /// Color register 07
        pub const REGISTER_COLOR07: u16 = 0x18E;
        /// Color register 08
        pub const REGISTER_COLOR08: u16 = 0x190;
        /// Color register 09
        pub const REGISTER_COLOR09: u16 = 0x192;
        /// Color register 10
        pub const REGISTER_COLOR10: u16 = 0x194;
        /// Color register 11
        pub const REGISTER_COLOR11: u16 = 0x196;
        /// Color register 12
        pub const REGISTER_COLOR12: u16 = 0x198;
        /// Color register 13
        pub const REGISTER_COLOR13: u16 = 0x19A;
        /// Color register 14
        pub const REGISTER_COLOR14: u16 = 0x19C;
        /// Color register 15
        pub const REGISTER_COLOR15: u16 = 0x19E;
        /// Color register 16
        pub const REGISTER_COLOR16: u16 = 0x1A0;
        /// Color register 17
        pub const REGISTER_COLOR17: u16 = 0x1A2;
        /// Color register 18
        pub const REGISTER_COLOR18: u16 = 0x1A4;
        /// Color register 19
        pub const REGISTER_COLOR19: u16 = 0x1A6;
        /// Color register 20
        pub const REGISTER_COLOR20: u16 = 0x1A8;
        /// Color register 21
        pub const REGISTER_COLOR21: u16 = 0x1AA;
        /// Color register 22
        pub const REGISTER_COLOR22: u16 = 0x1AC;
        /// Color register 23
        pub const REGISTER_COLOR23: u16 = 0x1AE;
        /// Color register 24
        pub const REGISTER_COLOR24: u16 = 0x1B0;
        /// Color register 25
        pub const REGISTER_COLOR25: u16 = 0x1B2;
        /// Color register 26
        pub const REGISTER_COLOR26: u16 = 0x1B4;
        /// Color register 27
        pub const REGISTER_COLOR27: u16 = 0x1B6;
        /// Color register 28
        pub const REGISTER_COLOR28: u16 = 0x1B8;
        /// Color register 29
        pub const REGISTER_COLOR29: u16 = 0x1BA;
        /// Color register 30
        pub const REGISTER_COLOR30: u16 = 0x1BC;
        /// Color register 31
        pub const REGISTER_COLOR31: u16 = 0x1BE;
    }

    /// Paula (MOS 8364) Audio, Disk, and Interrupt Controller Registers
    pub mod paula {
        /// DMA control read (channel status and blitter busy/zero flags)
        pub const REGISTER_DMACONR: u16 = 0x002;
        /// Disk DMA data read
        pub const REGISTER_DSKDATR: u16 = 0x008;
        /// Audio, disk, and UART control read
        pub const REGISTER_ADKCONR: u16 = 0x010;
        /// Potentiometer 0 data (port 1 pin 5/9)
        pub const REGISTER_POT0DAT: u16 = 0x012;
        /// Potentiometer 1 data (port 2 pin 5/9)
        pub const REGISTER_POT1DAT: u16 = 0x014;
        /// Potentiometer port data read (start pins / button 2/3)
        pub const REGISTER_POTGOR: u16 = 0x016;
        /// Serial port data and status read
        pub const REGISTER_SERDATR: u16 = 0x018;
        /// Disk data byte and status read
        pub const REGISTER_DSKBYTR: u16 = 0x01A;
        /// Interrupt enable bits read
        pub const REGISTER_INTENAR: u16 = 0x01C;
        /// Interrupt request bits read
        pub const REGISTER_INTREQR: u16 = 0x01E;
        /// Disk track buffer length
        pub const REGISTER_DSKLEN: u16 = 0x024;
        /// Disk DMA data write
        pub const REGISTER_DSKDAT: u16 = 0x026;
        /// Serial port data write
        pub const REGISTER_SERDAT: u16 = 0x030;
        /// Serial port period and control
        pub const REGISTER_SERPER: u16 = 0x032;
        /// Potentiometer port data write and start
        pub const REGISTER_POTGO: u16 = 0x034;
        /// Horizontal sync with vertical blank and equalization strobe
        pub const REGISTER_STREQU: u16 = 0x038;
        /// Horizontal sync with vertical blank strobe
        pub const REGISTER_STRVBL: u16 = 0x03A;
        /// Horizontal sync strobe shared with Denise
        pub const REGISTER_STRHOR: u16 = 0x03C;
        /// Long horizontal line identification strobe
        pub const REGISTER_STRLONG: u16 = 0x03E;
        /// Disk sync pattern register
        pub const REGISTER_DSKSYNC: u16 = 0x07E;
        /// DMA control write (set/clear bits for DMA channels)
        pub const REGISTER_DMACON: u16 = 0x096;
        /// Interrupt enable bits write
        pub const REGISTER_INTENA: u16 = 0x09A;
        /// Interrupt request bits write
        pub const REGISTER_INTREQ: u16 = 0x09C;
        /// Audio, disk, and UART control write
        pub const REGISTER_ADKCON: u16 = 0x09E;
        /// Audio channel 0 length (words)
        pub const REGISTER_AUD0LEN: u16 = 0x0A4;
        /// Audio channel 0 sample period
        pub const REGISTER_AUD0PER: u16 = 0x0A6;
        /// Audio channel 0 volume
        pub const REGISTER_AUD0VOL: u16 = 0x0A8;
        /// Audio channel 0 data word
        pub const REGISTER_AUD0DAT: u16 = 0x0AA;
        /// Audio channel 1 length (words)
        pub const REGISTER_AUD1LEN: u16 = 0x0B4;
        /// Audio channel 1 sample period
        pub const REGISTER_AUD1PER: u16 = 0x0B6;
        /// Audio channel 1 volume
        pub const REGISTER_AUD1VOL: u16 = 0x0B8;
        /// Audio channel 1 data word
        pub const REGISTER_AUD1DAT: u16 = 0x0BA;
        /// Audio channel 2 length (words)
        pub const REGISTER_AUD2LEN: u16 = 0x0C4;
        /// Audio channel 2 sample period
        pub const REGISTER_AUD2PER: u16 = 0x0C6;
        /// Audio channel 2 volume
        pub const REGISTER_AUD2VOL: u16 = 0x0C8;
        /// Audio channel 2 data word
        pub const REGISTER_AUD2DAT: u16 = 0x0CA;
        /// Audio channel 3 length (words)
        pub const REGISTER_AUD3LEN: u16 = 0x0D4;
        /// Audio channel 3 sample period
        pub const REGISTER_AUD3PER: u16 = 0x0D6;
        /// Audio channel 3 volume
        pub const REGISTER_AUD3VOL: u16 = 0x0D8;
        /// Audio channel 3 data word
        pub const REGISTER_AUD3DAT: u16 = 0x0DA;
    }
}

/// Memory-mapped byte addresses of the two MOS 8520 interface adapters.
///
/// Each chip has the same register selectors but occupies a different byte lane.
/// Selector $B is unconnected on both chips.
pub mod cia_reg {
    /// CIA-A registers on the odd-byte lane ($BFE001–$BFEF01).
    pub mod cia_a {
        /// Port A data: game-port fire, floppy status, LED, and overlay.
        pub const REGISTER_PRA: u32 = 0xBFE001;
        /// Port B data: parallel port.
        pub const REGISTER_PRB: u32 = 0xBFE101;
        /// Port A data direction.
        pub const REGISTER_DDRA: u32 = 0xBFE201;
        /// Port B data direction.
        pub const REGISTER_DDRB: u32 = 0xBFE301;
        /// Timer A low byte: counter on read, latch on write.
        pub const REGISTER_TALO: u32 = 0xBFE401;
        /// Timer A high byte: counter on read, latch on write.
        pub const REGISTER_TAHI: u32 = 0xBFE501;
        /// Timer B low byte: counter on read, latch on write.
        pub const REGISTER_TBLO: u32 = 0xBFE601;
        /// Timer B high byte: counter on read, latch on write.
        pub const REGISTER_TBHI: u32 = 0xBFE701;
        /// Time-of-day low byte on read; time or alarm low byte on write.
        pub const REGISTER_TODLO: u32 = 0xBFE801;
        /// Time-of-day middle byte on read; time or alarm middle byte on write.
        pub const REGISTER_TODMID: u32 = 0xBFE901;
        /// Time-of-day high byte on read; time or alarm high byte on write.
        pub const REGISTER_TODHI: u32 = 0xBFEA01;
        /// Serial data: received byte on read, transmit byte on write.
        pub const REGISTER_SDR: u32 = 0xBFEC01;
        /// Interrupt status on read (clears pending bits), mask on write.
        pub const REGISTER_ICR: u32 = 0xBFED01;
        /// Timer A and serial control.
        pub const REGISTER_CRA: u32 = 0xBFEE01;
        /// Timer B and time-of-day control.
        pub const REGISTER_CRB: u32 = 0xBFEF01;
    }

    /// CIA-B registers on the even-byte lane ($BFD000–$BFDF00).
    pub mod cia_b {
        /// Port A data: serial handshake and parallel control/status.
        pub const REGISTER_PRA: u32 = 0xBFD000;
        /// Port B data: floppy drive control.
        pub const REGISTER_PRB: u32 = 0xBFD100;
        /// Port A data direction.
        pub const REGISTER_DDRA: u32 = 0xBFD200;
        /// Port B data direction.
        pub const REGISTER_DDRB: u32 = 0xBFD300;
        /// Timer A low byte: counter on read, latch on write.
        pub const REGISTER_TALO: u32 = 0xBFD400;
        /// Timer A high byte: counter on read, latch on write.
        pub const REGISTER_TAHI: u32 = 0xBFD500;
        /// Timer B low byte: counter on read, latch on write.
        pub const REGISTER_TBLO: u32 = 0xBFD600;
        /// Timer B high byte: counter on read, latch on write.
        pub const REGISTER_TBHI: u32 = 0xBFD700;
        /// Time-of-day low byte on read; time or alarm low byte on write.
        pub const REGISTER_TODLO: u32 = 0xBFD800;
        /// Time-of-day middle byte on read; time or alarm middle byte on write.
        pub const REGISTER_TODMID: u32 = 0xBFD900;
        /// Time-of-day high byte on read; time or alarm high byte on write.
        pub const REGISTER_TODHI: u32 = 0xBFDA00;
        /// Serial data: received byte on read, transmit byte on write.
        pub const REGISTER_SDR: u32 = 0xBFDC00;
        /// Interrupt status on read (clears pending bits), mask on write.
        pub const REGISTER_ICR: u32 = 0xBFDD00;
        /// Timer A and serial control.
        pub const REGISTER_CRA: u32 = 0xBFDE00;
        /// Timer B and time-of-day control.
        pub const REGISTER_CRB: u32 = 0xBFDF00;
    }
}

/// Bitfields decoded by multiple chips or motherboard routing.
pub mod mask {
    /// Shared digital game-port encodings used by receivers and motherboard routing.
    pub mod game_port {
        /// Connector direction/quadrature contacts 1 through 4.
        pub const MASK_DIRECTION_INPUTS: u8 = 0x0f;
        /// Digital POT data bits in POTGO/POTGOR.
        pub const MASK_POT_DATA: u16 = 0x5500;
        /// Each POT output-enable bit sits one bit above its data bit.
        pub const BIT_POT_OUTPUT_ENABLE_SHIFT: u16 = 1;
    }

    /// Bit masks for DMACON ($096) and DMACONR ($002)
    pub mod dmacon {
        /// Bit 15: Set/Clear control bit (1 = set specified bits, 0 = clear specified bits)
        pub const BIT_SET_CLR: u16 = 0x8000;
        pub const MASK_WRITE: u16 = 0x7FFF;
        pub const MASK_READ_CONTROL: u16 = 0x07FF;
        /// Bit 14: Blitter busy status flag (read-only in DMACONR)
        pub const BIT_BBUSY: u16 = 0x4000;
        /// Bit 13: Blitter zero status flag (read-only in DMACONR)
        pub const BIT_BZERO: u16 = 0x2000;
        /// Bit 10: Blitter priority (BlitPri / Blitter Nasty: 1 = blitter has bus priority over CPU)
        pub const BIT_BLTPRI: u16 = 0x0400;
        /// Bit 9: Master DMA enable
        pub const BIT_DMAEN: u16 = 0x0200;
        /// Bit 8: Bitplane DMA enable
        pub const BIT_BPLEN: u16 = 0x0100;
        /// Bit 7: Copper DMA enable
        pub const BIT_COPEN: u16 = 0x0080;
        /// Bit 6: Blitter DMA enable
        pub const BIT_BLTEN: u16 = 0x0040;
        /// Bit 5: Sprite DMA enable
        pub const BIT_SPREN: u16 = 0x0020;
        /// Bit 4: Disk DMA enable
        pub const BIT_DSKEN: u16 = 0x0010;
        /// Bit 3: Audio channel 3 DMA enable
        pub const BIT_AUD3EN: u16 = 0x0008;
        /// Bit 2: Audio channel 2 DMA enable
        pub const BIT_AUD2EN: u16 = 0x0004;
        /// Bit 1: Audio channel 1 DMA enable
        pub const BIT_AUD1EN: u16 = 0x0002;
        /// Bit 0: Audio channel 0 DMA enable
        pub const BIT_AUD0EN: u16 = 0x0001;
        /// Mask covering all 4 audio channels
        pub const MASK_AUD_ALL: u16 = BIT_AUD0EN | BIT_AUD1EN | BIT_AUD2EN | BIT_AUD3EN;
    }

    /// Bit masks for BPLCON0 ($100)
    pub mod bplcon0 {
        /// Bit 15: High resolution mode (640 pixels per scanline)
        pub const BIT_HIRES: u16 = 0x8000;
        /// Bits 14..12: Number of active bitplanes (BPU2..BPU0)
        pub const MASK_BPU: u16 = 0x7000;
        /// Shift for BPU bitfield
        pub const BIT_BPU_SHIFT: u16 = 12;
        /// Bit 11: Hold and Modify mode (HAM)
        pub const BIT_HOMOD: u16 = 0x0800;
        /// Bit 10: Dual playfield mode
        pub const BIT_DBLPF: u16 = 0x0400;
        /// Bit 9: Color composite video enable
        pub const BIT_COLOR: u16 = 0x0200;
        /// Bit 8: Genlock audio enable
        pub const BIT_GAUD: u16 = 0x0100;
        /// Bit 3: Light pen trigger enable
        pub const BIT_LPEN: u16 = 0x0008;
        /// Bit 2: Interlace mode enable
        pub const BIT_LACE: u16 = 0x0004;
        /// Bit 1: External resync enable
        pub const BIT_ERSY: u16 = 0x0002;
    }
}
