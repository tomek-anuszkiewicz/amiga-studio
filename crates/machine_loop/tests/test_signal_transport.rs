#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use config::{custom_reg, A500Config, VideoStandard};
use machine_loop::{A500Machine, A500State};

fn idle(standard: VideoStandard) -> A500Machine {
    let mut machine = A500Machine::new(A500Config::bare_512k(standard));
    machine.cpu.state.set_stopped(true);
    machine
}

#[test]
fn legacy_disk_request_reaches_the_current_dma_slot() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = idle(standard);
        machine.agnus.hpos = 6;
        machine.agnus.dskpt = 0x2000;
        machine.agnus.dma.write_dmacon(0x8210);
        machine.paula.dma_active = true;
        machine.paula.dsklen = 0x8001;
        machine.floppy.dskdat = 0x5a42;
        machine.floppy.dma_word_ready = true;
        assert!(machine.paula.set_intena(0xc002));

        // The retained decoded request must arrive before Agnus grants HPOS 7.
        machine.step_subsystems_cck();
        assert_eq!(machine.agnus.hpos, 7);
        assert!(machine.agnus.dsk_dma_req);
        assert_eq!(machine.agnus.dskpt, 0x2002);
        assert_eq!(machine.physical_memory.read_chip_word(0x2000), 0x5a42);
        assert!(!machine.floppy.has_dma_word());
        assert!(!machine.paula.is_dsk_dma_active());
        assert!(!machine.floppy.dskblk_irq);
        assert_eq!(machine.cpu.state.ipl(), 1);

        machine.step_subsystems_cck();
        assert!(!machine.agnus.dsk_dma_req);
        assert_eq!(machine.agnus.dskpt, 0x2002);
    }
}

#[test]
fn legacy_disk_and_audio_events_reach_cpu_ipl_in_the_same_cck_once() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = idle(standard);
        let mask = paula::interrupts::IRQ_DSKSYN
            | paula::interrupts::IRQ_DSKBLK
            | (0..4).fold(0, |mask, ch| mask | paula::interrupts::irq_aud(ch));
        assert!(machine.paula.set_intena(0xc000 | mask));
        assert!(machine.paula.set_intreq(mask));
        machine.floppy.dsksyn_irq = true;
        machine.floppy.dskblk_irq = true;
        for ch in 0..4 {
            machine.agnus.audlc[ch] = 0x3000 + ch as u32 * 0x100;
            machine.paula.audio.channels[ch].restart_strobe = true;
        }

        // Decoded events follow Paula's due clear and precede this CCK's IPL delivery.
        machine.step_subsystems_cck();
        assert_eq!(machine.paula.interrupts.intreq & mask, mask);
        assert_eq!(machine.cpu.state.ipl(), 5);
        assert_eq!(machine.agnus.audpt, machine.agnus.audlc);
        assert!(!machine.floppy.dsksyn_irq);
        assert!(!machine.floppy.dskblk_irq);
        for channel in &machine.paula.audio.channels {
            assert!(!channel.restart_strobe);
        }

        assert!(machine.paula.set_intreq(mask));
        machine.step_subsystems_cck();
        assert_eq!(machine.paula.interrupts.intreq & mask, 0);
        assert_eq!(machine.cpu.state.ipl(), 0);
    }
}

