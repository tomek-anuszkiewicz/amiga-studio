#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use config::{stage_mutation, tick_mutations, DelayedMutation, MutationMode};

#[test]
fn test_stage_and_tick_pipeline_mode() {
    let mut buffer: [Option<DelayedMutation>; 8] = [None; 8];

    // Stage write 1 at cycle T with delay 2 (due at T+2)
    let staged1 = stage_mutation(&mut buffer, 0x180, 0x0F00, 2, MutationMode::Pipeline);
    assert!(staged1);

    // Tick 1 (cycle T advances to T+1)
    let mut committed = Vec::new();
    tick_mutations(&mut buffer, |reg, val| committed.push((reg, val)));
    assert!(committed.is_empty());

    // Stage write 2 to same register at cycle T+1 with delay 2 (due at T+3)
    let staged2 = stage_mutation(&mut buffer, 0x180, 0x00F0, 2, MutationMode::Pipeline);
    assert!(staged2);

    // Tick 2 (cycle T+2) - write 1 should mature
    tick_mutations(&mut buffer, |reg, val| committed.push((reg, val)));
    assert_eq!(committed, vec![(0x180, 0x0F00)]);

    // Tick 3 (cycle T+3) - write 2 should mature
    committed.clear();
    tick_mutations(&mut buffer, |reg, val| committed.push((reg, val)));
    assert_eq!(committed, vec![(0x180, 0x00F0)]);

    // Tick 4 - buffer should now be completely empty
    committed.clear();
    tick_mutations(&mut buffer, |reg, val| committed.push((reg, val)));
    assert!(committed.is_empty());
    assert!(buffer.iter().all(|s| s.is_none()));
}

#[test]
fn test_stage_and_tick_overwrite_pending_mode() {
    let mut buffer: [Option<DelayedMutation>; 8] = [None; 8];

    // Stage write 1 to DMACON with delay 2
    let staged1 = stage_mutation(
        &mut buffer,
        0x096,
        0x8200,
        2,
        MutationMode::OverwritePending,
    );
    assert!(staged1);

    // Cycle advances by 1 CCK
    let mut committed = Vec::new();
    tick_mutations(&mut buffer, |reg, val| committed.push((reg, val)));
    assert!(committed.is_empty());

    // Before it matures, a new write to DMACON arrives with delay 2
    let staged2 = stage_mutation(
        &mut buffer,
        0x096,
        0x0200,
        2,
        MutationMode::OverwritePending,
    );
    assert!(staged2);

    // Should only occupy 1 slot in the buffer (pending write was overwritten and timer reset)
    let occupied = buffer.iter().filter(|s| s.is_some()).count();
    assert_eq!(occupied, 1);

    // Tick 1 after overwrite - should NOT mature yet (remaining was reset to 2, now decremented to 1)
    tick_mutations(&mut buffer, |reg, val| committed.push((reg, val)));
    assert!(committed.is_empty());

    // Tick 2 after overwrite - now it matures with the new value
    tick_mutations(&mut buffer, |reg, val| committed.push((reg, val)));
    assert_eq!(committed, vec![(0x096, 0x0200)]);
}

#[test]
fn test_zero_delay_and_overflow_fallback() {
    let mut buffer: [Option<DelayedMutation>; 2] = [None; 2];

    // Delay 0 should not be staged (requires immediate commit)
    let staged_zero = stage_mutation(&mut buffer, 0x002, 0x1234, 0, MutationMode::Pipeline);
    assert!(!staged_zero);

    // Fill buffer
    assert!(stage_mutation(
        &mut buffer,
        0x010,
        1,
        2,
        MutationMode::Pipeline
    ));
    assert!(stage_mutation(
        &mut buffer,
        0x020,
        2,
        2,
        MutationMode::Pipeline
    ));

    // Third write should overflow and return false for immediate commit fallback
    let staged_overflow = stage_mutation(&mut buffer, 0x030, 3, 2, MutationMode::Pipeline);
    assert!(!staged_overflow);
}
