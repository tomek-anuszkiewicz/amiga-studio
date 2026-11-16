use frame_builder::{FrameBuilder, MAX_FRAME_HEIGHT, MAX_FRAME_WIDTH, OPAQUE_BLACK};

#[test]
fn exchange_moves_the_allocation_and_preserves_progressive_partial_output() {
    let mut frame = FrameBuilder::new();
    frame.write_pixel_pair(0, 0, [0xff12_3456, 0xff65_4321]);
    frame.write_pixel_pair(2, 2, [0xffab_cdef, 0xfffe_dcba]);
    frame.finish_field(2);
    frame.frame_buffer_mut()[MAX_FRAME_WIDTH - 1] = 0xff11_2233;
    let original = frame.clone();
    let published_address = frame.frame_buffer().as_ptr();
    let mut published = FrameBuilder::new_buffer();
    let next_address = published.as_ptr();

    assert!(frame.exchange_buffer(&mut published));

    assert_eq!(published.as_ptr(), published_address);
    assert_eq!(frame.frame_buffer().as_ptr(), next_address);
    assert_eq!(frame, original);
    assert_eq!(published.len(), MAX_FRAME_WIDTH * MAX_FRAME_HEIGHT);
    assert_eq!(published[MAX_FRAME_WIDTH - 1], 0xff11_2233);

    frame.write_pixel_pair(0, 0, [0xff00_ff00, 0xff00_00ff]);
    assert_eq!(published[0], 0xff12_3456);
    assert_eq!(published[MAX_FRAME_WIDTH], 0xff12_3456);
    assert_eq!(frame.get_pixel(0, 0), 0xff00_ff00);
    assert_eq!(frame.get_pixel(0, 1), 0xff00_ff00);
    assert_eq!(frame.get_pixel(2, 2), 0xffab_cdef);
    assert_eq!(frame.get_pixel(2, 3), 0xffab_cdef);
}

#[test]
fn alternating_exchange_keeps_the_latest_opposite_interlaced_field() {
    let mut frame = FrameBuilder::new();
    let mut published = FrameBuilder::new_buffer();
    frame.set_field_mode(true, false);
    frame.write_pixel_pair(0, 0, [0xffff_0000; 2]);
    frame.write_pixel_pair(0, 2, [0xff88_0000; 2]);
    frame.finish_field(2);
    assert!(frame.exchange_buffer(&mut published));
    assert_eq!(published[0], 0xffff_0000);
    assert_eq!(published[MAX_FRAME_WIDTH], OPAQUE_BLACK);

    frame.set_field_mode(true, true);
    frame.write_pixel_pair(0, 1, [0xff00_ff00; 2]);
    frame.write_pixel_pair(0, 3, [0xff00_8800; 2]);
    frame.finish_field(2);
    assert!(frame.exchange_buffer(&mut published));
    assert_eq!(published[0], 0xffff_0000);
    assert_eq!(published[MAX_FRAME_WIDTH], 0xff00_ff00);
    assert_eq!(published[MAX_FRAME_WIDTH * 2], 0xff88_0000);
    assert_eq!(published[MAX_FRAME_WIDTH * 3], 0xff00_8800);

    frame.set_field_mode(true, false);
    frame.write_pixel_pair(0, 0, [0xff00_00ff; 2]);
    assert_eq!(frame.get_pixel(0, 0), 0xff00_00ff);
    assert_eq!(frame.get_pixel(0, 1), 0xff00_ff00);
    assert_eq!(frame.get_pixel(0, 2), 0xff88_0000);
    assert_eq!(frame.get_pixel(0, 3), 0xff00_8800);
    assert_eq!(published[0], 0xffff_0000);

    let mut restored = frame.clone();
    restored.write_pixel_pair(0, 2, [0xff00_0088; 2]);
    frame.write_pixel_pair(0, 2, [0xff00_0088; 2]);
    restored.finish_field(2);
    frame.finish_field(2);
    assert_eq!(frame, restored);
    assert_eq!(frame.frame_number, 3);
}

#[test]
fn exchange_rejects_wrong_storage_size_without_mutation() {
    let mut frame = FrameBuilder::new();
    frame.write_pixel_pair(0, 0, [0xff12_3456; 2]);
    let original = frame.clone();
    let original_address = frame.frame_buffer().as_ptr();

    for length in [
        0,
        MAX_FRAME_WIDTH * MAX_FRAME_HEIGHT - 1,
        MAX_FRAME_WIDTH * MAX_FRAME_HEIGHT + 1,
    ] {
        let mut replacement = vec![0xffab_cdef; length];
        let address = replacement.as_ptr();
        assert!(!frame.exchange_buffer(&mut replacement));
        assert_eq!(replacement.as_ptr(), address);
        assert_eq!(replacement, vec![0xffab_cdef; length]);
        assert_eq!(frame, original);
        assert_eq!(frame.frame_buffer().as_ptr(), original_address);
    }
}
