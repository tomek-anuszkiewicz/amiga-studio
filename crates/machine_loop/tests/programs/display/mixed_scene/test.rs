use crate::common::MachineHarness;

#[path = "host.rs"]
mod scene;

use config::VideoStandard;

#[test]
fn mixed_guest_centers_256_pal_lines_inside_full_width_raster_bars() {
    let mut machine = scene::machine(VideoStandard::Pal);
    for field in 0..scene::WARMUP_FIELDS {
        scene::run_field(&mut machine, field);
    }
    assert_eq!(machine.denise.diwstrt, 0x2881);
    assert_eq!(machine.denise.diwstop, 0x28c1);

    // The palette repeats every eight lines, including across the Copper's
    // eight-bit vertical comparison wrap. Sample after each line's transfers.
    for line in 0..312u16 {
        assert_eq!(machine.agnus.vpos, line);
        machine.step_line();
        if line >= 26 {
            let expected = ((line - 26) % 8) * 0x11;
            assert_eq!(machine.denise.color[0], expected, "raster line {line}");
        }
    }

    let frame = &machine.denise.frame_builder;
    assert_eq!([frame.width, frame.height], [836, 570]);
    for line in 26..311 {
        let channel = ((line - 26) % 8) as u32 * 17;
        let border = 0xff00_0000 | (channel << 8) | channel;
        for y in (line - 26) * 2..(line - 26) * 2 + 2 {
            for x in 0..836 {
                if !(40..296).contains(&line) || !(112..752).contains(&x) {
                    assert_eq!(frame.get_pixel(x, y), border, "border pixel {x},{y}");
                }
            }
        }
    }
    // Fourteen lines above and fifteen below retain the full 256-line scene.
    for line in [40, 244, 255, 256, 295] {
        let y = (line - 26) * 2;
        let first = frame.get_pixel(200, y);
        assert!(
            (201..700).any(|x| frame.get_pixel(x, y) != first),
            "missing playfield on raster line {line}"
        );
    }
}

#[test]
fn mixed_guest_exercises_cpu_dma_and_input_together() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let mut harness = MachineHarness {
            machine: scene::machine(standard),
        };
        let machine = &mut harness.machine;
        for field in 0..scene::WARMUP_FIELDS {
            scene::run_field(machine, field);
        }
        let before = scene::progress(machine);
        let initial = machine.save_state();
        let image_before = machine.denise.frame_builder.frame_buffer().to_vec();
        let mut activity = scene::Activity::default();
        for field in scene::WARMUP_FIELDS..scene::WARMUP_FIELDS + 16 {
            scene::audit_field(machine, field, &mut activity);
        }
        let after = scene::progress(machine);
        assert_eq!(after.fault, 0, "guest exception: {after:?}");
        assert!(after.fields > before.fields && after.work_loops > before.work_loops);
        assert!(after.blits_started > before.blits_started);
        assert!(after.keyboard_bytes > before.keyboard_bytes);
        assert!(after.cia_interrupts > before.cia_interrupts);
        assert!(after.audio_interrupts > before.audio_interrupts);
        assert!(activity.instructions > 0 && activity.copper > 0 && activity.blitter > 0);
        assert!(activity.bitplanes[..5].iter().all(|count| *count > 0));
        assert!(activity.sprites.iter().all(|count| *count > 0));
        assert!(activity.audio.iter().all(|count| *count > 0));
        assert_eq!(activity.sprite_output_mask, 0xff);
        assert_ne!(
            activity.collisions & 0x7e00,
            0,
            "no sprite/sprite collision"
        );
        assert_ne!(
            activity.collisions & 0x01fe,
            0,
            "no sprite/playfield collision"
        );
        assert!(activity.nonzero_audio_samples > 0 && activity.keyboard_edges > 0);
        assert_ne!(after.mouse, before.mouse);
        assert_ne!(after.joystick, before.joystick);
        assert_eq!(activity.disk, 0);
        assert!(!machine.cpu.state.is_stopped() && !machine.cpu.state.is_halted());
        assert_ne!(machine.denise.frame_builder.frame_buffer(), image_before);

        // Restoring the initial state and using the uninstrumented runner must
        // preserve the complete result, including inputs, audio and video.
        let expected = machine.save_state();
        machine.load_state(&initial);
        for field in scene::WARMUP_FIELDS..scene::WARMUP_FIELDS + 16 {
            scene::run_field(machine, field);
        }
        assert!(
            machine.save_state() == expected,
            "audit changed execution state"
        );
    }
}
