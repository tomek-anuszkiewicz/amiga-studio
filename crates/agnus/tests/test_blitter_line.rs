use agnus::blitter::{Blitter, BlitterPhase};

fn line(octant: u16, single: bool, texture: u16) -> Blitter {
    let mut b = Blitter::new();
    b.bltcon0 = 0x0bca;
    b.bltcon1 = (octant << 2) | 1 | if single { 2 } else { 0 };
    b.bltafwm = 0xffff;
    b.bltalwm = 0xffff;
    b.bltadat = 0x8000;
    b.write_bltbdat(texture);
    b.bltapt = 0; // 4 * minor - 2 * major = 4 * 2 - 2 * 4.
    b.bltamod = -8;
    b.bltbmod = 8;
    b.bltcmod = 16;
    b.bltdmod = 16;
    b.bltcpt = 32 * 16 + 4;
    b.bltdpt = b.bltcpt;
    b.set_dma_enabled(true);
    b.trigger_blit((5 << 6) | 2);
    b
}

fn finish(b: &mut Blitter, ram: &mut [u8]) {
    for _ in 0..64 {
        b.step_cck_ram(ram, true);
        if !b.is_busy {
            return;
        }
    }
    assert!(!b.is_busy);
}

fn image(points: &[(i32, i32)]) -> [u8; 1024] {
    let mut ram = [0; 1024];
    for &(x, y) in points {
        ram[(y * 16 + x / 8) as usize] |= 0x80 >> (x % 8);
    }
    ram
}

#[test]
fn eight_documented_octants_follow_independent_pixel_coordinates() {
    for (octant, horizontal, dx, dy) in [
        (0, false, 1, 1),
        (1, false, 1, -1),
        (2, false, -1, 1),
        (3, false, -1, -1),
        (4, true, 1, 1),
        (5, true, -1, 1),
        (6, true, 1, -1),
        (7, true, -1, -1),
    ] {
        let mut b = line(octant, false, 0xffff);
        let mut ram = [0; 1024];
        finish(&mut b, &mut ram);
        let minor = [0, 1, 1, 2, 2];
        let points: Vec<_> = (0..5)
            .map(|i| {
                let (x, y) = if horizontal {
                    (i as i32, minor[i])
                } else {
                    (minor[i], i as i32)
                };
                (32 + dx * x, 32 + dy * y)
            })
            .collect();
        assert_eq!(ram, image(&points), "octant {octant}");
        assert!(b.poll_blit_irq());
        assert!(!b.poll_blit_irq());
    }
}

#[test]
fn single_dot_and_rotating_texture_select_pixels_without_stopping_coordinates() {
    for (single, texture, points) in [
        (true, 0xffff, vec![(32, 32), (33, 33), (35, 34)]),
        (false, 0x5001, vec![(32, 32), (34, 33), (36, 34)]),
    ] {
        let mut b = line(4, single, texture);
        let mut ram = [0; 1024];
        finish(&mut b, &mut ram);
        assert_eq!(ram, image(&points));
        assert_eq!(b.line_pixels_left, 0);
    }
}

#[test]
fn line_read_and_write_use_separate_grants() {
    let mut b = line(4, false, 0xffff);
    let mut ram = [0; 1024];
    b.step_cck_ram(&mut ram, false);
    assert_eq!(b.current_phase(), BlitterPhase::FetchC);
    let before = b.clone();
    b.step_cck_ram(&mut ram, false);
    assert_eq!(b, before);
    b.step_cck_ram(&mut ram, true);
    assert_eq!(ram, [0; 1024]);
    assert!(!b.wants_bus());
    b.step_cck_ram(&mut ram, false);
    assert_eq!(b.current_phase(), BlitterPhase::WriteD);
    b.step_cck_ram(&mut ram, false);
    assert_eq!(ram, [0; 1024]);
    b.step_cck_ram(&mut ram, true);
    assert_eq!(ram, image(&[(32, 32)]));
}
