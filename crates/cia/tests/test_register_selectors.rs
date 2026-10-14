use cia::{Cia, CiaId};
use config::{cia_reg, MutationMode};

#[test]
fn full_addresses_route_to_cia_registers() {
    let mut cia_a = Cia::new(CiaId::A);
    let mut cia_b = Cia::new(CiaId::B);

    cia_a.write_register(cia_reg::cia_a::PRA, 0x5A);
    cia_b.write_register(cia_reg::cia_b::PRB, 0xA5);
    assert_eq!(cia_a.read_register(cia_reg::cia_a::PRA), 0x5A);
    assert_eq!(cia_b.read_register_debug(cia_reg::cia_b::PRB), 0xA5);

    assert_eq!(
        cia_a.stage_write(cia_reg::cia_a::DDRA, 0x03, 1, MutationMode::Pipeline),
        None
    );
    for _ in 0..4 {
        cia_a.step_cck();
    }
    assert_eq!(cia_a.step_cck()[0], Some((cia_reg::cia_a::DDRA, 0x03)));
    assert_eq!(cia_a.read_register(cia_reg::cia_a::DDRA), 0x03);

    assert_eq!(
        cia_b.stage_write(cia_reg::cia_b::DDRB, 0xC0, 1, MutationMode::Pipeline),
        None
    );
    for _ in 0..4 {
        cia_b.step_cck();
    }
    assert_eq!(cia_b.step_cck()[0], Some((cia_reg::cia_b::DDRB, 0xC0)));
    assert_eq!(cia_b.read_register(cia_reg::cia_b::DDRB), 0xC0);
}

#[test]
fn out_of_range_selector_does_not_alias_register_zero() {
    let mut cia = Cia::new(CiaId::A);
    cia.pra = 0x5A;

    assert_eq!(cia.read_register(0x10), 0xFF);
    assert_eq!(cia.peek_register(0x10), 0xFF);
    assert_eq!(cia.read_register_debug(0x10), 0xFF);

    assert_eq!(cia.write_register(0x10, 0xA5), Some((0x10, 0xA5)));
    assert_eq!(cia.pra, 0x5A);

    cia.commit_register_write(0x10, 0xB4);
    assert_eq!(cia.pra, 0x5A);

    assert_eq!(
        cia.stage_write(0x10, 0xC3, 0, MutationMode::Pipeline),
        Some((0x10, 0xC3))
    );
    assert_eq!(cia.pra, 0x5A);

    assert_eq!(cia.stage_write(0x10, 0xD2, 1, MutationMode::Pipeline), None);
    for _ in 0..5 {
        cia.step_cck();
    }
    assert_eq!(cia.pra, 0x5A);
}
