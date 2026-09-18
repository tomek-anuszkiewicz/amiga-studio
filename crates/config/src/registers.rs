//! Amiga 500 Custom Chip Registers & Bitfield Masks
//!
//! Authoritative register offset definitions and bitmask constants according to the
//! Commodore-Amiga Hardware Reference Manual (HRM), Appendix A.

/// Custom chip register offsets in the $DFF000 - $DFFFFE range.
///
/// Register offsets are word-aligned ($000 to $1FE).
/// Registers are organized by physical custom chip (Agnus, Denise, Paula),
/// with root-level re-exports for flat addressing compatibility.
pub mod custom_reg {
    /// Agnus (MOS 8370 / 8371 / 8372) Address Generator & DMA Coprocessor Registers
    pub mod agnus {
        /// Blitter destination early read (pseudo-register)
        pub const BLTDDAT: u16 = 0x000;
        /// DMA control read (channel status and blitter busy/zero flags)
        pub const DMACONR: u16 = 0x002;
        /// Vertical beam position high bit, chip revision ID, and LOF
        pub const VPOSR: u16 = 0x004;
        /// Vertical and horizontal beam position read (raster beam position)
        pub const VHPOSR: u16 = 0x006;
        /// Disk track buffer pointer high
        pub const DSKPTH: u16 = 0x020;
        /// Disk track buffer pointer low
        pub const DSKPTL: u16 = 0x022;
        /// Vertical position write (high bits)
        pub const VPOSW: u16 = 0x02A;
        /// Vertical and horizontal position write
        pub const VHPOSW: u16 = 0x02C;
        /// Copper control register (CDANG bit enables blitter register writes)
        pub const COPCON: u16 = 0x02E;
        /// Strobe register for light pen horizontal latch
        pub const STREQU: u16 = 0x038;
        /// Strobe register for light pen vertical latch
        pub const STRVBL: u16 = 0x03A;
        /// Strobe register for horizontal sync
        pub const STRHOR: u16 = 0x03C;
        /// Strobe register for vertical sync
        pub const STRBUS: u16 = 0x03E;
        /// Blitter control register 0 (channels, minterms, A-shift)
        pub const BLTCON0: u16 = 0x040;
        /// Blitter control register 1 (B-shift, line mode, fill, desc)
        pub const BLTCON1: u16 = 0x042;
        /// Blitter first word mask for channel A
        pub const BLTAFWM: u16 = 0x044;
        /// Blitter last word mask for channel A
        pub const BLTALWM: u16 = 0x046;
        /// Blitter channel C pointer high
        pub const BLTCPTH: u16 = 0x048;
        /// Blitter channel C pointer low
        pub const BLTCPTL: u16 = 0x04A;
        /// Blitter channel B pointer high
        pub const BLTBPTH: u16 = 0x04C;
        /// Blitter channel B pointer low
        pub const BLTBPTL: u16 = 0x04E;
        /// Blitter channel A pointer high
        pub const BLTAPTH: u16 = 0x050;
        /// Blitter channel A pointer low
        pub const BLTAPTL: u16 = 0x052;
        /// Blitter channel D pointer high
        pub const BLTDPTH: u16 = 0x054;
        /// Blitter channel D pointer low
        pub const BLTDPTL: u16 = 0x056;
        /// Blitter start & window size (rows and word width)
        pub const BLTSIZE: u16 = 0x058;
        /// Blitter channel C modulo
        pub const BLTCMOD: u16 = 0x060;
        /// Blitter channel B modulo
        pub const BLTBMOD: u16 = 0x062;
        /// Blitter channel A modulo
        pub const BLTAMOD: u16 = 0x064;
        /// Blitter channel D modulo
        pub const BLTDMOD: u16 = 0x066;
        /// Blitter channel C data latch
        pub const BLTCDAT: u16 = 0x070;
        /// Blitter channel B data latch
        pub const BLTBDAT: u16 = 0x072;
        /// Blitter channel A data latch
        pub const BLTADAT: u16 = 0x074;
        /// Copper list 1 location high (bits 18..16)
        pub const COP1LCH: u16 = 0x080;
        /// Copper list 1 location low (bits 15..1)
        pub const COP1LCL: u16 = 0x082;
        /// Copper list 2 location high (bits 18..16)
        pub const COP2LCH: u16 = 0x084;
        /// Copper list 2 location low (bits 15..1)
        pub const COP2LCL: u16 = 0x086;
        /// Copper restart list 1 strobe
        pub const COPJMP1: u16 = 0x088;
        /// Copper restart list 2 strobe
        pub const COPJMP2: u16 = 0x08A;
        /// Copper instruction fetch strobe
        pub const COPINS: u16 = 0x08C;
        /// Display data fetch start (horizontal bitplane DMA window start)
        pub const DDFSTRT: u16 = 0x092;
        /// Display data fetch stop (horizontal bitplane DMA window stop)
        pub const DDFSTOP: u16 = 0x094;
        /// DMA control write (set/clear bits for DMA channels)
        pub const DMACON: u16 = 0x096;
        /// Audio channel 0 pointer high (Location High)
        pub const AUD0LCH: u16 = 0x0A0;
        /// Audio channel 0 pointer high alias
        pub const AUD0PTH: u16 = 0x0A0;
        /// Audio channel 0 pointer low (Location Low)
        pub const AUD0LCL: u16 = 0x0A2;
        /// Audio channel 0 pointer low alias
        pub const AUD0PTL: u16 = 0x0A2;
        /// Audio channel 1 pointer high (Location High)
        pub const AUD1LCH: u16 = 0x0B0;
        /// Audio channel 1 pointer high alias
        pub const AUD1PTH: u16 = 0x0B0;
        /// Audio channel 1 pointer low (Location Low)
        pub const AUD1LCL: u16 = 0x0B2;
        /// Audio channel 1 pointer low alias
        pub const AUD1PTL: u16 = 0x0B2;
        /// Audio channel 2 pointer high (Location High)
        pub const AUD2LCH: u16 = 0x0C0;
        /// Audio channel 2 pointer high alias
        pub const AUD2PTH: u16 = 0x0C0;
        /// Audio channel 2 pointer low (Location Low)
        pub const AUD2LCL: u16 = 0x0C2;
        /// Audio channel 2 pointer low alias
        pub const AUD2PTL: u16 = 0x0C2;
        /// Audio channel 3 pointer high (Location High)
        pub const AUD3LCH: u16 = 0x0D0;
        /// Audio channel 3 pointer high alias
        pub const AUD3PTH: u16 = 0x0D0;
        /// Audio channel 3 pointer low (Location Low)
        pub const AUD3LCL: u16 = 0x0D2;
        /// Audio channel 3 pointer low alias
        pub const AUD3PTL: u16 = 0x0D2;
        /// Bitplane 1 pointer high
        pub const BPL1PTH: u16 = 0x0E0;
        /// Bitplane 1 pointer low
        pub const BPL1PTL: u16 = 0x0E2;
        /// Bitplane 2 pointer high
        pub const BPL2PTH: u16 = 0x0E4;
        /// Bitplane 2 pointer low
        pub const BPL2PTL: u16 = 0x0E6;
        /// Bitplane 3 pointer high
        pub const BPL3PTH: u16 = 0x0E8;
        /// Bitplane 3 pointer low
        pub const BPL3PTL: u16 = 0x0EA;
        /// Bitplane 4 pointer high
        pub const BPL4PTH: u16 = 0x0EC;
        /// Bitplane 4 pointer low
        pub const BPL4PTL: u16 = 0x0EE;
        /// Bitplane 5 pointer high
        pub const BPL5PTH: u16 = 0x0F0;
        /// Bitplane 5 pointer low
        pub const BPL5PTL: u16 = 0x0F2;
        /// Bitplane 6 pointer high
        pub const BPL6PTH: u16 = 0x0F4;
        /// Bitplane 6 pointer low
        pub const BPL6PTL: u16 = 0x0F6;
        /// Bitplane control 0 (hires, bitplane count, interlace, color)
        pub const BPLCON0: u16 = 0x100;
        /// Bitplane modulo 1 (odd bitplanes 1, 3, 5)
        pub const BPL1MOD: u16 = 0x108;
        /// Bitplane modulo 2 (even bitplanes 2, 4, 6)
        pub const BPL2MOD: u16 = 0x10A;
        /// Sprite 0 pointer high
        pub const SPR0PTH: u16 = 0x120;
        /// Sprite 0 pointer low
        pub const SPR0PTL: u16 = 0x122;
        /// Sprite 1 pointer high
        pub const SPR1PTH: u16 = 0x124;
        /// Sprite 1 pointer low
        pub const SPR1PTL: u16 = 0x126;
        /// Sprite 2 pointer high
        pub const SPR2PTH: u16 = 0x128;
        /// Sprite 2 pointer low
        pub const SPR2PTL: u16 = 0x12A;
        /// Sprite 3 pointer high
        pub const SPR3PTH: u16 = 0x12C;
        /// Sprite 3 pointer low
        pub const SPR3PTL: u16 = 0x12E;
        /// Sprite 4 pointer high
        pub const SPR4PTH: u16 = 0x130;
        /// Sprite 4 pointer low
        pub const SPR4PTL: u16 = 0x132;
        /// Sprite 5 pointer high
        pub const SPR5PTH: u16 = 0x134;
        /// Sprite 5 pointer low
        pub const SPR5PTL: u16 = 0x136;
        /// Sprite 6 pointer high
        pub const SPR6PTH: u16 = 0x138;
        /// Sprite 6 pointer low
        pub const SPR6PTL: u16 = 0x13A;
        /// Sprite 7 pointer high
        pub const SPR7PTH: u16 = 0x13C;
        /// Sprite 7 pointer low
        pub const SPR7PTL: u16 = 0x13E;
    }

