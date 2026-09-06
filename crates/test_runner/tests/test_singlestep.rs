use test_runner::runner::run_test_file;

#[test]
fn test_mame_nop() {
    let res = run_test_file("ref_src/SingleStepTests-m68000/v1/NOP.json", Some(100));
    assert!(res.is_ok(), "Failed to open NOP.json");
    let (passed, failed) = res.unwrap();
    assert!(passed > 0, "No tests passed");
    assert_eq!(failed, 0, "NOP tests failed");
}

#[test]
fn test_mame_add_b_sample() {
    let res = run_test_file("ref_src/SingleStepTests-m68000/v1/ADD.b.json", Some(50));
    assert!(res.is_ok(), "Failed to open ADD.b.json");
    let (passed, failed) = res.unwrap();
    assert!(passed > 0, "No tests passed");
    assert_eq!(failed, 0, "ADD.b tests failed");
}

#[test]
fn test_mame_sub_b_sample() {
    let res = run_test_file("ref_src/SingleStepTests-m68000/v1/SUB.b.json", Some(50));
    assert!(res.is_ok(), "Failed to open SUB.b.json");
    let (passed, failed) = res.unwrap();
    assert!(passed > 0, "No tests passed");
    assert_eq!(failed, 0, "SUB.b tests failed");
}
