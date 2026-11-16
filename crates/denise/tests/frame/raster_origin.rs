use crate::receiver::Video;
use config::VideoStandard;
use denise::frame_builder::OPAQUE_BLACK;

fn emit(frame: &mut Video, line: u16, first: u16, end: u16) {
    for hpos in first..end {
        frame.step_cck(hpos, line);
        for half in 0..2 {
            let x = usize::from(hpos) * 4 + half * 2;
            frame.write_pair(half, [sample(line, x), sample(line, x + 1)]);
        }
    }
}
fn sample(line: u16, x: usize) -> u32 {
    0xff000000 | (u32::from(line) << 12) | x as u32
}

#[test]
fn rectangle_corners_keep_the_next_counter_prefix_for_both_line_lengths() {
    for (standard, first_line, height) in [
        (VideoStandard::Pal, 28, 568),
        (VideoStandard::Ntsc, 21, 480),
    ] {
        for ccks in [227, 228] {
            for interlaced in [false, true] {
                let mut frame = Video::new();
                frame.set_video_standard(standard);
                for line in first_line..first_line + height / 2 {
                    frame.begin_line(interlaced, true, false, false);
                    emit(&mut frame, line, 0, ccks);
                }
                frame.begin_line(interlaced, true, false, false);
                emit(&mut frame, first_line + height / 2, 0, 19);
                assert_eq!([frame.width, frame.height], [720, u32::from(height)]);
                let parity = usize::from(interlaced);
                for line in 0..height / 2 {
                    let row = usize::from(line) * 2 + parity;
                    for x in 0..720 {
                        let raster_x = x + 196;
                        let (source_line, source_x) = if raster_x < usize::from(ccks) * 4 {
                            (first_line + line, raster_x)
                        } else {
                            (first_line + line + 1, raster_x - usize::from(ccks) * 4)
                        };
                        assert_eq!(
                            frame.get_pixel(x, row),
                            sample(source_line, source_x),
                            "CCKs {ccks}, x {x}, row {row}"
                        );
                        if !interlaced {
                            assert_eq!(frame.get_pixel(x, row + 1), sample(source_line, source_x));
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn blanking_and_samples_outside_the_rectangle_do_not_touch_storage() {
    let mut frame = Video::new();
    frame.begin_line(false, false, false, true);
    emit(&mut frame, 25, 0, 227);
    frame.begin_line(false, false, false, false);
    emit(&mut frame, 27, 0, 227);
    frame.begin_line(false, false, false, false);
    emit(&mut frame, 28, 0, 49);
    assert!(frame.frame_buffer().iter().all(|p| *p == OPAQUE_BLACK));
    emit(&mut frame, 28, 49, 227);
    frame.begin_line(false, false, false, false);
    emit(&mut frame, 29, 0, 19);
    assert_eq!(frame.get_pixel(0, 0), sample(28, 196));
    assert_eq!(frame.get_pixel(719, 1), sample(29, 7));
    let before = frame.frame_buffer().to_vec();
    frame.write_pixel_pair(720, 0, [0xffffffff; 2]);
    frame.write_pixel_pair(0, 568, [0xffffffff; 2]);
    assert_eq!(frame.frame_buffer(), before);
    frame.begin_line(false, false, false, false);
    emit(&mut frame, 312, 49, 227);
    assert_eq!(frame.frame_buffer(), before);
}

#[test]
fn a_blank_counter_prefix_clears_only_its_missing_tail_and_preserves_the_other_field() {
    for ccks in [227, 228] {
        let mut frame = Video::new();
        frame.begin_line(true, false, false, false);
        emit(&mut frame, 28, 0, ccks);
        frame.begin_line(true, false, false, false);
        emit(&mut frame, 29, 0, 19);
        frame.write_pixel_pair(718, 1, [0xff123456; 2]);
        frame.begin_line(true, false, true, true);
        let mut restored = frame.clone();
        for target in [&mut frame, &mut restored] {
            emit(target, 0, 0, 19);
        }
        assert_eq!(frame.denise, restored.denise);
        assert_eq!(frame.frame_number, 1);
        assert_eq!(
            frame.get_pixel(719, 0),
            sample(29, if ccks == 227 { 7 } else { 3 })
        );
        assert_eq!(frame.get_pixel(718, 1), 0xff123456);
        frame.begin_line(true, true, false, false);
        emit(&mut frame, 28, 0, ccks);
        frame.begin_line(true, false, true, true);
        emit(&mut frame, 0, 0, 19);
        for x in (usize::from(ccks) - 49) * 4..720 {
            assert_eq!(frame.get_pixel(x, 1), OPAQUE_BLACK);
        }
        assert_eq!(
            frame.get_pixel(719, 0),
            sample(29, if ccks == 227 { 7 } else { 3 })
        );
    }
}

#[test]
fn early_strobes_and_reset_cannot_write_another_row() {
    let mut frame = Video::new();
    frame.begin_line(false, false, false, false);
    emit(&mut frame, 28, 0, 20);
    frame.begin_line(false, false, false, false);
    frame.write_pair(0, [0xffffffff; 2]);
    assert!(frame.frame_buffer().iter().all(|p| *p == OPAQUE_BLACK));
    emit(&mut frame, 29, 0, 227);
    frame.begin_line(false, false, false, false);
    emit(&mut frame, 30, 0, 8);
    frame.reset();
    frame.write_pair(0, [0xffffffff; 2]);
    frame.write_pair(1, [0xffffffff; 2]);
    assert_eq!(frame.denise, Video::new().denise);
}
