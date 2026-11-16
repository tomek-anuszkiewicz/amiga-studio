use frame_builder::{FrameBuilder, MAX_FRAME_HEIGHT, OPAQUE_BLACK};

#[test]
fn cck_samples_clip_at_complete_pairs_and_do_not_cross_storage_rows() {
    for interlaced in [false, true] {
        for standard in [config::VideoStandard::Pal, config::VideoStandard::Ntsc] {
            let mut frame = FrameBuilder::new();
            frame.set_video_standard(standard);
            frame.set_field_mode(interlaced, false);
            let width = frame.width as usize;
            frame.write_cck(width - 2, 0, [1, 2, 3, 4]);
            assert_eq!(frame.get_pixel(width - 2, 0), 1);
            assert_eq!(frame.get_pixel(width - 1, 0), 2);
            assert_eq!(frame.get_pixel(width, 0), OPAQUE_BLACK);
            assert_eq!(frame.get_pixel(0, frame.height as usize), OPAQUE_BLACK);
            assert_eq!(frame.get_pixel(0, 1), OPAQUE_BLACK);
            assert_eq!(
                frame.get_pixel(width - 2, 1),
                if interlaced { OPAQUE_BLACK } else { 1 }
            );
            let before = frame.clone();
            frame.write_cck(width - 1, 0, [5; 4]);
            frame.write_cck(width, 0, [6; 4]);
            frame.write_cck(0, MAX_FRAME_HEIGHT, [7; 4]);
            assert_eq!(frame, before);
            frame.write_cck(0, frame.height as usize - 1, [8, 9, 10, 11]);
            assert_eq!(frame.get_pixel(3, frame.height as usize - 1), 11);
        }
    }
}