#[test]
fn disconnected_serial_port_preserves_uart_and_ciab_register_logic() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = idle(standard);
        assert_eq!(machine.paula.serdatr(), 0x3800);
        assert!(machine.paula.set_serdat(0x0155));
        assert!(machine.paula.set_serper(0x802a));
        assert!(machine.cia_b.set_ddra(0xc0));
        assert!(machine.cia_b.set_pra(0x80));
        machine.step_cycles(6);
        assert_eq!(machine.paula.serial_port.serdat, 0x0155);
        assert_eq!(machine.paula.serial_port.serper, 0x802a);
        assert_eq!(
            machine.paula.serdatr(),
            0x1800,
            "SERDAT clears TBE, RXD stays high"
        );
        assert_eq!(machine.cia_b.pra() & 0xf8, 0xb8);
        assert!(machine.cia_b.set_pra(0x40));
        machine.step_cycles(6);
        assert_eq!(machine.cia_b.pra() & 0xf8, 0x78);
        assert_eq!(
            machine.paula.serdatr(),
            0x1800,
            "RTS/DTR do not feed back into RXD"
        );

        machine.reset_warm();
        assert_eq!(machine.paula.serdatr(), 0x3800);
        assert_eq!(machine.paula.serial_port.serdat, 0);
        assert_eq!(machine.paula.serial_port.serper, 0);
        assert_eq!(machine.cia_b.pra() & 0x38, 0x38);
    }
}

#[test]
fn ciab_shared_port_and_serial_lines_resolve_both_drivers_before_delivery() {
    // HRM Appendix F connects CIA-B PA0/SP and PA1/CNT on the motherboard.
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        for (sp_high, cnt_high, expected) in [
            (false, true, 0x02),
            (true, false, 0x01),
            (false, false, 0x00),
        ] {
            let mut machine = idle(standard);
            machine.cia_b.cra = 0x40; // Serial output mode; Port A remains input.
            machine.cia_b.pins.sp_output_high = sp_high;
            machine.cia_b.pins.cnt_output_high = cnt_high;
            machine.step_cck();
            assert_eq!(machine.cia_b.pra() & 0x03, 0x03, "still in flight");

            let json = serde_json::to_string(&machine.save_state()).unwrap();
            let state = serde_json::from_str::<A500State>(&json).unwrap();
            let mut restored = idle(standard);
            restored.load_state(&state);
            for target in [&mut machine, &mut restored] {
                // Changing a source after capture cannot replace its incoming level.
                target.cia_b.pins.sp_output_high = true;
                target.cia_b.pins.cnt_output_high = true;
                target.step_cck();
                assert_eq!(target.cia_b.pra() & 0x03, expected);
                assert_eq!(target.cia_b.pins.sp_input_high, sp_high);
                assert_eq!(target.cia_b.pins.cnt_input_high, cnt_high);
                target.step_cck();
                assert_eq!(target.cia_b.pra() & 0x03, 0x03);
            }
            assert_eq!(machine.save_state(), restored.save_state());

            // Port A can also drive the shared lines while serial pins are inputs.
            machine.cia_b.cra = 0;
            machine.cia_b.ddra = 0x03;
            machine.cia_b.pra = expected;
            machine.step_cycles(2);
            assert_eq!(machine.cia_b.pins.sp_input_high, sp_high);
            assert_eq!(machine.cia_b.pins.cnt_input_high, cnt_high);

            // Disabled serial outputs cannot hold the released lines low.
            machine.cia_b.ddra = 0;
            machine.cia_b.pins.sp_output_high = false;
            machine.cia_b.pins.cnt_output_high = false;
            machine.step_cycles(2);
            assert_eq!(machine.cia_b.pra() & 0x03, 0x03);
            assert!(machine.cia_b.pins.sp_input_high);
            assert!(machine.cia_b.pins.cnt_input_high);
            machine.reset_warm();
            assert_eq!(machine.cia_b.pra(), 0xff);
            assert!(machine.cia_b.pins.sp_input_high);
            assert!(machine.cia_b.pins.cnt_input_high);
        }
    }
}

#[test]
fn keyboard_queue_does_not_bypass_serial_input_pins() {
    let mut machine = A500Machine::new(A500Config::bare_512k(VideoStandard::Pal));
    machine.keyboard.enqueue_scancode(0x40);
    machine.step_cck();
    assert_eq!(
        machine.cia_a.sdr(),
        0,
        "a queued device byte is not eight sampled CNT edges"
    );
}

