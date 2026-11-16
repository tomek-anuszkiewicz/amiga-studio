use crate::receiver::Video;
use config::VideoStandard;
use denise::frame_builder::OPAQUE_BLACK;
const RED: u32 = 0xfff00000;
const GREEN: u32 = 0xff00f000;
const BLUE: u32 = 0xff0000f0;

#[test]
fn blank_lines_inside_the_rectangle_clear_old_pixels_in_the_current_field_only() {
    for interlaced in [false, true] {
        for odd in [false, true] {
            let mut frame = Video::new();
            frame.set_field_mode(interlaced, odd);
            frame.frame_buffer_mut().fill(0xffffffff);
            for (line, blank, color) in [(28, false, RED), (29, true, GREEN), (30, false, BLUE)] {
                frame.begin_line(interlaced, odd, false, blank);
                for hpos in 0..227 {
                    frame.step_cck(hpos, line);
                    for half in 0..2 {
                        frame.write_pair(half, [color; 2]);
                    }
                }
            }
            frame.begin_line(interlaced, interlaced && !odd, true, true);
            for hpos in 0..19 {
                frame.step_cck(hpos, 0);
            }
            let parity = usize::from(interlaced && odd);
            for x in 0..720 {
                assert_eq!(frame.get_pixel(x, 2 + parity), OPAQUE_BLACK);
                assert_eq!(
                    frame.get_pixel(x, parity),
                    if x < 712 { RED } else { OPAQUE_BLACK }
                );
                assert_eq!(
                    frame.get_pixel(x, 4 + parity),
                    if x < 712 { BLUE } else { OPAQUE_BLACK }
                );
                if interlaced {
                    for row in 0..3 {
                        assert_eq!(frame.get_pixel(x, row * 2 + 1 - parity), 0xffffffff);
                    }
                } else {
                    assert_eq!(frame.get_pixel(x, 3), OPAQUE_BLACK);
                }
            }
        }
    }
}

#[test]
fn fixed_rectangle_keeps_mixed_sample_markers_across_short_and_long_fields() {
    for (standard, base_lines, blank_lines, first_line, height) in [
        (VideoStandard::Pal, 312, 26, 28, 568),
        (VideoStandard::Ntsc, 262, 21, 21, 480),
    ] {
        for interlaced in [false, true] {
            let mut frame = Video::new();
            frame.set_video_standard(standard);
            for field in 0..2 {
                let odd = interlaced && field == 1;
                for line in 0..base_lines + u16::from(odd) {
                    frame.begin_line(interlaced, odd, field > 0 && line == 0, line < blank_lines);
                    let ccks = if standard == VideoStandard::Ntsc && line % 2 != 0 {
                        228
                    } else {
                        227
                    };
                    for hpos in 0..ccks {
                        frame.step_cck(hpos, line);
                        for half in 0..2 {
                            let x = hpos as usize * 4 + half * 2;
                            let color = if (200..208).contains(&x)
                                && (first_line + 10..first_line + 14).contains(&line)
                            {
                                BLUE
                            } else if field == 0 {
                                RED
                            } else {
                                GREEN
                            };
                            frame.write_pair(half, [color; 2]);
                        }
                    }
                }
            }
            frame.begin_line(interlaced, false, true, true);
            for hpos in 0..19 {
                frame.step_cck(hpos, 0);
            }
            assert_eq!(frame.frame_number, 2);
            assert_eq!([frame.width, frame.height], [720, height]);
            assert_eq!(frame.frame_buffer().len(), 720 * height as usize);
            for y in 0..height as usize {
                for x in 0..720 {
                    let missing_pal_prefix = standard == VideoStandard::Pal
                        && x >= 712
                        && (if interlaced { y == 566 } else { y >= 566 });
                    let expected = if missing_pal_prefix {
                        OPAQUE_BLACK
                    } else if (4..12).contains(&x) && (20..28).contains(&y) {
                        BLUE
                    } else if interlaced && y % 2 == 0 {
                        RED
                    } else {
                        GREEN
                    };
                    assert_eq!(
                        frame.get_pixel(x, y),
                        expected,
                        "{standard:?}, lace={interlaced}, {x},{y}"
                    );
                }
            }
        }
    }
}

#[test]
fn truncated_fields_clear_absent_rows_without_resizing_or_erasing_the_other_parity() {
    for interlaced in [false, true] {
        let mut frame = Video::new();
        let mut retained = [(0usize, OPAQUE_BLACK); 2];
        for (field, count) in [4, 4, 3, 2, 4, 4].into_iter().enumerate() {
            let odd = interlaced && field % 2 != 0;
            let color = 0xff000000 | (field as u32 + 1) * 0x10203;
            for line in 28..28 + count {
                frame.begin_line(interlaced, odd, false, false);
                for hpos in 0..227 {
                    frame.step_cck(hpos, line);
                    for half in 0..2 {
                        frame.write_pair(half, [color; 2]);
                    }
                }
            }
            frame.begin_line(interlaced, !odd && interlaced, true, true);
            for hpos in 0..19 {
                frame.step_cck(hpos, 0);
            }
            retained[usize::from(odd)] = (count as usize, color);
            assert_eq!([frame.width, frame.height], [720, 568]);
            for y in 0..568 {
                let (count, color) = retained[if interlaced { y % 2 } else { 0 }];
                for x in 0..720 {
                    let expected = if y / 2 < count && !(y / 2 + 1 == count && x >= 712) {
                        color
                    } else {
                        OPAQUE_BLACK
                    };
                    assert_eq!(
                        frame.get_pixel(x, y),
                        expected,
                        "lace={interlaced}, field={field}, {x},{y}"
                    );
                }
            }
        }
    }
}
