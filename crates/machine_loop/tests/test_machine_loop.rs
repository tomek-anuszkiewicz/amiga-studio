use config::A500Config;
use machine_loop::{A500Machine, AddressBus};

#[test]
fn test_machine_creation_and_stepping() {
    let config = A500Config::default();
    let mut machine = A500Machine::new(config);

    assert_eq!(machine.cck, 0);
    assert_eq!(machine.resolve_ipl(), 0);

    // Step 10 Color Clocks
    machine.step_cycles(10);
    assert_eq!(machine.cck, 10);
}

#[test]
fn test_machine_interrupt_arbitration() {
    let config = A500Config::default();
    let mut machine = A500Machine::new(config);

    assert_eq!(machine.resolve_ipl(), 0);

    // Trigger CIA-A interrupt -> IPL 2
    machine.cia_a.write_register(0xD, 0x81); // Enable Timer A IRQ
    machine.cia_a.write_register(0x4, 1);
    machine.cia_a.write_register(0x5, 0);
    machine.cia_a.write_register(0xE, 0x01); // Start Timer A
    for _ in 0..10 {
        machine.cia_a.step_cck();
    }
    assert!(machine.cia_a.irq_pending());
    assert_eq!(machine.resolve_ipl(), 2);

    // Trigger CIA-B interrupt -> IPL 6 (higher priority than Level 2)
    machine.cia_b.write_register(0xD, 0x81);
    machine.cia_b.write_register(0x4, 1);
    machine.cia_b.write_register(0x5, 0);
    machine.cia_b.write_register(0xE, 0x01);
    for _ in 0..10 {
        machine.cia_b.step_cck();
    }
    assert!(machine.cia_b.irq_pending());
    assert_eq!(machine.resolve_ipl(), 6);
}

#[test]
fn test_machine_cold_and_warm_reset() {
    let config = A500Config::default();
    let mut machine = A500Machine::new(config);

    // 1. Simulate running machine: disengage boot overlay, alter RAM, advance clocks, alter chip registers
    machine.physical_memory.map_chip_ram_to_low_memory();
    machine.step_cycles(500);
    assert_eq!(machine.cck, 500);
    machine.agnus.dma.write_dmacon(0x8200); // Enable DMA
    machine.paula.write_intena(0x8080); // Enable Audio 0 IRQ
    let _ = machine.memory_bus().write_byte(0x001000, 0x42);
    assert_eq!(machine.physical_memory.read_byte_debug(0x001000), 0x42);

    // 2. Perform warm reset: preserves RAM, re-engages boot overlay, resets CCK to 0, resets chips
    machine.reset_warm();
    assert_eq!(machine.cck, 0);
    assert_eq!(machine.agnus.dma.dmacon, 0x0000);
    assert_eq!(machine.paula.intena, 0x0000);
    assert!(machine.physical_memory.is_low_memory_overlay_active());
    // Disengage overlay to inspect physical Chip RAM
    machine.physical_memory.map_chip_ram_to_low_memory();
    assert_eq!(machine.physical_memory.read_byte_debug(0x001000), 0x42); // Preserved!

    // 3. Perform cold reset: zeroes RAM, re-engages boot overlay, resets CCK to 0, resets chips
    machine.reset_cold();
    assert_eq!(machine.cck, 0);
    assert_eq!(machine.agnus.dma.dmacon, 0x0000);
    assert_eq!(machine.paula.intena, 0x0000);
    assert!(machine.physical_memory.is_low_memory_overlay_active());
    machine.physical_memory.map_chip_ram_to_low_memory();
    assert_eq!(machine.physical_memory.read_byte_debug(0x001000), 0x00); // Zeroed!
}

#[test]
fn test_machine_game_ports_routing() {
    let config = A500Config::default();
    let mut machine = A500Machine::new(config);

    // Initial state: Port 1 = Mouse, Port 2 = Joystick
    assert!(!machine.game_ports.fire1_port1());
    assert!(!machine.game_ports.fire1_port2());

    // Host input events routed via machine forwarders
    machine.set_mouse_buttons(true, false, true);
    assert!(machine.game_ports.fire1_port1());
    assert_eq!(machine.game_ports.potgor(0x0000) & (1 << 8), 0); // Middle button pulled low

    machine.set_joystick(true, false, false, false, true, false);
    assert!(machine.game_ports.fire1_port2());
    assert_eq!(machine.game_ports.joy1dat(), 0x0100); // Up direction set in bit 8

    // Reset restores defaults
    machine.reset_cold();
    assert!(!machine.game_ports.fire1_port1());
    assert!(!machine.game_ports.fire1_port2());
}

#[test]
fn test_machine_step_frame() {
    let config = A500Config::default();
    let mut machine = A500Machine::new(config);

    assert_eq!(machine.cck, 0);
    machine.step_frame();

    // Standard PAL frame has 312 lines of 227 CCKs = 70,824 CCKs
    assert_eq!(machine.cck, 70_824);
}
