#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use config::{custom_reg, A500Config, VideoStandard};
use machine_loop::{A500Machine, A500State, AddressBus, BusResult};

fn idle_machine() -> A500Machine {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    machine.cpu.state.set_stopped(true);
    machine
}

#[test]
fn palette_write_waits_for_next_custom_chip_step() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    machine.cpu.state.set_stopped(true);
    machine
        .memory_bus()
        .write_custom_word(custom_reg::denise::COLOR00, 0x0abc);
    assert_eq!(
        machine.denise.color[0], 0,
        "pending palette must remain invisible"
    );
    machine.step_cck();
    assert_eq!(machine.denise.color[0], 0x0abc);
}

#[test]
fn every_simultaneously_matured_paula_write_is_applied() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    machine.cpu.state.set_stopped(true);
    for (offset, value) in [
        (custom_reg::paula::AUD0LEN, 10),
        (custom_reg::paula::AUD0PER, 100),
        (custom_reg::paula::AUD0VOL, 20),
        (custom_reg::paula::AUD1LEN, 11),
        (custom_reg::paula::AUD1PER, 101),
        (custom_reg::paula::AUD1VOL, 21),
        (custom_reg::paula::AUD2LEN, 12),
        (custom_reg::paula::AUD2PER, 102),
        (custom_reg::paula::POTGO, 0xabcd),
    ] {
        machine.memory_bus().write_custom_word(offset, value);
    }
    machine.step_cycles(2);
    assert_eq!(
        machine.paula.potgo, 0xabcd,
        "ninth due event must be applied"
    );
}

#[test]
fn concrete_setters_delay_controls_and_reads_do_not_flush_pending_changes() {
    let mut machine = idle_machine();
    assert!(machine.agnus.set_dmacon(0x8200));
    assert!(machine.denise.set_bplcon0(0x1000));
    assert!(machine.paula.set_intena(0xc020));
    for _ in 0..2 {
        assert_eq!(
            machine
                .memory_bus()
                .read_custom_word(custom_reg::agnus::DMACONR)
                & 0x200,
            0
        );
        assert_eq!(
            machine
                .memory_bus()
                .read_custom_word(custom_reg::paula::INTENAR),
            0
        );
        assert_eq!(
            machine
                .memory_bus()
                .read_custom_word(custom_reg::denise::BPLCON0),
            0xffff
        );
    }
    assert_eq!(machine.denise.bplcon0, 0);
    machine.step_cck();
    assert_eq!(machine.denise.bplcon0, 0x1000);
    assert_eq!(
        machine
            .memory_bus()
            .read_custom_word(custom_reg::paula::INTENAR),
        0x4020
    );
    assert_eq!(
        machine
            .memory_bus()
            .read_custom_word(custom_reg::agnus::DMACONR)
            & 0x200,
        0
    );
    machine.step_cck();
    assert_eq!(
        machine
            .memory_bus()
            .read_custom_word(custom_reg::agnus::DMACONR)
            & 0x200,
        0x200
    );
    assert_eq!(
        machine
            .memory_bus()
            .read_custom_word(custom_reg::denise::BPLCON0),
        0xffff
    );
}

#[test]
fn address_strobes_act_immediately_without_register_queue_entries() {
    let mut machine = idle_machine();
    machine.agnus.copper.cop1lc = 0x2000;
    machine.agnus.copper.cop2lc = 0x3000;
    assert!(machine
        .memory_bus()
        .write_custom_word(custom_reg::agnus::COPJMP1, 0xffff));
    assert_eq!(machine.agnus.copper.cop_pc, 0x2000);
    assert_eq!(
        machine
            .memory_bus()
            .read_custom_word(custom_reg::agnus::COPJMP2),
        0xffff
    );
    assert_eq!(machine.agnus.copper.cop_pc, 0x3000);
    assert!(machine.agnus.mutations.iter().all(Option::is_none));

    for offset in [
        custom_reg::denise::STREQU,
        custom_reg::denise::STRVBL,
        custom_reg::denise::STRHOR,
        custom_reg::denise::STRLONG,
    ] {
        assert!(machine.memory_bus().write_custom_word(offset, 0x1234));
    }
    assert!(machine.denise.mutations.iter().all(Option::is_none));
    assert!(machine.paula.mutations.iter().all(Option::is_none));
    assert!(machine.denise.sync.long_line);
    assert_eq!(machine.paula.sync.cck_since_horizontal, 0);
    assert_eq!(machine.denise.bplcon0, 0);
    assert_eq!(machine.paula.dma_enables, 0);
}