    /// Denise (MOS 8362 / 8373) Video Display & Serializer Registers
    pub mod denise {
        /// Joystick/mouse 0 counter data
        pub const JOY0DAT: u16 = 0x00A;
        /// Joystick/mouse 1 counter data
        pub const JOY1DAT: u16 = 0x00C;
        /// Collision detection data read
        pub const CLXDAT: u16 = 0x00E;
        /// Joy/mouse test register
        pub const JOYTEST: u16 = 0x036;
        /// Display window start (upper-left coordinates)
        pub const DIWSTRT: u16 = 0x08E;
        /// Display window stop (lower-right coordinates)
        pub const DIWSTOP: u16 = 0x090;
        /// Collision control
        pub const CLXCON: u16 = 0x098;
        /// Bitplane control 0 (hires, bitplane count, interlace, color)
        pub const BPLCON0: u16 = 0x100;
        /// Bitplane control 1 (horizontal scroll deltas)
        pub const BPLCON1: u16 = 0x102;
        /// Bitplane control 2 (playfield/sprite priority)
        pub const BPLCON2: u16 = 0x104;
        /// Bitplane control 3 (ECS enhanced color & border features)
        pub const BPLCON3: u16 = 0x106;
        /// Bitplane 1 data buffer
        pub const BPL1DAT: u16 = 0x110;
        /// Bitplane 2 data buffer
        pub const BPL2DAT: u16 = 0x112;
        /// Bitplane 3 data buffer
        pub const BPL3DAT: u16 = 0x114;
        /// Bitplane 4 data buffer
        pub const BPL4DAT: u16 = 0x116;
        /// Bitplane 5 data buffer
        pub const BPL5DAT: u16 = 0x118;
        /// Bitplane 6 data buffer
        pub const BPL6DAT: u16 = 0x11A;
        /// Sprite 0 position data
        pub const SPR0POS: u16 = 0x140;
        /// Sprite 0 control data
        pub const SPR0CTL: u16 = 0x142;
        /// Sprite 0 image data A
        pub const SPR0DATA: u16 = 0x144;
        /// Sprite 0 image data B
        pub const SPR0DATB: u16 = 0x146;
        /// Sprite 1 position data
        pub const SPR1POS: u16 = 0x148;
        /// Sprite 1 control data
        pub const SPR1CTL: u16 = 0x14A;
        /// Sprite 1 image data A
        pub const SPR1DATA: u16 = 0x14C;
        /// Sprite 1 image data B
        pub const SPR1DATB: u16 = 0x14E;
        /// Sprite 2 position data
        pub const SPR2POS: u16 = 0x150;
        /// Sprite 2 control data
        pub const SPR2CTL: u16 = 0x152;
        /// Sprite 2 image data A
        pub const SPR2DATA: u16 = 0x154;
        /// Sprite 2 image data B
        pub const SPR2DATB: u16 = 0x156;
        /// Sprite 3 position data
        pub const SPR3POS: u16 = 0x158;
        /// Sprite 3 control data
        pub const SPR3CTL: u16 = 0x15A;
        /// Sprite 3 image data A
        pub const SPR3DATA: u16 = 0x15C;
        /// Sprite 3 image data B
        pub const SPR3DATB: u16 = 0x15E;
        /// Sprite 4 position data
        pub const SPR4POS: u16 = 0x160;
        /// Sprite 4 control data
        pub const SPR4CTL: u16 = 0x162;
        /// Sprite 4 image data A
        pub const SPR4DATA: u16 = 0x164;
        /// Sprite 4 image data B
        pub const SPR4DATB: u16 = 0x166;
        /// Sprite 5 position data
        pub const SPR5POS: u16 = 0x168;
        /// Sprite 5 control data
        pub const SPR5CTL: u16 = 0x16A;
        /// Sprite 5 image data A
        pub const SPR5DATA: u16 = 0x16C;
        /// Sprite 5 image data B
        pub const SPR5DATB: u16 = 0x16E;
        /// Sprite 6 position data
        pub const SPR6POS: u16 = 0x170;
        /// Sprite 6 control data
        pub const SPR6CTL: u16 = 0x172;
        /// Sprite 6 image data A
        pub const SPR6DATA: u16 = 0x174;
        /// Sprite 6 image data B
        pub const SPR6DATB: u16 = 0x176;
        /// Sprite 7 position data
        pub const SPR7POS: u16 = 0x178;
        /// Sprite 7 control data
        pub const SPR7CTL: u16 = 0x17A;
        /// Sprite 7 image data A
        pub const SPR7DATA: u16 = 0x17C;
        /// Sprite 7 image data B
        pub const SPR7DATB: u16 = 0x17E;
        /// Color register 00 (background color)
        pub const COLOR00: u16 = 0x180;
        /// Color register 01
        pub const COLOR01: u16 = 0x182;
        /// Color register 02
        pub const COLOR02: u16 = 0x184;
        /// Color register 03
        pub const COLOR03: u16 = 0x186;
        /// Color register 04
        pub const COLOR04: u16 = 0x188;
        /// Color register 05
        pub const COLOR05: u16 = 0x18A;
        /// Color register 06
        pub const COLOR06: u16 = 0x18C;
        /// Color register 07
        pub const COLOR07: u16 = 0x18E;
        /// Color register 08
        pub const COLOR08: u16 = 0x190;
        /// Color register 09
        pub const COLOR09: u16 = 0x192;
        /// Color register 10
        pub const COLOR10: u16 = 0x194;
        /// Color register 11
        pub const COLOR11: u16 = 0x196;
        /// Color register 12
        pub const COLOR12: u16 = 0x198;
        /// Color register 13
        pub const COLOR13: u16 = 0x19A;
        /// Color register 14
        pub const COLOR14: u16 = 0x19C;
        /// Color register 15
        pub const COLOR15: u16 = 0x19E;
        /// Color register 16
        pub const COLOR16: u16 = 0x1A0;
        /// Color register 17
        pub const COLOR17: u16 = 0x1A2;
        /// Color register 18
        pub const COLOR18: u16 = 0x1A4;
        /// Color register 19
        pub const COLOR19: u16 = 0x1A6;
        /// Color register 20
        pub const COLOR20: u16 = 0x1A8;
        /// Color register 21
        pub const COLOR21: u16 = 0x1AA;
        /// Color register 22
        pub const COLOR22: u16 = 0x1AC;
        /// Color register 23
        pub const COLOR23: u16 = 0x1AE;
        /// Color register 24
        pub const COLOR24: u16 = 0x1B0;
        /// Color register 25
        pub const COLOR25: u16 = 0x1B2;
        /// Color register 26
        pub const COLOR26: u16 = 0x1B4;
        /// Color register 27
        pub const COLOR27: u16 = 0x1B6;
        /// Color register 28
        pub const COLOR28: u16 = 0x1B8;
        /// Color register 29
        pub const COLOR29: u16 = 0x1BA;
        /// Color register 30
        pub const COLOR30: u16 = 0x1BC;
        /// Color register 31
        pub const COLOR31: u16 = 0x1BE;
    }

