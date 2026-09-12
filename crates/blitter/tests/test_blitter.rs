use blitter::Blitter;

#[test]
fn test_blitter_reset_and_start() {
    let mut blit = Blitter::new();
    assert!(!blit.is_busy);

    blit.start_blit(0x0408); // 16 rows, 8 words
    assert!(blit.is_busy);
    assert_eq!(blit.bltsize, 0x0408);

    blit.reset();
    assert!(!blit.is_busy);
    assert_eq!(blit.bltsize, 0);
}