#[test]
fn full_receiver_rejects_shared_transfer_atomically_without_blocking_unrelated_chip() {
    let mut machine = idle_machine();
    for value in 0..agnus::Agnus::MUTATION_CAPACITY as u16 {
        assert!(machine.agnus.set_bltadat(value));
    }
    assert!(!machine.agnus.set_dmacon(0x8200));
    for _ in 0..2 {
        assert_eq!(
            machine.memory_bus().write_word(0xdff096, 0x8200),
            BusResult::WaitState
        );
        assert!(machine.paula.mutations.iter().all(Option::is_none));
        assert_eq!(machine.agnus.dmacon, 0);
    }
    assert_eq!(
        machine.memory_bus().write_byte(0xdff181, 0x33),
        BusResult::Ready(())
    );
    assert_eq!(machine.denise.color[0], 0);
    machine.step_cck();
    assert_eq!(machine.denise.color[0], 0x333);
    assert_eq!(machine.agnus.blitter.bltadat, 63);
    assert_eq!(
        machine.memory_bus().write_word(0xdff096, 0x8200),
        BusResult::Ready(())
    );
    assert_eq!(machine.agnus.mutations.iter().flatten().count(), 1);
    assert_eq!(machine.paula.mutations.iter().flatten().count(), 1);
    machine.step_cycles(2);
    assert_eq!(machine.agnus.dmacon, 0x200);
    assert!(machine.paula.dma_master);
    assert!(machine.agnus.mutations.iter().all(Option::is_none));
    assert!(machine.paula.mutations.iter().all(Option::is_none));
}

#[test]
fn full_capacity_register_replay_does_not_block_interrupt_pins() {
    let mut machine = idle_machine();
    for value in 0..agnus::Agnus::MUTATION_CAPACITY as u16 {
        assert!(machine.agnus.set_bltadat(value));
        assert!(machine.denise.set_bpldat(0, value));
    }
    let agnus_due = machine.agnus.step_cck();
    let denise_due = machine.denise.step_cck();
    assert_eq!(
        agnus_due.iter().flatten().count(),
        agnus::Agnus::MUTATION_CAPACITY
    );
    assert_eq!(
        denise_due.iter().flatten().count(),
        denise::Denise::MUTATION_CAPACITY
    );
    for (value, entry) in agnus_due.into_iter().flatten().enumerate() {
        assert_eq!(entry, (custom_reg::agnus::BLTADAT, value as u16));
    }
    assert_eq!(machine.denise.bpldat[0], 63);
    assert_eq!(machine.denise.bpldat_pipe[0], 63);
    assert!(machine.denise.bpl_armed);
    // One CPU clear plus data events fill all register-write capacity.
    assert!(machine.paula.set_intreq(0x2008));
    for value in 0..paula::Paula::MUTATION_CAPACITY as u16 - 1 {
        assert!(machine.paula.set_auddat(0, value));
    }
    assert!(!machine.paula.set_serdat(0x1234));
    machine.paula.pins.int2_high = false;
    machine.paula.pins.int6_high = false;
    assert_eq!(
        machine.paula.mutations.iter().flatten().count(),
        paula::Paula::MUTATION_CAPACITY
    );
    assert_eq!(machine.paula.intreqr(), 0);
    let due = machine.paula.step_cck();
    assert_eq!(
        due.iter().flatten().count(),
        paula::Paula::MUTATION_CAPACITY
    );
    assert_eq!(
        machine.paula.intreqr(),
        0x2008,
        "delivered input sampling follows due register commitment"
    );
    assert!(machine.paula.mutations.iter().all(Option::is_none));
    assert_eq!(machine.paula.step_cck().iter().flatten().count(), 0);
}

#[test]
fn interrupt_input_levels_are_sampled_after_cpu_acknowledgments() {
    let mut machine = idle_machine();
    machine.paula.receive_interrupt_pins(false, true, false);
    assert!(machine.paula.set_intreq(0x0008));
    assert_eq!(machine.paula.mutations.iter().flatten().count(), 1);
    assert_eq!(
        machine.paula.intreqr(),
        0,
        "pin capture is not register commitment"
    );
    machine.paula.step_cck();
    assert_eq!(
        machine.paula.intreqr(),
        0x2008,
        "held inputs win this CCK sampling phase"
    );
    machine.paula.receive_interrupt_pins(true, true, true);
    machine.paula.step_cck();
    assert_eq!(
        machine.paula.intreqr(),
        0x2008,
        "release does not acknowledge the latch"
    );
    assert!(machine.paula.set_intreq(0x0008));
    machine.paula.step_cck();
    assert_eq!(machine.paula.intreqr(), 0x2000);
}

