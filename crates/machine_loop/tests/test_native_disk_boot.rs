//! Explicit private-ROM native boot checks. No guest RAM injection or OS substitutes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use agnus::copper::CopperState;
use config::{A500Config, VideoStandard};
use machine_loop::A500Machine;
use std::path::PathBuf;
const SECOND: u64 = VideoStandard::Pal.color_clock_hz() as u64;
// The original bootblock reads offset $400, length $400 to $70000 and JMPs there.
const PAYLOAD: u32 = 0x70000;
const BODY: u32 = PAYLOAD + 1462 - 1024;
const COPPER: u32 = PAYLOAD + 1556 - 1024;
const LIST_END: usize = 0x77c;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn checksum(bytes: &[u8]) -> u32 {
    bytes.chunks_exact(4).fold(0u32, |sum, bytes| {
        let (result, carry) = sum.overflowing_add(u32::from_be_bytes(bytes.try_into().unwrap()));
        result.wrapping_add(u32::from(carry))
    })
}
fn machine(disk: Option<&[u8]>) -> A500Machine {
    let rom = std::fs::read(std::env::var("AMIGA_KICKSTART_ROM").expect("set AMIGA_KICKSTART_ROM"))
        .unwrap();
    assert_eq!(rom.len(), 262_144);
    assert_eq!(checksum(&rom), u32::MAX);
    assert_eq!(
        &rom[12..16],
        &[0, 34, 0, 5],
        "requires Kickstart 1.3 revision 34.5"
    );
    let mut m = A500Machine::new(A500Config::standard_1mb(VideoStandard::Pal));
    m.physical_memory
        .write_bytes_debug(config::address_map::KICKSTART_BASE, &rom)
        .unwrap();
    m.reset();
    assert_eq!(
        m.cpu.state.instruction_pc(),
        u32::from_be_bytes(rom[4..8].try_into().unwrap())
    );
    assert!(m.physical_memory.low_memory_overlay);
    if let Some(disk) = disk {
        assert_eq!(checksum(&disk[..1024]), u32::MAX);
        assert!(m.floppy.drives[0].insert_disk(disk));
        m.floppy.drives[0].write_protected = true;
    }
    m
}
fn bytes_at(m: &A500Machine, address: u32, bytes: &[u8]) -> bool {
    bytes
        .iter()
        .enumerate()
        .all(|(i, v)| m.physical_memory.read_byte_debug(address + i as u32) == *v)
}
fn cross1(disk: &[u8]) -> u64 {
    let mut m = machine(Some(disk));
    let mut stages = [None; 5];
    let mut pointer = 0u32;
    let mut previous_copper = (0, 0, 0);
    let mut colors = Vec::new();
    let mut committed_colors = Vec::new();
    let mut previous_color = 0;
    let mut loop_retirements = 0;
    let mut fields = Vec::new();
    let mut previous_vpos = m.agnus.vpos;
    for _ in 0..30 * SECOND {
        let retired = m.step_cck();
        assert!(!m.cpu.state.is_halted(), "CPU halted at CCK {}", m.cck);
        if !m.physical_memory.low_memory_overlay {
            stages[0].get_or_insert(m.cck);
        }
        if m.agnus.dskpt == pointer.wrapping_add(2) {
            stages[1].get_or_insert(m.cck);
        }
        pointer = m.agnus.dskpt;
        let pc = m.cpu.state.instruction_pc();
        if stages[2].is_none() && pc < 0x80000 && bytes_at(&m, pc, &disk[12..28]) {
            stages[2] = Some(m.cck);
        }
        if pc == PAYLOAD && bytes_at(&m, pc, &disk[1024..1040]) {
            stages[3].get_or_insert(m.cck);
        }
        if pc == BODY && bytes_at(&m, pc, &disk[1462..1556]) {
            stages[4].get_or_insert(m.cck);
        }
        if let Some(body_cck) = stages[4] {
            assert_eq!(m.agnus.copper.cop1lc, COPPER);
            if retired {
                // Original ministartup NewVBI saves registers and acknowledges VERTB.
                assert!(
                    pc == BODY || (0x7012c..=0x7014c).contains(&pc),
                    "unexpected post-setup PC {pc:06x}"
                );
                if pc == BODY {
                    loop_retirements += 1;
                }
            }
            if m.agnus.vpos < previous_vpos {
                fields.push(m.cck);
            }
            let c = &m.agnus.copper;
            let current = (c.cop_pc, c.ir1, c.ir2);
            if current != previous_copper && c.ir1 == 0x180 && c.state == CopperState::FetchIR1 {
                colors.push((c.cop_pc - 4, c.ir2));
            }
            previous_copper = current;
            if m.denise.color[0] != previous_color {
                previous_color = m.denise.color[0];
                committed_colors.push(previous_color);
            }
            if m.cck - body_cck > SECOND / 10 {
                break;
            }
        }
        previous_vpos = m.agnus.vpos;
    }
    eprintln!("cross1 [overlay, DMA, bootblock, startup, body]={stages:?}; fields={fields:?}; loop retirements={loop_retirements}");
    assert!(
        stages.iter().all(Option::is_some),
        "missing native startup stage"
    );
    assert!(stages.windows(2).all(|s| s[0] <= s[1]));
    assert!(fields.len() >= 3 && loop_retirements > 100);
    assert!(bytes_at(&m, COPPER, &disk[1556..LIST_END]));
    let expected: Vec<_> = disk[1556..LIST_END]
        .chunks_exact(4)
        .enumerate()
        .filter_map(|(i, b)| {
            (b[..2] == [1, 0x80])
                .then_some((COPPER + i as u32 * 4, u16::from_be_bytes([b[2], b[3]])))
        })
        .collect();
    assert_eq!(expected.len(), 83);
    assert_eq!(expected[1].1, 0xf0f);
    assert_eq!(expected[81].1, 0xfff);
    assert_eq!(expected[82].1, 0xf0f);
    assert!(colors.len() >= expected.len() * 2);
    for field in colors.chunks_exact(expected.len()) {
        assert_eq!(field, expected);
    }
    let expected_commits: Vec<_> = expected
        .iter()
        .map(|(_, v)| *v)
        .cycle()
        .skip(1)
        .take(committed_colors.len())
        .collect();
    assert_eq!(committed_colors, expected_commits);
    assert!(committed_colors.len() > 160);
    stages[4].unwrap()
}
#[test]
#[ignore = "explicit native boot gate requiring AMIGA_KICKSTART_ROM"]
fn cross1_boots_from_kickstart_twice_and_no_disk_cannot_produce_its_signature() {
    let disk = std::fs::read(root().join("ref_src/vAmigaTS/Agnus/Copper/cross/cross1/cross1.adf"))
        .unwrap();
    assert_eq!(&disk[12..18], &[0x4b, 0xf9, 0, 7, 0, 0]);
    assert_eq!(&disk[1462..1464], &[0x60, 0xfe]);
    assert_eq!(cross1(&disk), cross1(&disk));
    let mut control = machine(None);
    for _ in 0..30 * SECOND {
        control.step_cck();
        assert!(!control.cpu.state.is_halted());
        assert!(
            !(control.cpu.state.instruction_pc() == BODY
                && bytes_at(&control, BODY, &disk[1462..1556]))
        );
    }
    eprintln!(
        "no-disk control: {} CCK, PC={:06x}, cross1 absent",
        control.cck,
        control.cpu.state.instruction_pc()
    );
}
fn long(m: &A500Machine, a: u32) -> u32 {
    (u32::from(m.physical_memory.read_word_debug(a)) << 16)
        | u32::from(m.physical_memory.read_word_debug(a.wrapping_add(2)))
}
fn name(m: &A500Machine, a: u32) -> String {
    (0..128)
        .map(|i| m.physical_memory.read_byte_debug(a.wrapping_add(i)))
        .take_while(|b| *b != 0)
        .map(char::from)
        .collect()
}
fn list_names(m: &A500Machine, list: u32) -> Vec<String> {
    let mut names = Vec::new();
    let mut node = long(m, list);
    for _ in 0..64 {
        if node == 0 || long(m, node) == 0 {
            break;
        }
        names.push(name(m, long(m, node.wrapping_add(10))));
        node = long(m, node);
    }
    names
}
#[test]
#[ignore = "separate attempt requiring AMIGA_KICKSTART_ROM and AMIGA_WORKBENCH_ADF"]
fn workbench13_startup_attempt() {
    let disk =
        std::fs::read(std::env::var("AMIGA_WORKBENCH_ADF").expect("set AMIGA_WORKBENCH_ADF"))
            .unwrap();
    let mut m = machine(Some(&disk));
    let mut stages = [None; 4];
    for tick in 0..60 * SECOND {
        m.step_cck();
        assert!(!m.cpu.state.is_halted());
        if !m.physical_memory.low_memory_overlay {
            stages[0].get_or_insert(m.cck);
        }
        let pc = m.cpu.state.instruction_pc();
        if stages[1].is_none() && pc < 0x80000 && bytes_at(&m, pc, &disk[12..28]) {
            stages[1] = Some(m.cck);
        }
        if tick % (SECOND / 10) == 0 {
            let exec = long(&m, 4);
            let libraries = list_names(&m, exec.wrapping_add(378));
            let mut tasks = list_names(&m, exec.wrapping_add(406));
            tasks.extend(list_names(&m, exec.wrapping_add(420)));
            tasks.push(name(
                &m,
                long(&m, long(&m, exec.wrapping_add(276)).wrapping_add(10)),
            ));
            if libraries.iter().any(|n| n == "dos.library") {
                stages[2].get_or_insert(m.cck);
            }
            if tasks.iter().any(|n| n == "Workbench") {
                stages[3].get_or_insert(m.cck);
            }
            if tick % SECOND == 0 {
                eprintln!("Workbench t={} pc={pc:06x} tasks={tasks:?}", tick / SECOND);
            }
        }
    }
    let output = root().join(".agent/tmp/native-disk");
    std::fs::create_dir_all(&output).unwrap();
    let f = &m.denise.frame_builder;
    let mut ppm = format!("P6\n{} {}\n255\n", f.width, f.height).into_bytes();
    for row in f
        .frame_buffer()
        .chunks(denise::frame_builder::MAX_FRAME_WIDTH)
        .take(f.height as usize)
    {
        for pixel in &row[..f.width as usize] {
            ppm.extend_from_slice(&pixel.to_be_bytes()[1..]);
        }
    }
    std::fs::write(output.join("workbench.ppm"), ppm).unwrap();
    eprintln!("Workbench [overlay, bootblock, DOS, Workbench task]={stages:?}, budget={} CCK. Desktop appearance requires frame inspection.", m.cck);
    assert!(
        stages.iter().all(Option::is_some),
        "startup regression within the recorded budget"
    );
}
