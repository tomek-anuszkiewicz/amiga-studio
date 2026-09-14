use cia::{Cia, CiaId};

#[test]
fn test_timer_b_cascaded_32bit_counting() {
    let mut cia = Cia::new(CiaId::A);
    cia.reset();

    // Enable Timer A (bit 0) and Timer B (bit 1) in ICR: $80 | 0x03 = $83
    cia.write_register(0xD, 0x83);

    // Timer A latch = 2 (underflows every 3 E-Clocks)
    cia.write_register(0x4, 2);
    cia.write_register(0x5, 0);

    // Timer B latch = 1 (underflows after 2 Timer A underflows)
    cia.write_register(0x6, 1);
    cia.write_register(0x7, 0);

    // CRA = $01: Start Timer A in continuous mode
    cia.write_register(0xE, 0x01);

    // CRB = $41: Start Timer B (bit 0) in cascaded mode (bits 6..5 = %10 -> $40)
    cia.write_register(0xF, 0x41);

    assert_eq!(cia.tb_counter, 1);
    assert!(!cia.irq_pending());

    // Step 15 CCKs (3 E-Clocks): Timer A underflows (2 -> 1 -> 0 -> reload 2)
    // Timer B should decrement from 1 to 0
    for _ in 0..15 {
        cia.step_cck();
    }
    assert_eq!(cia.tb_counter, 0);
    // ICR has bit 0 (Timer A) set
    assert_eq!(cia.read_register(0xD) & 0x01, 0x01);

    // Step another 15 CCKs (3 E-Clocks): Timer A underflows again
    // Timer B should underflow (0 -> reload 1) and trigger Timer B interrupt!
    for _ in 0..15 {
        cia.step_cck();
    }
    assert_eq!(cia.tb_counter, 1);
    let icr = cia.read_register(0xD);
    assert_eq!(icr & 0x02, 0x02); // Bit 1: Timer B underflow
}

#[test]
fn test_timer_b_one_shot_mode() {
    let mut cia = Cia::new(CiaId::A);
    cia.reset();

    // Timer B latch = 1
    cia.write_register(0x6, 1);
    cia.write_register(0x7, 0);

    // CRB = $09: Start Timer B (bit 0) in one-shot mode (bit 3) counting E-Clocks
    cia.write_register(0xF, 0x09);
    assert_eq!(cia.crb & 0x01, 0x01);

    // 1st E-Clock: 1 -> 0
    for _ in 0..5 {
        cia.step_cck();
    }
    assert_eq!(cia.tb_counter, 0);
    assert_eq!(cia.crb & 0x01, 0x01);

    // 2nd E-Clock: 0 -> reload & stop
    for _ in 0..5 {
        cia.step_cck();
    }
    assert_eq!(cia.tb_counter, 1);
    // Timer B must have stopped in one-shot mode
    assert_eq!(cia.crb & 0x01, 0x00);

    // 3rd E-Clock: should not decrement since timer is stopped
    for _ in 0..5 {
        cia.step_cck();
    }
    assert_eq!(cia.tb_counter, 1);
}

#[test]
fn test_cnt_pin_step_mode() {
    let mut cia = Cia::new(CiaId::A);
    cia.reset();

    // Timer B latch = 3
    cia.write_register(0x6, 3);
    cia.write_register(0x7, 0);

    // CRB = $21: Start Timer B (bit 0) in CNT input mode (bits 6..5 = %01 -> $20)
    cia.write_register(0xF, 0x21);

    // Stepping E-Clocks must NOT decrement Timer B in CNT mode
    for _ in 0..25 {
        cia.step_cck();
    }
    assert_eq!(cia.tb_counter, 3);

    // Pulsing CNT pin decrements Timer B
    cia.step_cnt();
    assert_eq!(cia.tb_counter, 2);

    cia.step_cnt();
    assert_eq!(cia.tb_counter, 1);
}

#[test]
fn test_tod_halt_and_restart_on_write() {
    let mut cia = Cia::new(CiaId::A);
    cia.reset();

    // Write TOD: writing TODHI ($0A) halts counter
    cia.write_register(0x0A, 0x12);
    assert!(cia.tod_halted);
    cia.write_register(0x09, 0x34);
    assert!(cia.tod_halted);

    // Ticking TOD while halted must not increment counter
    cia.tick_tod();
    cia.tick_tod();
    assert_eq!(cia.tod, 0x123400);

    // Writing TODLO ($08) restarts counter
    cia.write_register(0x08, 0x56);
    assert!(!cia.tod_halted);
    assert_eq!(cia.tod, 0x123456);

    // Now ticking increments
    cia.tick_tod();
    assert_eq!(cia.tod, 0x123457);
}

#[test]
fn test_tod_alarm_match_interrupt() {
    let mut cia = Cia::new(CiaId::A);
    cia.reset();

    // Enable TOD interrupt in ICR mask: $80 | 0x04 = $84
    cia.write_register(0xD, 0x84);
    assert_eq!(cia.icr_mask, 0x04);

    // Set CRB bit 7 = 1 to write alarm register
    cia.write_register(0xF, 0x80);
    cia.write_register(0x0A, 0x00);
    cia.write_register(0x09, 0x00);
    cia.write_register(0x08, 0x03);
    assert_eq!(cia.tod_alarm, 0x000003);

    // Set CRB bit 7 = 0 to write TOD clock counter
    cia.write_register(0xF, 0x00);
    cia.write_register(0x0A, 0x00);
    cia.write_register(0x09, 0x00);
    cia.write_register(0x08, 0x01);
    assert_eq!(cia.tod, 0x000001);
    assert!(!cia.irq_pending());

    // Tick 1: TOD = 2 (no match)
    cia.tick_tod();
    assert!(!cia.irq_pending());

    // Tick 2: TOD = 3 (alarm match!)
    cia.tick_tod();
    assert!(cia.irq_pending());
    assert_eq!(cia.read_register(0xD) & 0x04, 0x04);
    assert!(!cia.irq_pending());
}

#[test]
fn test_sdr_shift_in_and_flag_interrupt() {
    let mut cia = Cia::new(CiaId::A);
    cia.reset();

    // Enable SDR (bit 3) and FLAG (bit 4) interrupts in ICR: $80 | 0x18 = $98
    cia.write_register(0xD, 0x98);

    // Shift in keyboard scancode $40
    cia.shift_in_sdr(0x40);
    assert_eq!(cia.sdr, 0x40);
    assert!(cia.irq_pending());

    // Read ICR clears interrupt
    assert_eq!(cia.read_register(0xD) & 0x08, 0x08);
    assert!(!cia.irq_pending());

    // Trigger FLAG pin
    cia.trigger_flag_pin();
    assert!(cia.irq_pending());
    assert_eq!(cia.read_register(0xD) & 0x10, 0x10);
    assert!(!cia.irq_pending());
}
