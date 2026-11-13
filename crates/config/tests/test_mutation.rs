#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use config::{MutationMode, MutationQueue};

#[test]
fn full_queue_overwrite_reissues_after_retained_entries_without_losing_capacity() {
    let mut queue = MutationQueue::<3>::default();
    assert!(queue.stage(1, 10, 2, MutationMode::OverwritePending));
    assert!(queue.stage(2, 20, 1, MutationMode::Pipeline));
    assert!(queue.stage(3, 30, 2, MutationMode::Pipeline));
    assert!(queue.is_full());
    assert!(!queue.stage(1, 99, 0, MutationMode::OverwritePending));
    assert!(!queue.stage(4, 40, 1, MutationMode::OverwritePending));
    assert!(queue.stage(1, 11, 2, MutationMode::OverwritePending));
    assert!(queue.is_full());
    queue.tick();
    assert_eq!(queue.due(), [(2, 20)]);
    assert!(queue.stage(4, 40, 1, MutationMode::Pipeline));
    assert_eq!(
        queue.due(),
        [(2, 20)],
        "staging cannot change this clock's due set"
    );
    queue.tick();
    assert_eq!(queue.due(), [(3, 30), (1, 11), (4, 40)]);
    assert!(queue.is_empty());
    queue.tick();
    assert!(
        queue.due().is_empty(),
        "old scratch entries must not be replayed"
    );
}

#[test]
fn maximum_delay_and_reset_preserve_the_complete_nonzero_delay_domain() {
    let mut queue = MutationQueue::<2>::default();
    assert!(queue.stage(1, 10, u8::MAX, MutationMode::Pipeline));
    for remaining in (1..u8::MAX).rev() {
        queue.tick();
        assert!(queue.due().is_empty());
        assert_eq!(queue.pending()[0].remaining_cck, remaining);
    }
    queue.tick();
    assert_eq!(queue.due(), [(1, 10)]);
    assert!(queue.stage(2, 20, 1, MutationMode::Pipeline));
    queue.clear();
    assert!(queue.is_empty());
    assert!(queue.due().is_empty());
    queue.tick();
    assert!(queue.due().is_empty());
}

#[test]
fn test_stage_and_tick_pipeline_mode() {
    let mut buffer = MutationQueue::<8>::default();

    // Stage write 1 at cycle T with delay 2 (due at T+2)
    let staged1 = buffer.stage(0x180, 0x0F00, 2, MutationMode::Pipeline);
    assert!(staged1);

    // Tick 1 (cycle T advances to T+1)
    let mut committed = Vec::new();
    buffer.tick();
    committed.extend_from_slice(buffer.due());
    assert!(committed.is_empty());

    // Stage write 2 to same register at cycle T+1 with delay 2 (due at T+3)
    let staged2 = buffer.stage(0x180, 0x00F0, 2, MutationMode::Pipeline);
    assert!(staged2);

    // Tick 2 (cycle T+2) - write 1 should mature
    buffer.tick();
    committed.extend_from_slice(buffer.due());
    assert_eq!(committed, vec![(0x180, 0x0F00)]);

    // Tick 3 (cycle T+3) - write 2 should mature
    committed.clear();
    buffer.tick();
    committed.extend_from_slice(buffer.due());
    assert_eq!(committed, vec![(0x180, 0x00F0)]);

    // Tick 4 - buffer should now be completely empty
    committed.clear();
    buffer.tick();
    committed.extend_from_slice(buffer.due());
    assert!(committed.is_empty());
    assert!(buffer.is_empty());
}

#[test]
fn test_stage_and_tick_overwrite_pending_mode() {
    let mut buffer = MutationQueue::<8>::default();

    // Stage write 1 to DMACON with delay 2
    let staged1 = buffer.stage(0x096, 0x8200, 2, MutationMode::OverwritePending);
    assert!(staged1);

    // Cycle advances by 1 CCK
    let mut committed = Vec::new();
    buffer.tick();
    committed.extend_from_slice(buffer.due());
    assert!(committed.is_empty());

    // Before it matures, a new write to DMACON arrives with delay 2
    let staged2 = buffer.stage(0x096, 0x0200, 2, MutationMode::OverwritePending);
    assert!(staged2);

    // Should only occupy 1 slot in the buffer (pending write was overwritten and timer reset)
    let occupied = buffer.len();
    assert_eq!(occupied, 1);

    // Tick 1 after overwrite - should NOT mature yet (remaining was reset to 2, now decremented to 1)
    buffer.tick();
    committed.extend_from_slice(buffer.due());
    assert!(committed.is_empty());

    // Tick 2 after overwrite - now it matures with the new value
    buffer.tick();
    committed.extend_from_slice(buffer.due());
    assert_eq!(committed, vec![(0x096, 0x0200)]);
}

#[test]
fn test_zero_delay_and_full_queue_reject_without_commit() {
    let mut buffer = MutationQueue::<2>::default();

    // Delay zero is rejected; the utility never applies a write
    let staged_zero = buffer.stage(0x002, 0x1234, 0, MutationMode::Pipeline);
    assert!(!staged_zero);

    // Fill buffer
    assert!(buffer.stage(0x010, 1, 2, MutationMode::Pipeline));
    assert!(buffer.stage(0x020, 2, 2, MutationMode::Pipeline));

    // A full queue rejects a new write and retains every accepted entry
    let staged_overflow = buffer.stage(0x030, 3, 2, MutationMode::Pipeline);
    assert!(!staged_overflow);
}

#[test]
fn reused_slots_and_overwrites_preserve_issue_order() {
    let mut buffer = MutationQueue::<4>::default();
    assert!(buffer.stage(1, 10, 1, MutationMode::Pipeline));
    assert!(buffer.stage(2, 20, 2, MutationMode::Pipeline));
    let mut due = Vec::new();
    buffer.tick();
    due.extend_from_slice(buffer.due());
    assert_eq!(due, [(1, 10)]);
    assert!(buffer.stage(3, 30, 1, MutationMode::Pipeline));
    due.clear();
    buffer.tick();
    due.extend_from_slice(buffer.due());
    assert_eq!(due, [(2, 20), (3, 30)]);

    assert!(buffer.stage(4, 40, 2, MutationMode::OverwritePending));
    assert!(buffer.stage(5, 50, 1, MutationMode::Pipeline));
    assert!(buffer.stage(4, 41, 1, MutationMode::OverwritePending));
    due.clear();
    buffer.tick();
    due.extend_from_slice(buffer.due());
    assert_eq!(due, [(5, 50), (4, 41)]);
}