#[test]
fn current_game_port_levels_and_captured_signals_resume_at_their_input_phases() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = idle(standard);
        machine.keyboard.pins.data_high = false;
        machine.keyboard.pins.clock_high = false;
        machine.game_ports.pins.direction_pins = [0x09, 0x06];
        machine.paula.pins.dmal_output_high = true;
        machine.floppy.drives[0].selected = true;
        machine.floppy.pins.index_high = false;
        machine.step_cck();
        // Independent inputs are current; keyboard and DMAL outputs remain in flight.
        assert!(machine.cia_a.pins.sp_input_high);
        assert!(machine.cia_a.pins.cnt_input_high);
        assert_ne!(machine.cia_a.pra() & 0x10, 0, "drive status is in flight");
        assert!(machine.cia_b.pins.flag_high, "INDEX is in flight");
        assert!(!machine.agnus.pins.dmal_input_high);
        assert_eq!(machine.denise.pins.game_port_inputs, [0x09, 0x06]);
        assert!(!machine.cia_a.pins.incoming.as_ref().unwrap().sp_high);
        assert_eq!(machine.agnus.pins.incoming_dmal_high, Some(true));
        assert_eq!(machine.paula.serdatr() & 0x0800, 0x0800);
        let json = serde_json::to_string(&machine.save_state()).unwrap();
        let state = serde_json::from_str::<A500State>(&json).unwrap();
        let mut restored = idle(standard);
        restored.load_state(&state);
        assert_eq!(
            restored.keyboard_reset_asserted,
            machine.keyboard_reset_asserted
        );
        assert!(restored.cia_a.pins.sp_input_high, "load must not deliver");
        for target in [&mut machine, &mut restored] {
            // Change the source after capture. The pending delivery must retain its values.
            target.keyboard.pins.data_high = true;
            target.paula.pins.dmal_output_high = false;
            target.game_ports.pins.direction_pins = [0xfa, 0xf5];
            target.floppy.drives[0].selected = false;
            target.floppy.pins.index_high = true;
            target.step_cck();
            assert!(!target.cia_a.pins.sp_input_high);
            assert!(!target.cia_a.pins.cnt_input_high);
            assert_eq!(target.cia_a.pra() & 0x10, 0);
            assert!(!target.cia_b.pins.flag_high);
            assert_eq!(target.cia_b.icr_debug() & 0x10, 0x10);
            assert!(target.agnus.pins.dmal_input_high);
            assert_eq!(target.denise.pins.game_port_inputs, [0x0a, 0x05]);
            assert_eq!(target.cia_b.pra() & 0x38, 0x38);
            assert_eq!(target.paula.serdatr() & 0x0800, 0x0800);
            assert_eq!(target.cia_a.sdr(), 0, "serial receiver work is deferred");
        }
        assert!(machine.save_state() == restored.save_state());
        machine.step_cck();
        restored.step_cck();
        assert!(machine.cia_a.pins.sp_input_high);
        assert_ne!(machine.cia_a.pra() & 0x10, 0);
        assert!(machine.cia_b.pins.flag_high);
        assert!(!machine.agnus.pins.dmal_input_high);
        assert!(machine.save_state() == restored.save_state());
    }
}

#[test]
fn disconnected_parallel_inputs_preserve_cia_ports_and_index() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = idle(standard);
        machine.cia_a.set_prb(0xa5);
        machine.step_cycles(3);
        assert_eq!(
            machine.cia_a.prb, 0xa5,
            "input levels preserve the output latch"
        );
        assert_eq!(machine.cia_a.prb(), 0xff);
        assert_eq!(machine.cia_b.pra() & 7, 7);
        assert!(machine.cia_a.pins.flag_high);
        assert!(machine.cia_b.pins.sp_input_high);
        assert!(machine.cia_b.pins.cnt_input_high);
        machine.cia_a.set_ddrb(0xff);
        machine.cia_b.set_ddra(3);
        machine.cia_b.set_pra(1);
        machine.step_cycles(6);
        assert_eq!(machine.cia_a.prb(), 0xa5);
        assert!(machine.cia_b.pins.sp_input_high);
        assert!(!machine.cia_b.pins.cnt_input_high);

        machine.floppy.pins.index_high = false;
        machine.cia_a.set_icr(0x90);
        machine.cia_b.set_icr(0x90);
        machine.step_cycles(6);
        assert_eq!(machine.cia_a.icr() & 0x90, 0);
        assert_eq!(machine.cia_b.icr() & 0x90, 0x90);
        machine.step_cycles(3);
        assert_eq!(
            machine.cia_b.icr_debug() & 0x10,
            0,
            "held INDEX is one edge"
        );
        machine.floppy.pins.index_high = true;
        machine.step_cycles(2);
        machine.floppy.pins.index_high = false;
        machine.step_cycles(2);
        assert_eq!(machine.cia_b.icr_debug() & 0x10, 0x10);
        assert_eq!(machine.cia_a.icr_debug() & 0x10, 0);
    }
}

