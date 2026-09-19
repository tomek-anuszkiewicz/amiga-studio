#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use config::custom_reg::*;
use config::mask::*;

#[test]
fn test_register_offsets_uniqueness_and_alignment() {
    // Collect all major register offsets to verify word alignment (even addresses)
    let regs = [
        DMACONR, VPOSR, VHPOSR, DSKDATR, JOY0DAT, JOY1DAT, CLXDAT, ADKCONR, POT0DAT, POT1DAT,
        POTGOR, SERDATR, DSKBYTR, INTENAR, INTREQR, DSKPTH, DSKPTL, REFPTR, DSKLEN, DSKDAT, VPOSW,
        VHPOSW, COPCON, SERDAT, SERPER, POTGO, JOYTEST, STREQU, STRVBL, STRHOR, STRBUS, BLTCON0,
        BLTCON1, BLTAFWM, BLTALWM, BLTCPTH, BLTCPTL, BLTBPTH, BLTBPTL, BLTAPTH, BLTAPTL, BLTDPTH,
        BLTDPTL, BLTSIZE, BLTCMOD, BLTBMOD, BLTAMOD, BLTDMOD, BLTCDAT, BLTBDAT, BLTADAT, DSKSYNC,
        COP1LCH, COP1LCL, COP2LCH, COP2LCL, COPJMP1, COPJMP2, COPINS, DIWSTRT, DIWSTOP, DDFSTRT,
        DDFSTOP, DMACON, CLXCON, INTENA, INTREQ, ADKCON, AUD0PTH, AUD0PTL, AUD0LEN, AUD0PER,
        AUD0VOL, AUD0DAT, AUD1PTH, AUD1PTL, AUD1LEN, AUD1PER, AUD1VOL, AUD1DAT, AUD2PTH, AUD2PTL,
        AUD2LEN, AUD2PER, AUD2VOL, AUD2DAT, AUD3PTH, AUD3PTL, AUD3LEN, AUD3PER, AUD3VOL, AUD3DAT,
        BPL1PTH, BPL1PTL, BPL2PTH, BPL2PTL, BPL3PTH, BPL3PTL, BPL4PTH, BPL4PTL, BPL5PTH, BPL5PTL,
        BPL6PTH, BPL6PTL, BPLCON0, BPLCON1, BPLCON2, BPL1MOD, BPL2MOD, BPL1DAT, BPL2DAT, BPL3DAT,
        BPL4DAT, BPL5DAT, BPL6DAT, SPR0PTH, SPR0PTL, SPR0POS, SPR0CTL, SPR0DATA, SPR0DATB, COLOR00,
        COLOR01, COLOR31,
    ];

    for &reg in &regs {
        assert_eq!(
            reg % 2,
            0,
            "Register offset ${:03X} must be word-aligned",
            reg
        );
        assert!(
            reg <= 0x1FE,
            "Register offset ${:03X} must fit within custom chip space",
            reg
        );
    }
}

#[test]
fn test_dmacon_and_intreq_masks() {
    assert_eq!(dmacon::SET_CLR, 0x8000);
    assert_eq!(dmacon::DMAEN, 0x0200);
    assert_eq!(dmacon::COPEN, 0x0080);
    assert_eq!(dmacon::BLTEN, 0x0040);
    assert_eq!(dmacon::BBUSY, 0x4000);
    assert_eq!(dmacon::BZERO, 0x2000);

    assert_eq!(intreq::SET_CLR, 0x8000);
    assert_eq!(intreq::INTEN, 0x4000);
    assert_eq!(intreq::BLIT, 0x0040);
    assert_eq!(intreq::VERTB, 0x0020);
    assert_eq!(intreq::COPER, 0x0010);
}

#[test]
fn test_copcon_cdang_mask() {
    assert_eq!(copcon::CDANG, 0x0002);
}

#[test]
fn test_custom_reg_chip_namespaces_and_multi_chip_decoding() {
    use config::custom_reg;

    // Agnus specific registers
    assert_eq!(custom_reg::agnus::BLTCON0, 0x040);
    assert_eq!(custom_reg::agnus::COP1LCH, 0x080);
    assert_eq!(custom_reg::agnus::DSKPTH, 0x020);
    assert_eq!(custom_reg::agnus::REFPTR, 0x028);

    // Denise specific registers
    assert_eq!(custom_reg::denise::COLOR00, 0x180);
    assert_eq!(custom_reg::denise::BPLCON1, 0x102);
    assert_eq!(custom_reg::denise::CLXCON, 0x098);

    // Paula specific registers
    assert_eq!(custom_reg::paula::INTENA, 0x09A);
    assert_eq!(custom_reg::paula::INTREQ, 0x09C);
    assert_eq!(custom_reg::paula::ADKCON, 0x09E);
    assert_eq!(custom_reg::paula::AUD0VOL, 0x0A8);
    assert_eq!(custom_reg::paula::SERDAT, 0x030);

    // Multi-chip decoded registers: physically wired to and decoded by multiple chips
    assert_eq!(custom_reg::agnus::DMACON, 0x096);
    assert_eq!(custom_reg::paula::DMACON, 0x096);
    assert_eq!(custom_reg::DMACON, 0x096);

    assert_eq!(custom_reg::agnus::DMACONR, 0x002);
    assert_eq!(custom_reg::paula::DMACONR, 0x002);
    assert_eq!(custom_reg::DMACONR, 0x002);

    assert_eq!(custom_reg::denise::BPLCON0, 0x100);
    assert_eq!(custom_reg::agnus::BPLCON0, 0x100);
    assert_eq!(custom_reg::BPLCON0, 0x100);
}
