use agnus::{Agnus, AgnusModel};

#[test]
fn copper_fetch_waits_while_four_hires_planes_own_the_bus() {
    // HRM 6, Figure 6-12: four high-resolution planes take every fetch slot.
    // HRM 2: both Copper instruction words require their own DMA opportunity.
    for model in [AgnusModel::OcsPal8371, AgnusModel::OcsNtsc8370] {
        let mut agnus = Agnus::new(model);
        let mut ram = vec![0; 0x1000];
        ram[..4].copy_from_slice(&[0x01, 0x80, 0x0F, 0x00]);
        agnus.dma.write_dmacon(0x8380);
        agnus.dma.set_bplcon0(0xC000);
        agnus.dma.set_vstart(0);
        agnus.dma.set_vstop(200);
        agnus.copper.set_dma_enabled(true);
        agnus.copper.restart_list1();
        agnus.hpos = 0x37;
        agnus.vpos = 100;

        for _ in 0..8 {
            agnus.step_cck_ram(&mut ram);
            assert_eq!(agnus.copper.cop_pc, 0, "denied fetch advanced the PC");
            assert_eq!(agnus.pending_copper_write, None);
            agnus.poll_bpl_dma();
        }
    }
}