    /// Paula (MOS 8364) Audio, Disk, and Interrupt Controller Registers
    pub mod paula {
        /// DMA control read (channel status and blitter busy/zero flags)
        pub const DMACONR: u16 = 0x002;
        /// Disk DMA data read
        pub const DSKDATR: u16 = 0x008;
        /// Audio, disk, and UART control read
        pub const ADKCONR: u16 = 0x010;
        /// Potentiometer 0 data (port 1 pin 5/9)
        pub const POT0DAT: u16 = 0x012;
        /// Potentiometer 1 data (port 2 pin 5/9)
        pub const POT1DAT: u16 = 0x014;
        /// Potentiometer port data read (start pins / button 2/3)
        pub const POTGOR: u16 = 0x016;
        /// Serial port data and status read
        pub const SERDATR: u16 = 0x018;
        /// Disk data byte and status read
        pub const DSKBYTR: u16 = 0x01A;
        /// Interrupt enable bits read
        pub const INTENAR: u16 = 0x01C;
        /// Interrupt request bits read
        pub const INTREQR: u16 = 0x01E;
        /// Disk track buffer length
        pub const DSKLEN: u16 = 0x024;
        /// Disk DMA data write
        pub const DSKDAT: u16 = 0x026;
        /// Serial port data write
        pub const SERDAT: u16 = 0x030;
        /// Serial port period and control
        pub const SERPER: u16 = 0x032;
        /// Potentiometer port data write and start
        pub const POTGO: u16 = 0x034;
        /// Disk sync pattern register
        pub const DSKSYNC: u16 = 0x07E;
        /// DMA control write (set/clear bits for DMA channels)
        pub const DMACON: u16 = 0x096;
        /// Interrupt enable bits write
        pub const INTENA: u16 = 0x09A;
        /// Interrupt request bits write
        pub const INTREQ: u16 = 0x09C;
        /// Audio, disk, and UART control write
        pub const ADKCON: u16 = 0x09E;
        /// Audio channel 0 length (words)
        pub const AUD0LEN: u16 = 0x0A4;
        /// Audio channel 0 sample period
        pub const AUD0PER: u16 = 0x0A6;
        /// Audio channel 0 volume
        pub const AUD0VOL: u16 = 0x0A8;
        /// Audio channel 0 data word
        pub const AUD0DAT: u16 = 0x0AA;
        /// Audio channel 1 length (words)
        pub const AUD1LEN: u16 = 0x0B4;
        /// Audio channel 1 sample period
        pub const AUD1PER: u16 = 0x0B6;
        /// Audio channel 1 volume
        pub const AUD1VOL: u16 = 0x0B8;
        /// Audio channel 1 data word
        pub const AUD1DAT: u16 = 0x0BA;
        /// Audio channel 2 length (words)
        pub const AUD2LEN: u16 = 0x0C4;
        /// Audio channel 2 sample period
        pub const AUD2PER: u16 = 0x0C6;
        /// Audio channel 2 volume
        pub const AUD2VOL: u16 = 0x0C8;
        /// Audio channel 2 data word
        pub const AUD2DAT: u16 = 0x0CA;
        /// Audio channel 3 length (words)
        pub const AUD3LEN: u16 = 0x0D4;
        /// Audio channel 3 sample period
        pub const AUD3PER: u16 = 0x0D6;
        /// Audio channel 3 volume
        pub const AUD3VOL: u16 = 0x0D8;
        /// Audio channel 3 data word
        pub const AUD3DAT: u16 = 0x0DA;
    }

