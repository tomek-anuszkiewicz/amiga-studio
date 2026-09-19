#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use cpu::CpuState;
use debugger::temporal::TemporalHistory;
use debugger::trace::{TraceRingBuffer, TRACE_BUFFER_SIZE};

#[test]
fn test_trace_ring_buffer_wrapping_and_indexing() {
    let mut trace = TraceRingBuffer::new();
    assert!(trace.is_empty());
    assert_eq!(trace.len(), 0);

    let state = CpuState::default();

    // 1. Fill exactly TRACE_BUFFER_SIZE entries (1024)
    for i in 0..TRACE_BUFFER_SIZE {
        trace.record(
            i as u64,
            0x1000 + (i as u32) * 2,
            0x4E71,
            format!("NOP #{}", i),
            state.clone(),
        );
    }
    assert_eq!(trace.len(), TRACE_BUFFER_SIZE);
    assert!(!trace.is_empty());
    assert_eq!(trace.get(0).unwrap().cck, 0);
    assert_eq!(trace.get(TRACE_BUFFER_SIZE - 1).unwrap().cck, 1023);

    // 2. Overwrite 100 entries (total 1124 recorded)
    for i in 0..100 {
        trace.record(
            (1024 + i) as u64,
            0x2000 + (i as u32) * 2,
            0x4E75,
            format!("RTS #{}", i),
            state.clone(),
        );
    }
    assert_eq!(trace.len(), TRACE_BUFFER_SIZE);

    // Chronological index 0 should now be entry 100 (cck = 100)
    assert_eq!(trace.get(0).unwrap().cck, 100);
    // Chronological index 1023 should be entry 1123 (cck = 1123)
    assert_eq!(trace.get(TRACE_BUFFER_SIZE - 1).unwrap().cck, 1123);

    // Out of bounds
    assert!(trace.get(TRACE_BUFFER_SIZE).is_none());

    // 3. Clear
    trace.clear();
    assert_eq!(trace.len(), 0);
    assert!(trace.is_empty());
}

#[test]
fn test_temporal_history_wrapping_and_resizing() {
    let mut history = TemporalHistory::new(100);
    assert_eq!(history.capacity(), 100);

    let mut state = CpuState::default();

    // Record 150 items into capacity of 100
    for i in 0..150 {
        state.set_d_long(0, i as u32);
        history.record(
            i as u64 * 10,
            0x1000 + (i as u32) * 2,
            0x4E71,
            state.clone(),
        );
    }

    assert_eq!(history.len(), 100);
    assert_eq!(history.total_recorded(), 150);

    // Oldest entry should have D0 = 50, CCK = 500
    let oldest = history.get_chronological(0).unwrap();
    assert_eq!(oldest.cck, 500);
    assert_eq!(oldest.state.d_long(0), 50);

    // Newest entry should have D0 = 149, CCK = 1490
    let newest = history.get_chronological(99).unwrap();
    assert_eq!(newest.cck, 1490);
    assert_eq!(newest.state.d_long(0), 149);

    // Resize up to 200: all 100 items should be retained
    history.set_capacity(200);
    assert_eq!(history.capacity(), 200);
    assert_eq!(history.len(), 100);
    assert_eq!(history.get_chronological(0).unwrap().cck, 500);

    // Resize down to 50: newest 50 should be retained
    history.set_capacity(50);
    assert_eq!(history.capacity(), 50);
    assert_eq!(history.len(), 50);
    assert_eq!(history.get_chronological(0).unwrap().cck, 1000); // 150 - 50 = 100 -> cck 1000
    assert_eq!(history.get_chronological(49).unwrap().cck, 1490);
}

#[test]
fn test_temporal_history_scrub_boundary_clamping() {
    let mut history = TemporalHistory::new(50);
    let state = CpuState::default();

    for i in 0..20 {
        history.record(i as u64 * 1000, 0x1000, 0x4E71, state.clone());
    }

    // Step back beyond start -> clamps to 0
    let idx = history.step_back_n(100).unwrap();
    assert_eq!(idx, 0);
    assert_eq!(history.scrub_cursor, Some(0));

    // Step back again at 0 -> stays at 0
    let idx2 = history.step_back_n(5).unwrap();
    assert_eq!(idx2, 0);

    // Step forward past head -> returns None (resets to live head)
    let fwd = history.step_forward_n(100);
    assert_eq!(fwd, None);
    assert_eq!(history.scrub_cursor, None);
}

#[test]
fn test_temporal_history_seek_cck_edge_cases() {
    let mut history = TemporalHistory::new(100);

    // Empty history seek
    assert_eq!(history.find_closest_cck(1000), None);

    let state = CpuState::default();
    history.record(100, 0x1000, 0x4E71, state.clone());

    // Single item seek
    assert_eq!(history.find_closest_cck(50), Some(0));
    assert_eq!(history.find_closest_cck(100), Some(0));
    assert_eq!(history.find_closest_cck(200), Some(0));

    // Multiple items
    history.record(200, 0x1002, 0x4E71, state.clone());
    history.record(300, 0x1004, 0x4E71, state);

    assert_eq!(history.find_closest_cck(90), Some(0));
    assert_eq!(history.find_closest_cck(140), Some(0));
    assert_eq!(history.find_closest_cck(160), Some(1));
    assert_eq!(history.find_closest_cck(240), Some(1));
    assert_eq!(history.find_closest_cck(260), Some(2));
    assert_eq!(history.find_closest_cck(290), Some(2));
    assert_eq!(history.find_closest_cck(500), Some(2));
}
