#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use config::{A500Config, VideoStandard};
use machine_loop::{paula::audio::AudioState, A500Machine};

fn playing(standard: VideoStandard) -> A500Machine {
    let mut machine = A500Machine::new(A500Config::bare_512k(standard));
    machine.cpu.state.set_stopped(true);
    machine.agnus.vpos = 100;
    for channel in 0..4usize {
        let address = 0x1000 + channel * 16;
        let word = 0x1234 + channel as u16 * 0x1010;
        machine.physical_memory.chip_ram[address..address + 2].copy_from_slice(&word.to_be_bytes());
        machine.physical_memory.chip_ram[address + 2..address + 4]
            .copy_from_slice(&0x7f80u16.to_be_bytes());
        let register = 0x00a0 + channel as u16 * 16;
        let mut bus = machine.memory_bus();
        for (offset, value) in [(0, 0), (2, address as u16), (4, 2), (6, 400), (8, 64)] {
            assert!(bus.write_custom_word(register + offset, value));
        }
    }
    assert!(machine.memory_bus().write_custom_word(0x0096, 0x820f));
    machine
}

#[test]
fn four_serial_request_pairs_supply_real_words_and_restart_addresses() {
    for standard in [VideoStandard::Pal, VideoStandard::Ntsc] {
        let transitions = match standard {
            VideoStandard::Pal => [0, 6, 24, 100],
            VideoStandard::Ntsc => [0, 8, 19, 100],
        };
        for start_line in transitions {
            let mut machine = playing(standard);
            machine.agnus.vpos = start_line;
            let mut grants = [0; 4];
            for _ in 0..700 {
                machine.step_cck();
                if let machine_loop::agnus::dma::DmaChannel::Audio(channel) =
                    machine.agnus.dma.current_owner
                {
                    let channel = usize::from(channel);
                    grants[channel] += 1;
                    assert_eq!(machine.agnus.hpos, 13 + channel as u16 * 2);
                    assert_eq!(machine.agnus.audpt[channel], 0x1002 + channel as u32 * 16);
                    assert!(!machine.agnus.audio_dma_request[channel]);
                }
                if grants == [2; 4] {
                    machine.step_cck();
                    break;
                }
            }
            assert_eq!(grants, [2; 4]);
            for channel in 0..4 {
                let audio = &machine.paula.audio.channels[channel];
                assert_eq!(audio.state, AudioState::HighByte);
                assert_eq!(audio.buffer, 0x1234 + channel as u16 * 0x1010);
                assert!(audio.dma_request);
            }
            assert_eq!(machine.paula.interrupts.intreq & 0x0780, 0x0780);
            // Updating the next block's location must not redirect an active fetch.
            assert!(machine.memory_bus().write_custom_word(0x00a2, 0x3000));
            machine.step_cycles(2);
            assert_eq!(machine.agnus.audlc[0], 0x3000);
            assert_eq!(machine.agnus.audpt[0], 0x1002);
        }
    }
}

#[test]
fn serialized_partial_request_and_unread_sound_resume_identically() {
    let mut machine = playing(VideoStandard::Pal);
    for _ in 0..1000 {
        machine.step_cck();
        if machine.agnus.vpos == 102 && machine.agnus.hpos == 10 {
            break;
        }
    }
    assert_eq!(machine.agnus.hpos, 10);
    assert!(machine.agnus.audio_dma_request[0]);
    assert!(machine.agnus.audio_dma_restart[0]);
    let json = serde_json::to_string(&machine.save_state()).unwrap();
    let state = serde_json::from_str(&json).unwrap();
    let mut restored = playing(VideoStandard::Pal);
    restored.load_state(&state);
    assert_eq!(machine.paula.audio, restored.paula.audio);
    for _ in 0..450 {
        machine.step_cck();
        restored.step_cck();
        assert_eq!(machine.paula.audio, restored.paula.audio);
        assert_eq!(
            machine.agnus.audio_dma_request,
            restored.agnus.audio_dma_request
        );
        assert_eq!(machine.agnus.audpt, restored.agnus.audpt);
    }
    let mut samples = vec![machine_loop::paula::audio::StereoSample::default(); 1024];
    let mut observed = machine.paula.audio.clone();
    let count = observed.read_samples(&mut samples);
    assert!(samples[..count]
        .iter()
        .any(|sample| sample.left != 0 || sample.right != 0));
    assert_eq!(machine.save_state(), restored.save_state());
    restored.reset_warm();
    assert_eq!(restored.agnus.audio_dma_request, [false; 4]);
    assert_eq!(restored.agnus.audio_dma_restart, [false; 4]);
    assert_eq!(restored.paula.audio.samples_available(), 0);
}
