//! Shared guest loading, deterministic connector events and activity observation.
//! This host fixture never writes custom registers or chip internals for the guest.

use config::{A500Config, VideoStandard};
use machine_loop::{agnus::dma::DmaChannel, game_ports::PortDevice, A500Machine};
use serde::Serialize;

pub const PROGRAM: &[u8] = include_bytes!("program.bin");
pub const ENTRY: u32 = 0x1000;
pub const STATS: u32 = 0x8000;
pub const WARMUP_FIELDS: u64 = 32;

pub fn machine(standard: VideoStandard) -> A500Machine {
    let mut machine = A500Machine::new(A500Config::standard_1mb(standard));
    machine.physical_memory.map_chip_ram_to_low_memory();
    machine.cpu.state.set_sr(0x2700);
    machine.cpu.state.set_ssp(0x7f000);
    for (offset, byte) in PROGRAM.iter().enumerate() {
        machine
            .physical_memory
            .write_byte_debug(ENTRY + offset as u32, *byte);
    }
    machine.set_pc_and_prime_prefetch(ENTRY);
    machine
}

fn input(machine: &mut A500Machine, field: u64) {
    let line = machine.agnus.vpos;
    if line % 16 == 0 {
        if let PortDevice::Mouse(mouse) = machine.game_ports.port1_mut() {
            let direction = if field % 64 < 32 { 1 } else { -1 };
            mouse.move_rel(2 * direction, direction);
            mouse.set_left_button(field % 32 < 8);
            mouse.set_right_button(field % 32 >= 24);
        }
    }
    if line == 0 {
        let phase = (field / 8) % 4;
        if let PortDevice::Joystick(joystick) = machine.game_ports.port2_mut() {
            joystick.set_directions(phase == 0, phase == 2, phase == 3, phase == 1);
            joystick.set_fire1(field % 16 < 8);
            joystick.set_fire2(field % 32 < 8);
        }
        match field % 16 {
            0 => machine.keyboard.key_down(0x20),
            8 => machine.keyboard.key_up(0x20),
            _ => {}
        }
    }
}

fn drain_audio(machine: &mut A500Machine) -> u64 {
    let mut nonzero = 0;
    while let Some(sample) = machine.paula.audio.pop_sample() {
        nonzero += u64::from(sample.left != 0 || sample.right != 0);
        std::hint::black_box(sample);
    }
    nonzero
}

/// Normal production stepping, with external events and digital audio consumption.
pub fn run_field(machine: &mut A500Machine, field: u64) {
    loop {
        input(machine, field);
        machine.step_line();
        drain_audio(machine);
        if machine.agnus.vpos == 0 {
            break;
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct Activity {
    pub ccks: u64,
    pub instructions: u64,
    pub refresh: u64,
    pub disk: u64,
    pub audio: [u64; 4],
    pub bitplanes: [u64; 6],
    pub sprites: [u64; 8],
    pub copper: u64,
    pub blitter: u64,
    pub cpu_or_unclaimed: u64,
    pub unassigned: u64,
    pub blitter_busy: u64,
    pub sprite_output_mask: u8,
    pub collisions: u16,
    pub nonzero_audio_samples: u64,
    pub keyboard_edges: u64,
}

/// Detailed observation is intentionally separate from the performance loop.
pub fn audit_field(machine: &mut A500Machine, field: u64, activity: &mut Activity) {
    loop {
        input(machine, field);
        let line = machine.agnus.vpos;
        loop {
            let keyboard_clock = machine.keyboard.pins.clock_high;
            activity.instructions += u64::from(machine.step_cck());
            activity.ccks += 1;
            activity.keyboard_edges +=
                u64::from(keyboard_clock != machine.keyboard.pins.clock_high);
            match machine.agnus.dma.current_owner {
                DmaChannel::Refresh => activity.refresh += 1,
                DmaChannel::Disk => activity.disk += 1,
                DmaChannel::Audio(index) => activity.audio[usize::from(index)] += 1,
                DmaChannel::Bitplane(index) => activity.bitplanes[usize::from(index)] += 1,
                DmaChannel::Sprite(index) => activity.sprites[usize::from(index)] += 1,
                DmaChannel::Copper => activity.copper += 1,
                DmaChannel::Blitter => activity.blitter += 1,
                DmaChannel::Cpu => activity.cpu_or_unclaimed += 1,
                DmaChannel::None => activity.unassigned += 1,
            }
            activity.blitter_busy += u64::from(machine.agnus.blitter.is_busy);
            for (index, channel) in machine.denise.sprites.channels().iter().enumerate() {
                if channel.pixel_counter != 0 {
                    activity.sprite_output_mask |= 1 << index;
                }
            }
            activity.collisions |= machine.denise.clxdat_debug();
            if machine.agnus.vpos != line {
                break;
            }
        }
        activity.nonzero_audio_samples += drain_audio(machine);
        if machine.agnus.vpos == 0 {
            break;
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct GuestProgress {
    pub fields: u32,
    pub work_loops: u32,
    pub blits_started: u32,
    pub keyboard_bytes: u32,
    pub cia_interrupts: u32,
    pub audio_interrupts: u32,
    pub collision_union: u16,
    pub mouse: u16,
    pub joystick: u16,
    pub fault: u32,
}

pub fn progress(machine: &A500Machine) -> GuestProgress {
    let word = |offset| machine.physical_memory.read_word_debug(STATS + offset);
    let long = |offset| (u32::from(word(offset)) << 16) | u32::from(word(offset + 2));
    GuestProgress {
        fields: long(0),
        work_loops: long(4),
        blits_started: long(8),
        keyboard_bytes: long(12),
        cia_interrupts: long(16),
        audio_interrupts: long(20),
        collision_union: word(24),
        mouse: word(26),
        joystick: word(28),
        fault: long(36),
    }
}
