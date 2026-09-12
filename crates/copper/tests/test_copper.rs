use copper::Copper;

#[test]
fn test_copper_reset_and_restart() {
    let mut cop = Copper::new();
    cop.cop1lc = 0x00040000;
    cop.cop2lc = 0x00050000;

    cop.restart_list1();
    assert_eq!(cop.cop_pc, 0x00040000);
    assert!(cop.is_running);

    cop.restart_list2();
    assert_eq!(cop.cop_pc, 0x00050000);

    cop.set_copcon(0x0002);
    assert!(cop.cdang);

    cop.reset();
    assert_eq!(cop.cop_pc, 0);
    assert!(!cop.is_running);
    assert!(!cop.cdang);
}
