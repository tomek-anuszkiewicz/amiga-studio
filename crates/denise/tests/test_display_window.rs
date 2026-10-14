use config::{BeamPosition, DeniseModel};
use denise::{frame_builder, Denise};

#[test]
fn test_denise_display_window_uses_horizontal_bits_only() {
    let mut inside_vertical_window = Denise::new(DeniseModel::Ocs8362);
    let mut outside_vertical_window = Denise::new(DeniseModel::Ocs8362);

    for (denise, diwstrt) in [
        (&mut inside_vertical_window, 0x6481),
        (&mut outside_vertical_window, 0xAA81),
    ] {
        denise.diwstrt = diwstrt;
        denise.diwstop = 0xC8C1;
        denise.bplcon0 = 0x1000;
        denise.color[1] = 0x00F0;

        for hpos in 0..80 {
            if hpos == 64 {
                denise.write_bpldat(0, 0xFFFF);
            }
            denise.step_cck(BeamPosition::new(hpos, 150, false));
        }
    }

    let color = frame_builder::rgb444_to_argb32(0x00F0);
    let green_pixels = (256..320)
        .filter(|&x| inside_vertical_window.frame_builder.get_pixel(x, 150) == color)
        .count();
    assert!(green_pixels > 0);
    for x in 256..320 {
        assert_eq!(
            outside_vertical_window.frame_builder.get_pixel(x, 150),
            inside_vertical_window.frame_builder.get_pixel(x, 150),
            "Denise must ignore DIW vertical bits at pixel {x}"
        );
    }
}
