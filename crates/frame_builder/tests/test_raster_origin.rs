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

#[test]
fn long_lines_keep_nonzero_rows_and_clear_short_line_tails() {
    for interlaced in [false, true] {
        let mut frame = FrameBuilder::new();
        frame.begin_line(interlaced, true, false, false);
        emit(&mut frame, 100, 0, 227);
        frame.begin_line(interlaced, true, false, false);
        emit(&mut frame, 101, 0, 228);
        frame.begin_line(interlaced, true, false, false);
        emit(&mut frame, 102, 0, 19);
        assert_eq!(frame.width, 840);
        let first_row = usize::from(interlaced);
        for x in 0..840 {
            let raster_x = x + 144;
            let expected = if raster_x < 912 {
                sample(101, raster_x)
            } else {
                sample(102, raster_x - 912)
            };
            assert_eq!(frame.get_pixel(x, first_row + 2), expected);
            if !interlaced {
                assert_eq!(frame.get_pixel(x, first_row + 3), expected);
            }
        }
        // The wider image preserves the earlier row and its unused right edge.
        assert_eq!(frame.get_pixel(0, first_row), sample(100, 144));
        assert_eq!(frame.get_pixel(835, first_row), sample(101, 71));
        for x in 836..840 {
            assert_eq!(frame.get_pixel(x, first_row), OPAQUE_BLACK);
        }

        let short_row = first_row + 4;
        frame.frame_buffer_mut()[short_row * 840..(short_row + 1) * 840].fill(0xffffffff);
        emit(&mut frame, 102, 19, 227);
        frame.begin_line(interlaced, true, false, false);
        emit(&mut frame, 103, 0, 19);
        assert_eq!(frame.get_pixel(835, short_row), sample(103, 71));
        for x in 836..840 {
            assert_eq!(frame.get_pixel(x, short_row), OPAQUE_BLACK);
            if !interlaced {
                assert_eq!(frame.get_pixel(x, short_row + 1), OPAQUE_BLACK);
            }
        }
    }
}

#[test]
fn early_strobes_and_out_of_range_coordinates_cannot_write_another_row() {
    let mut frame = FrameBuilder::new();
    frame.begin_line(false, false, false, false);
    emit(&mut frame, 100, 0, 20);
    frame.begin_line(false, false, false, false);
    frame.write_pixel_pair(0, [0xffffffff; 2]);
    assert!(frame
        .frame_buffer()
        .iter()
        .all(|pixel| *pixel == OPAQUE_BLACK));

    emit(&mut frame, 101, 0, 227);
    let before = frame.frame_buffer().to_vec();
    frame.write_pixel_pair(72 + 836 / 2, [0xffffffff; 2]);
    assert_eq!(frame.frame_buffer(), before);

    frame.begin_line(false, false, false, false);
    frame.step_cck(36, 386);
    let before = frame.frame_buffer().to_vec();
    frame.write_pixel_pair(72, [0xffffffff; 2]);
    assert_eq!(frame.frame_buffer(), before);
}

#[test]
fn reset_discards_mapped_rows_and_pending_prefix_output() {
    let mut frame = FrameBuilder::default();
    frame.begin_line(true, true, false, false);
    emit(&mut frame, 100, 0, 228);
    frame.begin_line(true, true, false, false);
    emit(&mut frame, 101, 0, 8);
    frame.reset();
    assert_eq!(frame, FrameBuilder::new());
    frame.write_pixel_pair(0, [0xffffffff; 2]);
    frame.write_pixel_pair(72, [0xffffffff; 2]);
    assert_eq!(frame, FrameBuilder::new());
}