    // Flat re-exports for seamless compatibility and general access
    pub use agnus::*;
    pub use denise::*;
    pub use paula::*;

    // Explicit disambiguation for registers decoded by multiple physical chips
    pub const DMACON: u16 = agnus::DMACON;
    pub const DMACONR: u16 = agnus::DMACONR;
    pub const BPLCON0: u16 = denise::BPLCON0;
}

/// Bitfield masks for custom chip control and status registers
pub mod mask {
    /// Bit masks for DMACON ($096) and DMACONR ($002)
    pub mod dmacon {
        /// Bit 15: Set/Clear control bit (1 = set specified bits, 0 = clear specified bits)
        pub const SET_CLR: u16 = 0x8000;
        /// Bit 14: Blitter busy status flag (read-only in DMACONR)
        pub const BBUSY: u16 = 0x4000;
        /// Bit 13: Blitter zero status flag (read-only in DMACONR)
        pub const BZERO: u16 = 0x2000;
        /// Bit 10: Blitter priority (BlitPri / Blitter Nasty: 1 = blitter has bus priority over CPU)
        pub const BLTPRI: u16 = 0x0400;
        /// Bit 9: Master DMA enable
        pub const DMAEN: u16 = 0x0200;
        /// Bit 8: Bitplane DMA enable
        pub const BPLEN: u16 = 0x0100;
        /// Bit 7: Copper DMA enable
        pub const COPEN: u16 = 0x0080;
        /// Bit 6: Blitter DMA enable
        pub const BLTEN: u16 = 0x0040;
        /// Bit 5: Sprite DMA enable
        pub const SPREN: u16 = 0x0020;
        /// Bit 4: Disk DMA enable
        pub const DSKEN: u16 = 0x0010;
        /// Bit 3: Audio channel 3 DMA enable
        pub const AUD3EN: u16 = 0x0008;
        /// Bit 2: Audio channel 2 DMA enable
        pub const AUD2EN: u16 = 0x0004;
        /// Bit 1: Audio channel 1 DMA enable
        pub const AUD1EN: u16 = 0x0002;
        /// Bit 0: Audio channel 0 DMA enable
        pub const AUD0EN: u16 = 0x0001;
        /// Mask covering all 4 audio channels
        pub const AUD_ALL: u16 = AUD0EN | AUD1EN | AUD2EN | AUD3EN;
    }

