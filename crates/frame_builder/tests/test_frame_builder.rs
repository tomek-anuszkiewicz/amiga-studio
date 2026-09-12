use frame_builder::{FrameBuilder, FRAME_BUFFER_PIXELS, MAX_FRAME_WIDTH};

#[test]
fn test_frame_builder_dimensions_and_pixels() {
    let mut fb = FrameBuilder::new();
    assert_eq!(fb.frame_buffer().len(), FRAME_BUFFER_PIXELS);
    assert!(!fb.frame_ready);

    fb.set_pixel(10, 20, 0xFFFF0000); // Red
    assert_eq!(fb.frame_buffer()[20 * MAX_FRAME_WIDTH + 10], 0xFFFF0000);

    fb.end_frame();
    assert!(fb.frame_ready);

    fb.begin_frame();
    assert!(!fb.frame_ready);
}
