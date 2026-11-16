//! Explicit local-asset boot/save observations; not a physical input certification.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use machine_loop::A500Machine;
use save_state::SaveState;

#[test]
#[ignore = "requires AMIGA_KICKSTART_ROM, AMIGA_GUEST_ADF and AMIGA_GUEST_LABEL"]
fn private_guest_boot_and_save_continuation() {
    let rom = std::fs::read(std::env::var("AMIGA_KICKSTART_ROM").expect("set AMIGA_KICKSTART_ROM"))
        .unwrap();
    let disk =
        std::fs::read(std::env::var("AMIGA_GUEST_ADF").expect("set AMIGA_GUEST_ADF")).unwrap();
    let label = std::env::var("AMIGA_GUEST_LABEL").expect("set AMIGA_GUEST_LABEL");
    assert!(label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
    let seconds: u64 = std::env::var("AMIGA_GUEST_SECONDS")
        .unwrap_or_else(|_| "60".into())
        .parse()
        .unwrap();
    let mut machine = A500Machine::new(config::A500Config::default());
    machine
        .physical_memory
        .write_bytes_debug(config::address_map::KICKSTART_BASE, &rom)
        .unwrap();
    machine.reset();
    assert!(machine.floppy.drives[0].insert_disk(&disk));
    let hz = u64::from(machine.config.video_standard().color_clock_hz());
    let output = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.agent/tmp/player-guests")
        .join(label);
    std::fs::create_dir_all(&output).unwrap();
    let mut continued = false;
    let mut dma_seen = false;
    for tick in 0..seconds * hz {
        machine.step_cck();
        assert!(
            !machine.cpu.state.is_halted(),
            "CPU halted at CCK {}",
            machine.cck
        );
        dma_seen |= machine.paula.dma_active;
        if !continued && machine.cck > hz && machine.paula.dma_active {
            let encoded = SaveState::capture(&machine).to_json().unwrap();
            let saved = SaveState::from_json(&encoded).unwrap();
            let mut restored = A500Machine::new(config::A500Config::default());
            restored.load_state(&saved.state);
            let mut reference = machine.clone();
            for _ in 0..20_000 {
                assert_eq!(reference.step_cck(), restored.step_cck());
            }
            assert_eq!(
                reference.save_state(),
                restored.save_state(),
                "real guest continuation diverged"
            );
            continued = true;
            std::fs::write(output.join("during-dma.json"), encoded).unwrap();
        }
        if tick % hz == 0 {
            eprintln!(
                "guest t={} PC={:06x} DMA={} fields={}",
                tick / hz,
                machine.cpu.state.instruction_pc(),
                machine.paula.dma_active,
                machine.denise.frame_builder.frame_number
            );
        }
    }
    assert!(dma_seen && continued);
    assert!(machine.denise.frame_builder.frame_number > 0);
    let mut saved = SaveState::capture(&machine);
    saved.header.timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    std::fs::write(output.join("final.png"), &saved.header.screenshot_png).unwrap();
    std::fs::write(output.join("final.json"), saved.to_json().unwrap()).unwrap();
    eprintln!("Observed guest and exact 20000-CCK continuation during DMA; final CCK {}. Appearance and completed startup require separate inspection.",machine.cck);
}
