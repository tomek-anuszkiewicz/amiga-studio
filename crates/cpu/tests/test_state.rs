#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Comprehensive unit tests for CpuState encapsulation, stack pointer synchronization,
//! supervisor transitions, and Status Register invariants.

use cpu::{CpuState, SR_MASK, SR_RESET_DEFAULT, SR_T};

#[test]
fn test_stack_pointer_synchronization_in_supervisor_mode() {
    let mut state = CpuState::default();
    assert!(state.is_supervisor());
    assert_eq!(state.sr(), SR_RESET_DEFAULT);

    // Initial state: A7 is bound to SSP
    state.set_ssp(0x080000);
    assert_eq!(state.ssp(), 0x080000);
    assert_eq!(state.a_long(7), 0x080000);
    assert_eq!(state.a7(), 0x080000);
    assert_eq!(state.usp(), 0);

    // Modifying A7 via set_a_long(7, ...) must synchronize SSP
    state.set_a_long(7, 0x07FFFE);
    assert_eq!(state.a_long(7), 0x07FFFE);
    assert_eq!(state.a7(), 0x07FFFE);
    assert_eq!(state.ssp(), 0x07FFFE);
    assert_eq!(state.usp(), 0);

    // Modifying A7 via set_a7(...) must also synchronize SSP
    state.set_a7(0x07FFFC);
    assert_eq!(state.a_long(7), 0x07FFFC);
    assert_eq!(state.ssp(), 0x07FFFC);
    assert_eq!(state.usp(), 0);

    // Modifying USP while in Supervisor mode must not affect active A7
    state.set_usp(0x003000);
    assert_eq!(state.usp(), 0x003000);
    assert_eq!(state.ssp(), 0x07FFFC);
    assert_eq!(state.a_long(7), 0x07FFFC);
}

#[test]
fn test_stack_pointer_synchronization_in_user_mode() {
    let mut state = CpuState::default();
    state.set_ssp(0x080000);
    state.set_usp(0x003000);

    // Switch to User mode (S=0)
    state.set_supervisor(false);
    assert!(!state.is_supervisor());
    assert_eq!(state.a_long(7), 0x003000);
    assert_eq!(state.a7(), 0x003000);
    assert_eq!(state.usp(), 0x003000);
    assert_eq!(state.ssp(), 0x080000);

    // Modifying A7 via set_a_long(7, ...) must synchronize USP
    state.set_a_long(7, 0x002FFE);
    assert_eq!(state.a_long(7), 0x002FFE);
    assert_eq!(state.a7(), 0x002FFE);
    assert_eq!(state.usp(), 0x002FFE);
    assert_eq!(state.ssp(), 0x080000); // SSP untouched

    // Modifying A7 via set_a7(...) must synchronize USP
    state.set_a7(0x002FFC);
    assert_eq!(state.a_long(7), 0x002FFC);
    assert_eq!(state.usp(), 0x002FFC);
    assert_eq!(state.ssp(), 0x080000);

    // Modifying SSP while in User mode must not affect active A7
    state.set_ssp(0x090000);
    assert_eq!(state.ssp(), 0x090000);
    assert_eq!(state.usp(), 0x002FFC);
    assert_eq!(state.a_long(7), 0x002FFC);

    // Modifying USP via set_usp must update active A7 in User mode
    state.set_usp(0x002FF0);
    assert_eq!(state.usp(), 0x002FF0);
    assert_eq!(state.a_long(7), 0x002FF0);
    assert_eq!(state.ssp(), 0x090000);
}

