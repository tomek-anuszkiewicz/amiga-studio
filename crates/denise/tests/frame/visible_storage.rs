use crate::frame::Video;

#[test]
fn framebuffer_keeps_visible_dimensions_with_a_fixed_row_stride() {
    let mut frame = Video::new();
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
    assert_eq!([frame.width, frame.height], [720, 568]);
    assert_eq!(frame.row_stride(), 720);
    assert_eq!(frame.frame_buffer().len(), frame.row_stride() * 568);
}