#[test]
fn copper_and_cpu_shared_writes_start_from_the_same_receiver_boundary() {
    let mut copper = idle_machine();
    copper.physical_memory.map_chip_ram_to_low_memory();
    for (index, word) in [0x0100_u16, 0x1000, 0xffff, 0xfffe].into_iter().enumerate() {
        copper
            .physical_memory
            .write_word_debug(0x2000 + index as u32 * 2, word);
    }
    assert!(copper.agnus.set_cop1lcl(0x2000));
    assert!(copper
        .memory_bus()
        .write_custom_word(custom_reg::agnus::DMACON, 0x8280));
    copper.step_cycles(2);
    copper.agnus.strobe_copjmp1();
    let mut issued = false;
    for _ in 0..24 {
        copper.step_cck();
        if copper
            .agnus
            .mutations
            .iter()
            .flatten()
            .any(|m| m.reg_offset == custom_reg::agnus::BPLCON0)
        {
            issued = true;
            break;
        }
    }
    assert!(issued, "authored Copper MOVE was not issued");
    assert_eq!(copper.agnus.bplcon0, 0);
    assert_eq!(copper.denise.bplcon0, 0);
    let mut cpu_bus = idle_machine();
    assert!(cpu_bus
        .memory_bus()
        .write_custom_word(custom_reg::agnus::BPLCON0, 0x1000));
    for step in 1..=4 {
        copper.step_cck();
        cpu_bus.step_cck();
        assert_eq!(
            copper.agnus.bplcon0, cpu_bus.agnus.bplcon0,
            "Agnus step {step}"
        );
        assert_eq!(
            copper.denise.bplcon0, cpu_bus.denise.bplcon0,
            "Denise step {step}"
        );
        assert_eq!(copper.denise.bplcon0, 0x1000);
        assert_eq!(copper.agnus.bplcon0, if step == 4 { 0x1000 } else { 0 });
    }
}

#[test]
fn pending_registers_and_unconsumed_bus_outputs_resume_after_serialization() {
    let mut machine = idle_machine();
    assert!(machine
        .memory_bus()
        .write_custom_word(custom_reg::agnus::BPLCON0, 0x1000));
    assert!(machine
        .memory_bus()
        .write_custom_word(custom_reg::agnus::DMACON, 0x8200));
    assert!(machine.denise.set_color(2, 0x0abc));
    assert!(machine.paula.set_adkcon(0x8001));
    machine.paula.pins.int2_high = false;
    machine.agnus.pending_copper_write = Some((custom_reg::denise::COLOR03, 0x123));
    machine.agnus.pending_bpl_dma = Some((0, 0x4567));
    let json = serde_json::to_string(&machine.save_state()).unwrap();
    let state = serde_json::from_str::<A500State>(&json).unwrap();
    let mut restored = idle_machine();
    restored.load_state(&state);
    assert!(machine.save_state() == restored.save_state());
    for step in 1..=5 {
        machine.step_cck();
        restored.step_cck();
        assert!(
            machine.save_state() == restored.save_state(),
            "continuation step {step}"
        );
        assert_eq!(restored.denise.color[3], if step >= 2 { 0x123 } else { 0 });
        assert_eq!(
            restored.denise.bpldat[0],
            if step >= 2 { 0x4567 } else { 0 }
        );
    }
}

#[test]
fn authored_cpu_byte_write_propagates_once_while_dma_competes_for_chip_ram() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = A500Machine::new(A500Config::bare_512k(standard));
        machine.physical_memory.map_chip_ram_to_low_memory();
        // MOVE.B #$5A,$DFF181; STOP #$2700. No firmware or disk media.
        for (index, word) in [0x13fc_u16, 0x005a, 0x00df, 0xf181, 0x4e72, 0x2700]
            .into_iter()
            .enumerate()
        {
            machine
                .physical_memory
                .write_word_debug(0x1000 + index as u32 * 2, word);
        }
        machine.cpu.state.set_stopped(true);
        assert!(machine
            .memory_bus()
            .write_custom_word(custom_reg::agnus::DMACON, 0x8300));
        assert!(machine
            .memory_bus()
            .write_custom_word(custom_reg::agnus::BPLCON0, 0x6000));
        machine.step_cycles(4);
        machine.agnus.hpos = 0x36;
        machine.agnus.vpos = 100;
        machine.set_pc_and_prime_prefetch(0x1000);
        machine.cpu.state.set_stopped(false);
        machine.cpu.state.set_sr(0x2700);
        let mut accepted = 0;
        let mut blocked = false;
        for _ in 0..400 {
            machine.step_cck();
            blocked |= machine.physical_memory.chip_ram_blocked;
            if machine
                .denise
                .mutations
                .iter()
                .flatten()
                .any(|m| m.reg_offset == custom_reg::denise::COLOR00)
            {
                accepted += 1;
                assert_eq!(machine.denise.color[0], 0);
            }
            if machine.cpu.state.is_stopped() {
                break;
            }
        }
        assert!(blocked, "active DMA did not compete for Chip RAM");
        assert!(machine.cpu.state.is_stopped());
        assert_eq!(
            accepted, 1,
            "one accepted CPU transfer inserts one palette event"
        );
        assert_eq!(machine.denise.color[0], 0x0a5a);
    }
}

