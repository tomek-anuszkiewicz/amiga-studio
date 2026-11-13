use frame_builder::{FrameBuilder, OPAQUE_BLACK};

fn emit(frame: &mut FrameBuilder, line: u16, first: u16, end: u16) {
    for hpos in first..end {
        frame.step_cck(hpos, line);
        for half in 0..2 {
            let x = usize::from(hpos) * 2 + half;
            frame.write_pixel_pair(x, [sample(line, x * 2), sample(line, x * 2 + 1)]);
        }
    }
}

fn sample(line: u16, x: usize) -> u32 {
    0xff000000 | (u32::from(line) << 12) | x as u32
}

#[test]
fn visible_rows_keep_the_next_counter_prefix_without_storing_horizontal_blanking() {
    for line_ccks in [227, 228] {
        for interlaced in [false, true] {
            let mut frame = FrameBuilder::new();
            frame.begin_line(interlaced, true, false, false);
            emit(&mut frame, 100, 0, line_ccks);
            let row = usize::from(interlaced);
            frame.begin_line(interlaced, true, false, false);
            emit(&mut frame, 101, 0, 19);
            let width = usize::from(line_ccks) * 4 - 72;
            assert_eq!(frame.width as usize, width);
            for x in 0..width {
                let raster_x = x + 144;
                let expected = if raster_x < usize::from(line_ccks) * 4 {
                    sample(100, raster_x)
                } else {
                    sample(101, raster_x - usize::from(line_ccks) * 4)
                };
                assert_eq!(frame.get_pixel(x, row), expected, "CCKs {line_ccks}, x {x}");
                if !interlaced {
                    assert_eq!(frame.get_pixel(x, row + 1), expected);
                }
            }
        }
    }
}

#[test]
fn vertical_blanking_and_both_partial_edge_lines_are_excluded() {
    let mut frame = FrameBuilder::new();
    frame.begin_line(false, false, false, true);
    emit(&mut frame, 25, 0, 227);
    let before = frame.frame_buffer().to_vec();
    frame.begin_line(false, false, false, false);
    emit(&mut frame, 26, 0, 36);
    assert_eq!(
        frame.frame_buffer(),
        before,
        "top partial line and HBLANK must not write pixels"
    );
    emit(&mut frame, 26, 36, 227);
    frame.begin_line(false, false, false, false);
    emit(&mut frame, 27, 0, 227);
    frame.begin_line(false, false, true, true);
    let before = frame.frame_buffer().to_vec();
    emit(&mut frame, 0, 0, 18);
    assert_eq!(frame.frame_buffer(), before, "VBLANK must not write pixels");
    assert_eq!(frame.frame_number, 0);
    let mut restored = frame.clone();
    emit(&mut frame, 0, 18, 19);
    emit(&mut restored, 0, 18, 19);
    assert_eq!(restored, frame);
    assert_eq!(frame.frame_number, 1);
    assert_eq!(frame.height, 2, "only the complete scanline is retained");
    assert_eq!(frame.get_pixel(835, 0), sample(27, 71));
    assert_eq!(frame.get_pixel(0, 2), OPAQUE_BLACK);
    let before = frame.frame_buffer().to_vec();
    emit(&mut frame, 0, 19, 227);
    assert_eq!(frame.frame_buffer(), before);
}
