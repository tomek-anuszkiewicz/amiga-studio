use frame_builder::rgb444_to_argb32;

#[test]
fn rgb444_expands_to_full_range_argb_and_ignores_unused_bits() {
    for (rgb, expected) in [
        (0x0000, 0xff00_0000),
        (0x0fff, 0xffff_ffff),
        (0x0f00, 0xffff_0000),
        (0x00f0, 0xff00_ff00),
        (0x000f, 0xff00_00ff),
        (0x0888, 0xff88_8888),
        (0x0123, 0xff11_2233),
        (0x0002, 0xff00_0022),
        (0xf123, 0xff11_2233),
        (0xf000, 0xff00_0000),
    ] {
        assert_eq!(rgb444_to_argb32(rgb), expected, "RGB444 {rgb:#06x}");
    }
}