    /// Bit masks for INTENA ($09A), INTENAR ($01C), INTREQ ($09C), INTREQR ($01E)
    pub mod intreq {
        /// Bit 15: Set/Clear control bit
        pub const SET_CLR: u16 = 0x8000;
        /// Bit 14: Master interrupt enable (INTEN)
        pub const INTEN: u16 = 0x4000;
        /// Bit 13: External interrupt (Level 6)
        pub const EXTER: u16 = 0x2000;
        /// Bit 12: Disk sync match (Level 5)
        pub const DSKSYN: u16 = 0x1000;
        /// Bit 11: Serial receive buffer full (Level 5)
        pub const RBF: u16 = 0x0800;
        /// Bit 10: Audio channel 3 finished (Level 4)
        pub const AUD3: u16 = 0x0400;
        /// Bit 9: Audio channel 2 finished (Level 4)
        pub const AUD2: u16 = 0x0200;
        /// Bit 8: Audio channel 1 finished (Level 4)
        pub const AUD1: u16 = 0x0100;
        /// Bit 7: Audio channel 0 finished (Level 4)
        pub const AUD0: u16 = 0x0080;
        /// Bit 6: Blitter finished (Level 3)
        pub const BLIT: u16 = 0x0040;
        /// Bit 5: Vertical blank interval start (Level 3)
        pub const VERTB: u16 = 0x0020;
        /// Bit 4: Copper instruction finished / trap (Level 3)
        pub const COPER: u16 = 0x0010;
        /// Bit 3: I/O Ports and timers (CIA-A, Level 2)
        pub const PORTS: u16 = 0x0008;
        /// Bit 2: Software interrupt (Level 1)
        pub const SOFT: u16 = 0x0004;
        /// Bit 1: Disk block transfer finished (Level 1)
        pub const DSKBLK: u16 = 0x0002;
        /// Bit 0: Serial transmit buffer empty (Level 1)
        pub const TBE: u16 = 0x0001;
    }

