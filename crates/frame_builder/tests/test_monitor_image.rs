use frame_builder::{FrameBuilder, OPAQUE_BLACK};

const RED: u32 = 0xfff00000;
const GREEN: u32 = 0xff00f000;
const BLUE: u32 = 0xff0000f0;

#[test]
fn monitor_image_keeps_the_border_and_square_pixels_without_retrace_or_partial_lines() {
    for (lines, blank_lines, ccks, width, height) in
        [(313, 26, 227, 836, 572), (263, 21, 228, 840, 482)]
    {
        for interlaced in [false, true] {
            let mut frame = FrameBuilder::new();
            for field in 0..2 {
                let odd = interlaced && field == 1;
                for line in 0..lines {
                    let blank = line < blank_lines;
                    frame.begin_line(interlaced, odd, field > 0 && line == 0, blank);
                    for hpos in 0..ccks {
                        frame.step_cck(hpos, line);
                        for half in 0..2 {
                            let x = hpos as usize * 4 + half * 2;
                            let color = if blank || (18..36).contains(&hpos) {
                                OPAQUE_BLACK
                            } else if (160..168).contains(&x)
                                && (blank_lines + 10..blank_lines + 14).contains(&line)
                            {
                                BLUE
                            } else if field == 0 {
                                RED
                            } else {
                                GREEN
                            };
                            frame.write_pixel_pair(x / 2, [color; 2]);
                        }
                    }
                }
            }
            frame.begin_line(interlaced, false, true, true);
            for hpos in 0..19 {
                frame.step_cck(hpos, 0);
                for half in 0..2 {
                    frame.write_pixel_pair(hpos as usize * 2 + half, [OPAQUE_BLACK; 2]);
                }
            }
            assert_eq!(frame.frame_number, 2);
            let before = frame.frame_buffer().to_vec();
            assert_eq!(
                [frame.width as usize, frame.height as usize],
                [width, height]
            );
            let pixels = frame.frame_buffer();
            assert_eq!(pixels.len(), width * height);
            for y in 0..height {
                for x in 0..width {
                    let expected = if (16..24).contains(&x) && (20..28).contains(&y) {
                        BLUE
                    } else if interlaced && y % 2 == 0 {
                        RED
                    } else {
                        GREEN
                    };
                    assert_eq!(pixels[y * width + x], expected, "pixel {x},{y}");
                }
            }
            assert_eq!(frame.frame_buffer(), before);
        }
    }
}

#[test]
fn short_interlaced_field_cannot_leave_a_partial_row_inside_the_opposite_long_field() {
    let mut frame = FrameBuilder::new();
    for (field, lines) in [(0, 312), (1, 313)] {
        let color = if field == 0 { RED } else { GREEN };
        for line in 0..lines {
            frame.begin_line(true, field == 1, field > 0 && line == 0, line < 26);
            for hpos in 0..227 {
                frame.step_cck(hpos, line);
                for half in 0..2 {
                    frame.write_pixel_pair(hpos as usize * 2 + half, [color; 2]);
                }
            }
        }
    }
    frame.begin_line(true, false, true, true);
    for hpos in 0..19 {
        frame.step_cck(hpos, 0);
    }
    assert_eq!([frame.width, frame.height], [836, 572]);
    for x in 0..836 {
        assert_eq!(frame.get_pixel(x, 568), RED);
        assert_eq!(frame.get_pixel(x, 569), GREEN);
        assert_eq!(frame.get_pixel(x, 570), OPAQUE_BLACK);
        assert_eq!(frame.get_pixel(x, 571), GREEN);
    }
}