#[test]
fn granted_audio_and_sprite_bus_words_use_receiver_queues_once() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = idle(standard);
        // Deterministic granted RGA/data outputs, not reconstructed DMA scheduling.
        for channel in 0..4 {
            for word in [0x1234, 0x6789] {
                let reg = custom_reg::paula::AUD0DAT + channel as u16 * 16;
                machine.agnus.pending_dma_write = Some((reg, word));
                let before = machine.paula.audio.channels[channel].dat;
                machine.step_cck();
                assert!(machine.agnus.pending_dma_write.is_none());
                assert_eq!(machine.paula.audio.channels[channel].dat, before);
                machine.step_cck();
                assert_eq!(machine.paula.audio.channels[channel].dat, word);
                assert!(machine.paula.mutations.iter().all(Option::is_none));
            }
        }
        for sprite in 0..8 {
            for (offset, value) in [(4, 0x1357), (6, 0x2468)] {
                let reg = custom_reg::denise::SPR0POS + sprite as u16 * 8 + offset;
                machine.agnus.pending_dma_write = Some((reg, value));
                machine.step_cck();
                machine.step_cck();
                let channel = &machine.denise.sprites.channels[sprite];
                assert_eq!(
                    if offset == 4 {
                        channel.data_a
                    } else {
                        channel.data_b
                    },
                    value
                );
            }
        }
    }
}

#[test]
fn refused_dma_output_survives_a_snapshot_until_capacity_returns() {
    let mut machine = idle(VideoStandard::Pal);
    for _ in 0..machine_loop::paula::Paula::MUTATION_CAPACITY {
        assert!(machine.paula.set_auddat(0, 0x1111));
    }
    // Keep the receiving local queue busy across the common issue boundary.
    for mutation in machine.paula.mutations.iter_mut().flatten() {
        mutation.remaining_cck = 5;
    }
    let output = Some((custom_reg::paula::AUD1DAT, 0x4567));
    machine.agnus.pending_dma_write = output;
    machine.step_cck();
    assert_eq!(machine.agnus.pending_dma_write, output);
    assert_eq!(machine.paula.audio.channels[1].dat, 0);
    let state =
        serde_json::from_str::<A500State>(&serde_json::to_string(&machine.save_state()).unwrap())
            .unwrap();
    let mut restored = idle(VideoStandard::Pal);
    restored.load_state(&state);
    for cycle in 1..=6 {
        machine.step_cck();
        restored.step_cck();
        assert!(
            machine.save_state() == restored.save_state(),
            "cycle {cycle}"
        );
        if cycle < 4 {
            assert_eq!(machine.agnus.pending_dma_write, output);
            assert_eq!(machine.paula.audio.channels[1].dat, 0);
        }
    }
    assert!(machine.agnus.pending_dma_write.is_none());
    assert_eq!(machine.paula.audio.channels[1].dat, 0x4567);
    assert!(machine.paula.mutations.iter().all(Option::is_none));
}

