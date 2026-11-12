use paula::interrupts::InterruptController;

#[test]
fn all_sources_and_simultaneous_priority_groups_match_hrm_figure_7_4() {
    let groups = [
        (0x0007u16, 1),
        (0x0008, 2),
        (0x0070, 3),
        (0x0780, 4),
        (0x1800, 5),
        (0x2000, 6),
    ];
    for (mask, level) in groups {
        for source in 0..14 {
            if mask & (1 << source) == 0 {
                continue;
            }
            let mut controller = InterruptController::new();
            controller.write_intena(0xffff);
            controller.request(1 << source);
            assert_eq!(controller.pending_level(), level);
            for (other_mask, other_level) in groups {
                controller.write_intreq(0x7fff);
                controller.request((1 << source) | other_mask);
                assert_eq!(controller.pending_level(), level.max(other_level));
                assert_eq!(controller.read_intreqr(), (1 << source) | other_mask);
            }
        }
    }
}

#[test]
fn enable_mask_and_acknowledgement_preserve_unselected_pending_sources() {
    let mut controller = InterruptController::new();
    controller.request(0x2001);
    controller.write_intena(0x8001);
    assert_eq!(controller.pending_level(), 0);
    controller.write_intena(0xc000);
    assert_eq!(controller.pending_level(), 1);
    controller.write_intena(0xa000);
    assert_eq!(controller.pending_level(), 6);
    controller.write_intreq(0x2000);
    assert_eq!(controller.pending_level(), 1);
    assert_eq!(controller.read_intreqr(), 1);
    controller.write_intena(0x4000);
    assert_eq!(controller.pending_level(), 0);
    assert_eq!(controller.read_intreqr(), 1);
    controller.write_intreq(0xc000);
    controller.write_intena(0xc000);
    assert_eq!(
        controller.pending_level(),
        1,
        "bit 14 is not another source"
    );
    controller.reset();
    assert_eq!(controller, InterruptController::new());
}
