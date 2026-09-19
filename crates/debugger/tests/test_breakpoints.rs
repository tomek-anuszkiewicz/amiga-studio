use cpu::CpuState;
use debugger::{
    BreakpointCondition, BreakpointManager, ConditionOp, ConditionRegister, WatchAccess,
};

#[test]
fn test_condition_register_evaluation() {
    let mut state = CpuState::default();
    state.set_d_long(0, 0x12345678);
    state.set_d_long(7, 0x87654321);
    state.set_a_long(0, 0x00020000);
    state.set_a_long(7, 0x0007FFFE);
    state.instruction_pc = 0x001000;
    state.pc = 0x001004; // Hardware prefetch bus PC
    state.sr = 0x2715; // CCR is lower 5 bits (0x15)

    assert_eq!(ConditionRegister::D(0).get_value(&state), 0x12345678);
    assert_eq!(ConditionRegister::D(7).get_value(&state), 0x87654321);
    assert_eq!(ConditionRegister::A(0).get_value(&state), 0x00020000);
    assert_eq!(ConditionRegister::A(7).get_value(&state), 0x0007FFFE);
    assert_eq!(ConditionRegister::PC.get_value(&state), 0x001000);
    assert_eq!(ConditionRegister::SR.get_value(&state), 0x2715);
    assert_eq!(ConditionRegister::CCR.get_value(&state), 0x15);
}

#[test]
fn test_condition_operators() {
    let mut state = CpuState::default();
    state.set_d_long(0, 50);

    // Eq
    let cond_eq = BreakpointCondition::eq(ConditionRegister::D(0), 50);
    assert!(cond_eq.evaluate(&state));
    assert_eq!(cond_eq.format(), "D0 == $32");

    // Ne
    let cond_ne = BreakpointCondition {
        register: ConditionRegister::D(0),
        op: ConditionOp::Ne,
        value: 50,
        mask: None,
    };
    assert!(!cond_ne.evaluate(&state));

    // Lt & Gt
    let cond_lt = BreakpointCondition {
        register: ConditionRegister::D(0),
        op: ConditionOp::Lt,
        value: 100,
        mask: None,
    };
    assert!(cond_lt.evaluate(&state));

    let cond_gt = BreakpointCondition {
        register: ConditionRegister::D(0),
        op: ConditionOp::Gt,
        value: 100,
        mask: None,
    };
    assert!(!cond_gt.evaluate(&state));

    // Lte & Gte
    let cond_lte = BreakpointCondition {
        register: ConditionRegister::D(0),
        op: ConditionOp::Lte,
        value: 50,
        mask: None,
    };
    assert!(cond_lte.evaluate(&state));

    let cond_gte = BreakpointCondition {
        register: ConditionRegister::D(0),
        op: ConditionOp::Gte,
        value: 50,
        mask: None,
    };
    assert!(cond_gte.evaluate(&state));

    // MaskEq (e.g. check only lowest byte == 0x32)
    state.set_d_long(0, 0xFFFF0032);
    let cond_mask = BreakpointCondition {
        register: ConditionRegister::D(0),
        op: ConditionOp::MaskEq,
        value: 0x00000032,
        mask: Some(0x000000FF),
    };
    assert!(cond_mask.evaluate(&state));
    assert_eq!(cond_mask.format(), "(D0 & $FF) == $32");
}

#[test]
fn test_breakpoint_manager_pc_lifecycle() {
    let mut bpm = BreakpointManager::new();
    assert_eq!(bpm.total_count(), 0);

    // Add breakpoint at $1000
    bpm.add_pc_breakpoint(0x1000);
    assert!(bpm.has_pc_breakpoint(0x1000));
    assert!(!bpm.has_pc_breakpoint(0x1002));
    assert_eq!(bpm.total_count(), 1);

    // Toggle off
    bpm.toggle_pc_breakpoint(0x1000);
    assert!(!bpm.has_pc_breakpoint(0x1000));
    assert_eq!(bpm.total_count(), 0);

    // Toggle on again
    bpm.toggle_pc_breakpoint(0x1000);
    assert!(bpm.has_pc_breakpoint(0x1000));
    assert_eq!(bpm.total_count(), 1);

    // Remove explicitly
    bpm.remove_pc_breakpoint(0x1000);
    assert!(!bpm.has_pc_breakpoint(0x1000));
    assert_eq!(bpm.total_count(), 0);
}

#[test]
fn test_watchpoint_lifecycle_and_access_filtering() {
    let mut bpm = BreakpointManager::new();

    // 1. Write watchpoint at $2000..$2010
    bpm.add_watchpoint(0x2000, 0x2010, WatchAccess::Write);
    assert_eq!(bpm.total_count(), 1);

    assert!(bpm.check_watchpoint(0x2000, true)); // Start write -> trigger
    assert!(bpm.check_watchpoint(0x2008, true)); // Inside write -> trigger
    assert!(bpm.check_watchpoint(0x2010, true)); // End write -> trigger
    assert!(!bpm.check_watchpoint(0x2012, true)); // Outside -> no trigger
    assert!(!bpm.check_watchpoint(0x2008, false)); // Inside read -> no trigger (Write only)

    // 2. Read watchpoint at $3000..$3010
    bpm.add_watchpoint(0x3000, 0x3010, WatchAccess::Read);
    assert!(bpm.check_watchpoint(0x3005, false)); // Inside read -> trigger
    assert!(!bpm.check_watchpoint(0x3005, true)); // Inside write -> no trigger (Read only)

    // 3. Any access watchpoint at $4000..$4010
    bpm.add_watchpoint(0x4000, 0x4010, WatchAccess::Any);
    assert!(bpm.check_watchpoint(0x4005, false)); // Read -> trigger
    assert!(bpm.check_watchpoint(0x4005, true)); // Write -> trigger

    // Remove watchpoint by index
    bpm.remove_watchpoint(0);
    assert_eq!(bpm.total_count(), 2);
    assert!(!bpm.check_watchpoint(0x2008, true));

    // Clear all
    bpm.clear();
    assert_eq!(bpm.total_count(), 0);
}

#[test]
fn test_watchpoint_helpers() {
    let mut bpm = BreakpointManager::new();

    // Toggle on
    bpm.toggle_byte_watchpoint(0x002000, WatchAccess::Write);
    assert_eq!(bpm.total_count(), 1);
    assert!(bpm.find_watchpoint_at(0x002000).is_some());
    assert_eq!(
        bpm.find_watchpoint_at(0x002000).unwrap().access,
        WatchAccess::Write
    );
    assert!(bpm.find_watchpoint_at(0x002001).is_none());

    // Toggle off
    bpm.toggle_byte_watchpoint(0x002000, WatchAccess::Write);
    assert_eq!(bpm.total_count(), 0);
    assert!(bpm.find_watchpoint_at(0x002000).is_none());

    // Range watchpoint removal
    bpm.add_watchpoint(0x003000, 0x003010, WatchAccess::Read);
    assert!(bpm.find_watchpoint_at(0x003005).is_some());
    bpm.remove_watchpoints_at(0x003005);
    assert_eq!(bpm.total_count(), 0);
}
