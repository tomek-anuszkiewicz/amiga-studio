#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use paula::{Paula, SerialPort};

#[test]
fn transmit_words_use_lsb_order_automatic_start_and_explicit_stop_bits() {
    for (word, bits) in [(0x0155u16, 9), (0x02a5, 10), (0x8001, 16)] {
        for period in [1u16, 4, 43] {
            let mut serial = SerialPort::new();
            serial.write_serper(0x8000 | (period - 1));
            serial.write_serdat(word);
            assert_eq!(serial.read_status(false) & 0x3000, 0x1000);
            assert_eq!(serial.step_cck(true, false), 1);
            assert_eq!(serial.read_status(false) & 0x3000, 0x2000);
            let mut expected = vec![false];
            expected.extend((0..bits).map(|bit| word & (1 << bit) != 0));
            for high in expected {
                for _ in 0..period {
                    assert_eq!(serial.transmit_high(false), high);
                    assert!(!serial.transmit_high(true));
                    assert_eq!(serial.step_cck(true, false), 0);
                }
            }
            assert!(serial.transmit_high(false));
            assert_eq!(serial.read_status(false) & 0x3000, 0x3000);
        }
    }
}

#[test]
fn transmit_holding_word_waits_and_zero_does_not_start_a_packet() {
    let mut serial = SerialPort::new();
    serial.write_serper(3);
    serial.write_serdat(0);
    assert_eq!(serial.step_cck(true, false), 0);
    assert_eq!(serial.read_status(false), 0x3800);
    serial.write_serdat(0x100);
    assert_eq!(serial.step_cck(true, false), 1);
    serial.write_serdat(0x1ff);
    assert_eq!(serial.read_status(false) & 0x3000, 0);
    for _ in 0..39 {
        assert_eq!(serial.step_cck(true, false), 0);
    }
    assert_eq!(serial.step_cck(true, false), 1);
    assert_eq!(serial.read_status(false) & 0x3000, 0x2000);
    for _ in 0..40 {
        assert_eq!(serial.step_cck(true, false), 0);
    }
    assert_eq!(serial.read_status(false), 0x3800);
}

fn receive(serial: &mut SerialPort, word: u16, bits: u8, stop: bool, pending: bool) -> u16 {
    let period = (serial.serper & 0x7fff) + 1;
    let mut levels = vec![false];
    levels.extend((0..bits).map(|bit| word & (1 << bit) != 0));
    levels.push(stop);
    let mut requests = 0;
    for high in levels {
        for _ in 0..period {
            requests |= serial.step_cck(high, pending);
        }
    }
    requests
}

#[test]
fn receive_width_stop_readback_overrun_and_acknowledgement_follow_intreq() {
    for bits in [8, 9] {
        for period in [1, 4, 43] {
            let mut serial = SerialPort::new();
            serial.write_serper((period - 1) | if bits == 9 { 0x8000 } else { 0 });
            assert_eq!(receive(&mut serial, 0x155, bits, true, false), 0x0800);
            let first = if bits == 8 { 0x155 } else { 0x355 };
            assert_eq!(serial.read_status(true), 0x7800 | first);
            assert_eq!(serial.read_status(true), 0x7800 | first, "read is neutral");
            assert_eq!(receive(&mut serial, 0x0a6, bits, false, true), 0x0800);
            assert_eq!(serial.read_status(true), 0xf000 | 0x0a6);
            serial.acknowledge_receive();
            assert_eq!(serial.read_status(false), 0x3000 | 0x0a6);
            serial.reset();
            assert_eq!(serial, SerialPort::new());
        }
    }
}

#[test]
fn production_transmitter_interrupt_break_and_disconnected_receive_are_independent() {
    let mut paula = Paula::default();
    paula.set_intena(0xc001);
    paula.set_serper(3);
    paula.set_serdat(0x01ff);
    paula.step_cck();
    assert_eq!(paula.pending_interrupt_level(), 0);
    paula.step_cck();
    assert_eq!(paula.pending_interrupt_level(), 1);
    assert_eq!(paula.serdatr(), 0x2800);
    for _ in 0..4 {
        paula.step_cck();
    }
    assert!(paula.serial_tx_high());
    paula.set_adkcon(0x8800);
    paula.set_intreq(1);
    paula.step_cck();
    assert!(!paula.serial_tx_high());
    assert_eq!(paula.pending_interrupt_level(), 0);
    assert_eq!(paula.serdatr() & 0x4800, 0x0800);
    paula.set_adkcon(0x0800);
    paula.step_cck();
    assert!(paula.serial_tx_high());
}