#[test]
fn rga_events_arrive_at_receiver_phase_without_copying_agnus_counters() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = idle(standard);
        machine.agnus.vpos = 100;
        machine.denise.sync.vpos = 7;
        machine.denise.sync.hpos = 40;
        machine.denise.sync.synchronized = true;
        machine.paula.sync.cck_since_horizontal = 20;
        machine.step_cck(); // Source HPOS 1: first refresh strobe.
        assert_eq!(
            machine.agnus.pins.rga_strobe,
            Some(custom_reg::denise::STRHOR)
        );
        assert_eq!(machine.denise.sync.vpos, 7);
        assert_eq!(machine.denise.sync.hpos, 41);
        machine.step_cck(); // Retained address reaches both documented recipients.
        assert_eq!(machine.denise.sync.vpos, 8);
        assert_eq!(machine.denise.sync.hpos, 1);
        assert_eq!(machine.paula.sync.cck_since_horizontal, 0);
        assert_ne!(machine.denise.sync.vpos, machine.agnus.vpos);
        machine.step_cck();
        assert_eq!(machine.denise.sync.vpos, 8, "one event must not repeat");
        assert_eq!(machine.paula.sync.cck_since_horizontal, 1);
    }
}

#[test]
fn reset_cancels_pending_connector_and_bus_deliveries() {
    let mut machine = idle(VideoStandard::Pal);
    machine.keyboard.pins.reset_line_asserted = true;
    machine.keyboard.pins.clock_high = false;
    machine.floppy.pins.index_high = false;
    machine.step_cck();
    assert!(machine.keyboard_reset_asserted);
    machine.agnus.pending_dma_write = Some((custom_reg::paula::AUD0DAT, 0xffff));
    machine.reset_warm();
    assert!(!machine.keyboard_reset_asserted);
    machine.cpu.state.set_stopped(true);
    machine.step_cycles(2);
    assert_eq!(
        machine.cck, 2,
        "warm reset cancels the pending keyboard reset"
    );
    assert!(machine.cia_a.pins.flag_high);
    assert!(machine.cia_b.pins.flag_high);
    assert!(machine.cia_a.pins.cnt_input_high);
    assert!(machine.agnus.pending_dma_write.is_none());
    assert_eq!(machine.paula.audio.channels[0].dat, 0);
    assert_eq!(machine.cia_a.icr_debug(), 0);
    assert_eq!(machine.cia_b.icr_debug(), 0);
}

#[test]
fn disk_data_gate_and_controller_output_direction_reach_connector_inputs() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut machine = idle(standard);
        machine.floppy.pins.read_data_high = false;
        machine.paula.pins.disk_write_high = false;
        machine.paula.pins.disk_write_enable = true;
        assert!(machine.cia_a.set_ddra(0xc0));
        assert!(machine.cia_a.set_pra(0x40));
        assert!(machine.paula.set_potgo(0x0f00));
        machine.step_cck();
        assert!(machine.paula.pins.disk_read_high);
        assert!(machine.floppy.pins.write_data_high);
        assert!(machine.floppy.pins.write_enable_high);
        assert_eq!(machine.game_ports.pins.pot_output_mask, 0);
        machine.step_cck();
        assert!(!machine.paula.pins.disk_read_high);
        assert!(!machine.floppy.pins.write_data_high);
        assert!(!machine.floppy.pins.write_enable_high);
        // One CCK to commit POTGO, then one captured connector delivery.
        assert_eq!(machine.game_ports.pins.pot_output_mask, 0x0500);
        assert_eq!(machine.game_ports.pins.fire_output_mask, 0);
        machine.step_cck();
        assert_eq!(machine.game_ports.pins.pot_output_mask, 0x0500);
        assert_eq!(machine.game_ports.pins.pot_output_high, 0x0500);
        assert_eq!(machine.game_ports.pins.fire_output_mask, 0xc0);
        assert_eq!(machine.game_ports.pins.fire_output_high, 0x40);
        assert!(!machine.floppy.drives[0].disk_inserted);
        assert!(!machine.floppy.has_dma_word());
        machine.paula.pins.disk_write_enable = false;
        machine.step_cycles(2);
        assert!(machine.floppy.pins.write_enable_high);
    }
}
