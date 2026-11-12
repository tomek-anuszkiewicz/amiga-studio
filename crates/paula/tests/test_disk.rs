use paula::Paula;

fn arm(length: u16, control: u16) -> Paula {
    let mut paula = Paula::new();
    assert!(paula.set_dmacon(0x8210));
    assert!(paula.set_adkcon(0x8000 | control));
    assert!(paula.set_dsklen(length));
    paula.step_cck();
    paula.step_cck();
    assert!(!paula.dma_active);
    assert!(paula.set_dsklen(length));
    paula.step_cck();
    paula
}

fn send_bits(paula: &mut Paula, value: u16, count: u8, clocks: usize) {
    for bit in (0..count).rev() {
        paula.pins.disk_read_high = value & (1 << bit) == 0;
        paula.step_cck();
        paula.pins.disk_read_high = true;
        for _ in 1..clocks {
            paula.step_cck();
        }
    }
}

#[test]
fn read_requests_use_the_last_available_disk_slots() {
    // Captured A500 DMAL frames use slot 11 for one word, 9/11 for two.
    for (count, expected) in [(1, 0x80), (2, 0xa0), (3, 0xa8)] {
        let mut paula = arm(0x8003, 0x0500);
        send_bits(&mut paula, 0x4489, 16, 7);
        for _ in 0..count {
            send_bits(&mut paula, 0xffff, 16, 7);
        }
        paula.strobe_sync(0x03c);
        assert_eq!(paula.sync.dmal_frame & 0xff, expected);
    }
}

#[test]
fn read_fifo_resynchronizes_and_counts_only_selected_dma_words() {
    for (control, clocks) in [(0x0500, 7), (0x0400, 14)] {
        let mut paula = arm(0x8003, control);
        send_bits(&mut paula, 0x1234, 16, clocks);
        assert_eq!(paula.disk.buffered_words(), 0);
        for word in [0x4489, 0x5a42, 0x4489, 0xa55a, 0xffff] {
            send_bits(&mut paula, word, 16, clocks);
        }
        assert_eq!(paula.disk.buffered_words(), 3);
        assert_eq!(paula.dsklen, 0x8003);
        assert_eq!(paula.interrupts.intreq & 0x1000, 0x1000);
        assert_eq!(paula.peek_dskbytr(), 0xc0ff);
        assert_eq!(paula.read_dskbytr(), 0xc0ff);
        assert_eq!(paula.peek_dskbytr(), 0x40ff);
        paula.strobe_sync(0x03c);
        assert_eq!(paula.sync.dmal_frame & 0xff, 0xa8);
        for (word, remaining) in [(0x5a42, 2), (0xa55a, 1), (0xffff, 0)] {
            assert_eq!(paula.take_disk_dma_word(), Some(word));
            assert_eq!(paula.dsklen & 0x3fff, remaining);
        }
        assert_eq!(paula.take_disk_dma_word(), None);
        assert!(!paula.dma_active);
        paula.step_cck();
        assert_eq!(paula.interrupts.intreq & 2, 2);
        assert_eq!(paula.peek_dskbytr() & 0x4000, 0);
    }
}

#[test]
fn byte_sync_skips_leading_zeroes_and_word_match_does_not_need_wordsync() {
    let mut paula = arm(0x8001, 0x0200);
    send_bits(&mut paula, 0, 5, 14);
    send_bits(&mut paula, 0xa5, 8, 14);
    send_bits(&mut paula, 0, 3, 14);
    send_bits(&mut paula, 0xd3, 8, 14);
    assert_eq!(paula.take_disk_dma_word(), Some(0xa5d3));
    paula.reset();
    assert!(paula.set_adkcon(0x8100));
    paula.step_cck();
    send_bits(&mut paula, 0x4489, 16, 7);
    assert_eq!(paula.peek_dskbytr() & 0x1000, 0x1000);
    assert_eq!(paula.interrupts.intreq & 0x1000, 0x1000);
    send_bits(&mut paula, 0, 1, 7);
    assert_eq!(paula.peek_dskbytr() & 0x1000, 0);
}

#[test]
fn wordsync_detects_a_second_match_between_software_word_boundaries() {
    let mut paula = arm(0x8001, 0x0500);
    // $448912345678 contains another $4489 ending seven bits after the first.
    // The following sixteen bits are $1A2B, not the software-framed $1234.
    for word in [0x4489, 0x1234, 0x5678] {
        send_bits(&mut paula, word, 16, 7);
    }
    assert_eq!(paula.take_disk_dma_word(), Some(0x1a2b));
}

#[test]
fn write_fifo_drains_msb_first_after_dma_count_reaches_zero() {
    for (control, clocks) in [(0x0100, 7), (0, 14)] {
        let mut paula = arm(0xc003, control);
        let words = [0xa55a, 0x8001, 0xffff];
        // All three queued register commits enter the FIFO on the next CCK.
        for word in words {
            assert!(paula.set_dskdat(word));
        }
        paula.step_cck();
        assert_eq!(paula.disk.buffered_words(), 3);
        assert_eq!(paula.dsklen & 0x3fff, 0);
        assert!(!paula.dma_active);
        assert_eq!(paula.interrupts.intreq & 2, 2);
        for word in words {
            for bit in (0..16).rev() {
                paula.step_cck();
                assert!(paula.pins.disk_write_enable);
                assert_eq!(!paula.pins.disk_write_high, word & (1 << bit) != 0);
                for _ in 1..clocks {
                    paula.step_cck();
                    assert!(paula.pins.disk_write_high, "one CCK per transition pulse");
                }
            }
        }
        paula.step_cck();
        assert!(!paula.pins.disk_write_enable);
        assert_eq!(paula.disk.buffered_words(), 0);
    }
}

#[test]
fn dma_disable_suppresses_requests_and_abort_cancels_buffered_output() {
    let mut paula = arm(0xc004, 0x0100);
    paula.strobe_sync(0x03c);
    assert_eq!(paula.sync.dmal_frame & 0xff, 0xfc);
    assert!(paula.set_dmacon(0x0010));
    paula.step_cck();
    paula.step_cck();
    paula.strobe_sync(0x03c);
    assert_eq!(paula.sync.dmal_frame & 0xff, 0);
    assert!(paula.set_dskdat(0xffff));
    paula.step_cck();
    assert_eq!(
        paula.disk.buffered_words(),
        1,
        "accepted word remains buffered"
    );
    assert!(paula.set_dsklen(0x4000));
    paula.step_cck();
    assert_eq!(paula.disk.buffered_words(), 0);
    assert!(!paula.pins.disk_write_enable);
    assert!(!paula.dma_armed);
    assert!(paula.set_dsklen(0x8000));
    paula.step_cck();
    assert!(paula.set_dsklen(0x8000));
    paula.step_cck();
    assert!(!paula.dma_active);
    assert_eq!(paula.interrupts.intreq & 2, 2);
}