#[test]
fn dma_disable_retains_accepted_bitplane_word_and_enable_restores_fetches() {
    let mut machine = idle_machine();
    machine.physical_memory.map_chip_ram_to_low_memory();
    assert!(machine
        .memory_bus()
        .write_custom_word(custom_reg::agnus::BPLCON0, 0x1000));
    assert!(machine
        .memory_bus()
        .write_custom_word(custom_reg::agnus::DMACON, 0x8300));
    machine.step_cycles(4);
    machine.physical_memory.write_word_debug(0x2000, 0xabcd);
    machine.physical_memory.write_word_debug(0x2002, 0x5678);
    machine.agnus.bplpt[0] = 0x2000;
    machine.agnus.vpos = 100;
    machine.agnus.hpos = 0x3e;
    machine.step_cck();
    assert!(machine.physical_memory.chip_ram_blocked);
    assert_eq!(machine.agnus.bplpt[0], 0x2002);
    assert_eq!(machine.denise.bpldat[0], 0);
    assert!(machine
        .memory_bus()
        .write_custom_word(custom_reg::agnus::DMACON, 0x0100));
    machine.step_cck();
    assert_eq!(
        machine.denise.bpldat[0], 0xabcd,
        "accepted data survives a pending DMA disable"
    );
    assert_eq!(machine.agnus.dmacon & 0x100, 0x100);
    machine.step_cck();
    assert_eq!(machine.agnus.dmacon & 0x100, 0);
    machine.agnus.hpos = 0x3e;
    machine.step_cck();
    assert_eq!(machine.agnus.bplpt[0], 0x2002);
    assert!(!machine.physical_memory.chip_ram_blocked);
    assert!(machine
        .memory_bus()
        .write_custom_word(custom_reg::agnus::DMACON, 0x8100));
    machine.step_cycles(2);
    machine.agnus.hpos = 0x3e;
    machine.step_cck();
    assert_eq!(machine.agnus.bplpt[0], 0x2004);
    assert_eq!(machine.denise.bpldat[0], 0xabcd);
    machine.step_cck();
    assert_eq!(machine.denise.bpldat[0], 0x5678);
}

#[test]
fn blitter_restart_and_control_data_changes_apply_at_their_own_commit_boundaries() {
    let mut machine = idle_machine();
    let previous_data = machine.agnus.blitter.bltadat;
    assert!(machine.agnus.set_bltcon0(0x09f0));
    assert!(machine.agnus.set_bltadat(0x1234));
    assert!(machine.agnus.set_bltsize(0x41));
    assert!(!machine.agnus.blitter.is_busy);
    assert_eq!(machine.agnus.blitter.bltadat, previous_data);
    assert_eq!(machine.agnus.blitter.bltcon0, 0);
    machine.step_cck();
    assert!(machine.agnus.blitter.is_busy);
    assert_eq!(machine.agnus.blitter.bltadat, 0x1234);
    assert_eq!(machine.agnus.blitter.bltcon0, 0x09f0);
    assert!(machine.agnus.set_bltadat(0x5678));
    machine.step_cck();
    assert_eq!(machine.agnus.blitter.bltcon0, 0x09f0);
    assert_eq!(machine.agnus.blitter.bltadat, 0x5678);
    assert!(machine.agnus.set_bltsize(0x82));
    let active_size = machine.agnus.blitter.bltsize;
    assert_eq!(active_size, 0x41);
    machine.step_cck();
    assert_eq!(machine.agnus.blitter.bltsize, 0x82);
    assert!(machine.agnus.blitter.is_busy);
}
