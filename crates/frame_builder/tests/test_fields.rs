use frame_builder::FrameBuilder;

#[test]
fn both_standards_weave_alternate_fields_and_preserve_the_other_field() {
    for (lines, height) in [(313, 626), (263, 526)] {
        let mut frame = FrameBuilder::new();
        frame.begin_field(true, false, lines, false);
        frame.set_pixel(10, 100, 0xfff00000);
        assert_eq!(frame.get_pixel(10, 200), 0xfff00000);
        assert_eq!(frame.get_pixel(10, 201), 0xff000000);
        frame.begin_field(true, true, lines, true);
        frame.set_pixel(10, 100, 0xff00f000);
        assert_eq!(frame.get_pixel(10, 200), 0xfff00000);
        assert_eq!(frame.get_pixel(10, 201), 0xff00f000);
        assert_eq!(frame.height, height);
        assert_eq!(frame.frame_number, 1);
        assert!(frame.frame_ready);
        frame.begin_field(false, false, lines, true);
        frame.set_pixel(10, 100, 0xff0000f0);
        assert_eq!(frame.get_pixel(10, 100), 0xff0000f0);
        assert_eq!(frame.get_pixel(10, 200), 0xff000000);
        assert_eq!(frame.height, height / 2);
    }
}