    /// Bit masks for COPCON ($02E)
    pub mod copcon {
        /// Bit 1: Copper Danger bit (allows Copper to write registers below $080, e.g. Blitter)
        pub const CDANG: u16 = 0x0002;
    }

    /// Bit masks for VPOSR ($004)
    pub mod vposr {
        /// Bit 15: Long Frame toggle bit (0 = short field, 1 = long field in interlace)
        pub const LOF: u16 = 0x8000;
        /// Bits 14..8: Agnus chip revision ID
        pub const CHIP_ID_MASK: u16 = 0x7F00;
        /// OCS NTSC 8370 chip ID value
        pub const CHIP_ID_NTSC_8370: u16 = 0x1000;
        /// OCS PAL 8371 chip ID value
        pub const CHIP_ID_PAL_8371: u16 = 0x0000;
        /// Bit 7: Long Line bit (ECS LOL)
        pub const LOL: u16 = 0x0080;
        /// Bits 2..0: Vertical position high bits (V10..V8)
        pub const VPOS_HI_MASK: u16 = 0x0007;
        /// Bit 0: Vertical position bit 8 (V8)
        pub const V8: u16 = 0x0001;
    }

    /// Bit masks for BPLCON0 ($100)
    pub mod bplcon0 {
        /// Bit 15: High resolution mode (640 pixels per scanline)
        pub const HIRES: u16 = 0x8000;
        /// Bits 14..12: Number of active bitplanes (BPU2..BPU0)
        pub const BPU_MASK: u16 = 0x7000;
        /// Shift for BPU bitfield
        pub const BPU_SHIFT: u16 = 12;
        /// Bit 11: Hold and Modify mode (HAM)
        pub const HOMOD: u16 = 0x0800;
        /// Bit 10: Dual playfield mode
        pub const DBLPF: u16 = 0x0400;
        /// Bit 9: Color composite video enable
        pub const COLOR: u16 = 0x0200;
        /// Bit 8: Genlock audio enable
        pub const GAUD: u16 = 0x0100;
        /// Bit 3: Light pen trigger enable
        pub const LPEN: u16 = 0x0008;
        /// Bit 2: Interlace mode enable
        pub const LACE: u16 = 0x0004;
        /// Bit 1: External resync enable
        pub const ERSY: u16 = 0x0002;
    }

