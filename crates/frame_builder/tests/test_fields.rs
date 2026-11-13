use frame_builder::FrameBuilder;

#[test]
fn alternate_fields_keep_the_other_parity_and_progressive_lines_are_doubled() {
    let mut frame = FrameBuilder::new();
    frame.begin_line(true, false, false, false);
    frame.step_cck(36, 100);
    frame.write_pixel_pair(77, [0xfff00000; 2]);
    assert_eq!(frame.get_pixel(10, 0), 0xfff00000);
    assert_eq!(frame.get_pixel(10, 1), 0xff000000);
    frame.step_cck(226, 100);
    frame.begin_line(true, false, false, false);
    frame.step_cck(36, 101);
    frame.begin_line(true, true, true, true);
    frame.step_cck(18, 0);
    frame.begin_line(true, true, false, false);
    frame.step_cck(36, 100);
    frame.write_pixel_pair(77, [0xff00f000; 2]);
    assert_eq!(frame.get_pixel(10, 0), 0xfff00000);
    frame.step_cck(226, 100);
    frame.begin_line(true, true, false, false);
    frame.step_cck(36, 101);
    frame.begin_line(true, false, true, true);
    frame.step_cck(18, 0);
    assert_eq!(frame.get_pixel(10, 1), 0xff00f000);
    frame.begin_line(false, false, false, false);
    frame.step_cck(36, 100);
    frame.write_pixel_pair(77, [0xff0000f0; 2]);
    assert_eq!(frame.get_pixel(10, 0), 0xff0000f0);
    assert_eq!(frame.get_pixel(10, 1), 0xff0000f0);
    assert_eq!(frame.get_pixel(10, 2), 0xff000000);
}
