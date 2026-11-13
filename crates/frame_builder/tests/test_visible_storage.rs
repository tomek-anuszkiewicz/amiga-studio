use frame_builder::FrameBuilder;

#[test]
fn framebuffer_contains_only_the_complete_visible_rectangle() {
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
    assert_eq!(frame.frame_buffer().len(), 836 * 572);
}