#[test]
fn test_supervisor_transition_atomically_swaps_a7() {
    let mut state = CpuState::default();
    state.set_ssp(0x080000);
    state.set_usp(0x003000);

    // Transition from Supervisor to User
    state.set_supervisor(false);
    assert_eq!(state.a_long(7), 0x003000);
    assert_eq!(state.usp(), 0x003000);
    assert_eq!(state.ssp(), 0x080000);

    // Push value onto user stack
    state.set_a_long(7, 0x002FFC);
    assert_eq!(state.a_long(7), 0x002FFC);
    assert_eq!(state.usp(), 0x002FFC);

    // Transition from User to Supervisor
    state.set_supervisor(true);
    assert_eq!(state.a_long(7), 0x080000);
    assert_eq!(state.ssp(), 0x080000);
    assert_eq!(state.usp(), 0x002FFC);

    // Push value onto supervisor stack
    state.set_a_long(7, 0x07FFF8);
    assert_eq!(state.a_long(7), 0x07FFF8);
    assert_eq!(state.ssp(), 0x07FFF8);
    assert_eq!(state.usp(), 0x002FFC);

    // Transition back to User
    state.set_supervisor(false);
    assert_eq!(state.a_long(7), 0x002FFC);
    assert_eq!(state.usp(), 0x002FFC);
    assert_eq!(state.ssp(), 0x07FFF8);
}

#[test]
fn test_set_sr_masks_reserved_bits_and_swaps_stack_pointers() {
    let mut state = CpuState::default();
    state.set_ssp(0x080000);
    state.set_usp(0x003000);

    // Setting SR with reserved bits (0xFFFF) must mask them with SR_MASK (0xA71F)
    state.set_sr(0xFFFF);
    assert_eq!(state.sr(), SR_MASK);
    assert_eq!(state.sr() & !SR_MASK, 0);
    assert!(state.is_supervisor());
    assert!(state.is_trace());
    assert_eq!(state.a_long(7), 0x080000);

    // Setting SR with S=0 (User mode, e.g. 0x0000)
    state.set_sr(0x0000);
    assert_eq!(state.sr(), 0x0000);
    assert!(!state.is_supervisor());
    assert_eq!(state.a_long(7), 0x003000);

    // Setting SR with S=1 (Supervisor mode, e.g. 0x2700)
    state.set_sr(0x2700);
    assert_eq!(state.sr(), 0x2700);
    assert!(state.is_supervisor());
    assert_eq!(state.a_long(7), 0x080000);

    // Setting SR without changing S bit must preserve A7
    state.set_a_long(7, 0x07FFFC);
    state.set_sr(0x2000); // S still 1, but interrupt mask changed to 0
    assert_eq!(state.a_long(7), 0x07FFFC);
    assert_eq!(state.ssp(), 0x07FFFC);
    assert_eq!(state.usp(), 0x003000);
}

#[test]
fn test_trace_bit_helpers() {
    let mut state = CpuState::default();
    assert!(!state.is_trace());

    state.set_trace(true);
    assert!(state.is_trace());
    assert_eq!(state.sr() & SR_T, SR_T);

    state.clear_trace();
    assert!(!state.is_trace());
    assert_eq!(state.sr() & SR_T, 0);
}

#[test]
fn test_clear_registers_clears_all_including_ssp_and_usp() {
    let mut state = CpuState::default();
    for i in 0..8 {
        state.set_d_long(i, 0x1000_0000 + (i as u32));
        state.set_a_long(i, 0x2000_0000 + (i as u32));
    }
    state.set_usp(0x003000);
    state.set_ssp(0x080000);

    state.clear_registers();

    for i in 0..8 {
        assert_eq!(state.d_long(i), 0);
        assert_eq!(state.a_long(i), 0);
    }
    assert_eq!(state.usp(), 0);
    assert_eq!(state.ssp(), 0);
}

#[test]
fn test_serde_state_roundtrip_preserves_invariants() {
    let mut state = CpuState::default();
    state.set_ssp(0x080000);
    state.set_usp(0x003000);
    state.set_sr(0x2715);
    state.set_d_long(0, 0x12345678);
    state.set_a_long(0, 0x9ABCDEF0);

    let serialized = serde_json::to_string(&state).unwrap();
    let deserialized: CpuState = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized.sr(), 0x2715);
    assert_eq!(deserialized.ssp(), 0x080000);
    assert_eq!(deserialized.usp(), 0x003000);
    assert_eq!(deserialized.a_long(7), 0x080000);
    assert_eq!(deserialized.d_long(0), 0x12345678);
    assert_eq!(deserialized.a_long(0), 0x9ABCDEF0);
}
