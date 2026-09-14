use config::A500Config;
use machine_loop::A500Machine;
use physical_memory::BusResult;

#[test]
fn test_fixed_dma_slot_cpu_wait_states() {
    let mut machine = A500Machine::new(A500Config::default());
    machine.reset_cold();

    // 1. Refresh Slots (HPOS 0..3) unconditionally block Chip RAM & Slow RAM
    for target_hpos in 1..=3 {
        machine.agnus.hpos = target_hpos - 1;
        machine.step_subsystems_cck();
        assert_eq!(machine.agnus.hpos, target_hpos);

        assert!(
            machine.physical_memory.chip_ram_blocked,
            "HPOS {} must be blocked by DRAM Refresh",
            target_hpos
        );
        assert_eq!(
            machine.physical_memory.read_word(0x001000),
            BusResult::WaitState
        );
        assert_eq!(
            machine.physical_memory.write_word(0x001000, 0x1234),
            BusResult::WaitState
        );
        assert_eq!(
            machine.physical_memory.read_word(0xC00000),
            BusResult::WaitState
        );
    }

    // 2. Idle slot (HPOS 50, outside fixed DMA and no bitplanes active)
    machine.agnus.hpos = 49;
    machine.step_subsystems_cck();
    assert_eq!(machine.agnus.hpos, 50);
    assert!(!machine.physical_memory.chip_ram_blocked);
    assert!(matches!(
        machine.physical_memory.read_word(0x001000),
        BusResult::Ready(_)
    ));
    assert_eq!(
        machine.physical_memory.write_word(0x001000, 0x5678),
        BusResult::Ready(())
    );
}

#[test]
fn test_fast_ram_immunity_under_heavy_dma() {
    // Configure ExpandedPowerUser with 4 MB Fast RAM at $200000..$5FFFFF
    let mut machine = A500Machine::new(A500Config::expanded_power_user(config::VideoStandard::Pal));
    machine.reset_cold();

    // Write a test value to Fast RAM
    assert_eq!(
        machine.physical_memory.write_word(0x200000, 0xABCD),
        BusResult::Ready(())
    );

    // Assert heavy DMA contention: Blitter Nasty + Refresh at HPOS 0
    machine
        .agnus
        .dma
        .write_dmacon(0x8000 | 0x0200 | 0x0400 | 0x0040); // DMAEN | BLTPRI | BLTEN
    machine.agnus.blitter.is_busy = true;
    machine.agnus.hpos = 0; // Steps to HPOS 1 (Refresh)
    machine.step_subsystems_cck();

    assert!(machine.physical_memory.chip_ram_blocked);

    // Chip RAM and Slow RAM stall with WaitState
    assert_eq!(
        machine.physical_memory.read_word(0x001000),
        BusResult::WaitState
    );
    assert_eq!(
        machine.physical_memory.read_word(0xC00000),
        BusResult::WaitState
    );

    // Fast RAM is 100% IMMUNE: reads and writes succeed immediately without wait states!
    assert_eq!(
        machine.physical_memory.read_word(0x200000),
        BusResult::Ready(0xABCD)
    );
    assert_eq!(
        machine.physical_memory.write_word(0x200000, 0x9876),
        BusResult::Ready(())
    );
    assert_eq!(
        machine.physical_memory.read_word(0x200000),
        BusResult::Ready(0x9876)
    );
}