    /// Bit masks for BLTCON0 ($040) and BLTCON1 ($042)
    pub mod bltcon {
        /// BLTCON0 Bit 11: Channel A DMA enable
        pub const USEA: u16 = 0x0800;
        /// BLTCON0 Bit 10: Channel B DMA enable
        pub const USEB: u16 = 0x0400;
        /// BLTCON0 Bit 9: Channel C DMA enable
        pub const USEC: u16 = 0x0200;
        /// BLTCON0 Bit 8: Channel D DMA enable
        pub const USED: u16 = 0x0100;
        /// BLTCON0 Bits 7..0: 8-bit minterm logical operation code
        pub const MINTERM_MASK: u16 = 0x00FF;
        /// BLTCON1 Bit 0: Line mode enable (0 = area fill/copy, 1 = line draw)
        pub const LINE: u16 = 0x0001;
        /// BLTCON1 Bit 1: Descending addressing mode (decrement pointers)
        pub const DESC: u16 = 0x0002;
        /// BLTCON1 Bit 2: Single-bit fill mode
        pub const SING: u16 = 0x0004;
        /// BLTCON1 Bit 3: Exclusive-OR fill mode
        pub const EFE: u16 = 0x0008;
        /// BLTCON1 Bit 4: Inclusive-OR fill mode
        pub const IFE: u16 = 0x0010;
        /// BLTCON1 Bit 7: Disable channel D output
        pub const DOFF: u16 = 0x0080;
    }
}
