use frame_builder::FrameBuilder;

#[test]
fn framebuffer_keeps_visible_dimensions_with_a_fixed_row_stride() {
    let mut frame = FrameBuilder::new();
    for field in 0..2 {
        for line in 0..313 {
            frame.begin_line(false, false, field > 0 && line == 0, line < 26);
            for hpos in 0..227 {
                frame.step_cck(hpos, line);
            }
        }
    }
    frame.begin_line(false, false, true, true);
    frame.step_cck(18, 0);
    assert_eq!([frame.width, frame.height], [836, 572]);
    assert_eq!(frame.row_stride(), 840);
    assert_eq!(frame.frame_buffer().len(), frame.row_stride() * 572);
}