#[test]
fn test_bitplane_contention_scaling() {
    let mut machine = A500Machine::new(A500Config::default());
    machine.reset_cold();

    // Enable DMA + Bitplanes: $8000 | 0x0200 | 0x0100 = $8300
    machine.agnus.dma.write_dmacon(0x8300);
    machine.agnus.dma.set_ddfstrt(0x0038);
    machine.agnus.dma.set_ddfstop(0x00D0);
    machine.agnus.vpos = 100; // Active display scanline

    let base = 0x0038;

    // Case 1: LoRes 4 Planes (BPLCON0 = 0x4000)
    machine.agnus.set_bplcon0(0x4000);

    // Even phase 0: Plane 1 fetches -> Chip RAM blocked
    machine.agnus.hpos = base - 1;
    machine.step_subsystems_cck();
    assert_eq!(machine.agnus.hpos, base);
    assert!(machine.physical_memory.chip_ram_blocked);

    // Odd phase 1: Free for CPU -> zero wait states
    machine.agnus.hpos = base + 1 - 1;
    machine.step_subsystems_cck();
    assert_eq!(machine.agnus.hpos, base + 1);
    assert!(!machine.physical_memory.chip_ram_blocked);

    // Case 2: LoRes 6 Planes (BPLCON0 = 0x6000) -> 50% odd cycle stealing
    machine.agnus.set_bplcon0(0x6000);

    // Odd phase 1: Plane 5 steals cycle -> Chip RAM blocked!
    machine.agnus.hpos = base + 1 - 1;
    machine.step_subsystems_cck();
    assert_eq!(machine.agnus.hpos, base + 1);
    assert!(machine.physical_memory.chip_ram_blocked);

    // Odd phase 3: Plane 6 steals cycle -> Chip RAM blocked!
    machine.agnus.hpos = base + 3 - 1;
    machine.step_subsystems_cck();
    assert_eq!(machine.agnus.hpos, base + 3);
    assert!(machine.physical_memory.chip_ram_blocked);

    // Odd phase 5: Free for CPU -> zero wait states
    machine.agnus.hpos = base + 5 - 1;
    machine.step_subsystems_cck();
    assert_eq!(machine.agnus.hpos, base + 5);
    assert!(!machine.physical_memory.chip_ram_blocked);

    // Case 3: HiRes 4 Planes (BPLCON0 = 0xC000) -> 100% CPU lock-out in DDF window
    machine.agnus.set_bplcon0(0xC000);
    for phase in 0..8 {
        machine.agnus.hpos = base + phase - 1;
        machine.step_subsystems_cck();
        assert_eq!(machine.agnus.hpos, base + phase);
        assert!(
            machine.physical_memory.chip_ram_blocked,
            "HiRes 4 planes: phase {} must lock out the CPU",
            phase
        );
    }
}

#[test]
fn test_blitter_nasty_cpu_lockout_integration() {
    let mut machine = A500Machine::new(A500Config::default());
    machine.reset_cold();

    // Enable DMA + Blitter Nasty: DMAEN (bit 9) | BLTPRI (bit 10) | BLTEN (bit 6)
    machine
        .agnus
        .dma
        .write_dmacon(0x8000 | 0x0200 | 0x0400 | 0x0040);
    machine.agnus.blitter.is_busy = true;

    // Across 10 consecutive cycles at idle HPOS 50, Chip RAM remains continuously blocked
    for _ in 0..10 {
        machine.agnus.hpos = 50;
        machine.step_subsystems_cck();
        assert!(machine.physical_memory.chip_ram_blocked);
        assert_eq!(
            machine.physical_memory.read_word(0x001000),
            BusResult::WaitState
        );
    }
}

#[test]
fn test_cpu_three_cycle_starvation_yield_integration() {
    let mut machine = A500Machine::new(A500Config::default());
    machine.reset_cold();

    // Enable DMA + Normal Blitter (BLTPRI == 0): DMAEN (bit 9) | BLTEN (bit 6)
    machine.agnus.dma.write_dmacon(0x8000 | 0x0200 | 0x0040);
    machine.agnus.blitter.is_busy = true;
    machine.agnus.hpos = 50;

    // Cycles 1..3: Blitter takes the bus, starving the CPU
    for i in 1..=3 {
        machine.step_subsystems_cck();
        assert!(machine.physical_memory.chip_ram_blocked);
        assert_eq!(machine.agnus.dma.cpu_starvation_counter, i);
        assert_eq!(
            machine.physical_memory.read_word(0x001000),
            BusResult::WaitState
        );
    }

    // Cycle 4: Agnus 3-cycle CPU Starvation Yield!
    // Arbiter forces Blitter to yield 1 cycle to CPU!
    machine.step_subsystems_cck();
    assert!(!machine.physical_memory.chip_ram_blocked);
    assert_eq!(machine.agnus.dma.cpu_starvation_counter, 0);
    assert!(matches!(
        machine.physical_memory.read_word(0x001000),
        BusResult::Ready(_)
    ));
}
