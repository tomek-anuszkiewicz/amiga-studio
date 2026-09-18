use physical_memory::{AddressBus, BusAccessSize, BusResult};
use test_runner::{MemoryType, RecordedTransaction, TestMemoryBus, FLAT_TEST_RAM_SIZE};

#[test]
fn test_sparse_test_memory_bus_basics() {
    let mut bus = TestMemoryBus::new();
    assert_eq!(bus.unmapped_byte(), 0xFF);
    assert_eq!(bus.read_byte_debug(0x1000), 0xFF);
    assert_eq!(bus.read_word_debug(0x1000), 0xFFFF);

    // Reconfigure unmapped byte
    bus.set_unmapped_byte(0x00);
    assert_eq!(bus.unmapped_byte(), 0x00);
    assert_eq!(bus.read_byte_debug(0x1000), 0x00);
    assert_eq!(bus.read_word_debug(0x1000), 0x0000);

    // Load test RAM vectors
    bus.load_test_ram(&[[0x1000, 0x12], [0x1001, 0x34], [0x2000, 0xAB]]);
    assert_eq!(bus.read_byte_debug(0x1000), 0x12);
    assert_eq!(bus.read_byte_debug(0x1001), 0x34);
    assert_eq!(bus.read_word_debug(0x1000), 0x1234);
    assert_eq!(bus.read_byte_debug(0x2000), 0xAB);

    // Direct write debug
    bus.write_byte_debug(0x3000, 0x55);
    assert_eq!(bus.read_byte_debug(0x3000), 0x55);
    bus.write_word_debug(0x3002, 0xCAFE);
    assert_eq!(bus.read_word_debug(0x3002), 0xCAFE);

    // Invert memory
    bus.invert_test_memory();
    assert_eq!(bus.read_byte_debug(0x1000), !0x12);
    assert_eq!(bus.read_word_debug(0x3002), !0xCAFE);

    // Clear resets storage
    bus.clear();
    assert_eq!(bus.read_byte_debug(0x1000), 0xFF);
}

#[test]
fn test_flat_test_memory_bus_and_reset() {
    let mut bus = TestMemoryBus::new_flat();
    assert_eq!(bus.unmapped_byte(), 0x00);
    assert_eq!(FLAT_TEST_RAM_SIZE, 16 * 1024 * 1024);

    // Load test RAM into flat model
    bus.load_test_ram(&[[0x0000, 0x11], [0x0001, 0x22], [0x1000, 0x33]]);
    assert_eq!(bus.read_word_debug(0x0000), 0x1122);
    assert_eq!(bus.read_byte_debug(0x1000), 0x33);

    // Direct writes
    bus.write_byte_debug(0x2000, 0x77);
    assert_eq!(bus.read_byte_debug(0x2000), 0x77);
    bus.write_word_debug(0x2002, 0x8899);
    assert_eq!(bus.read_word_debug(0x2002), 0x8899);

    // O(K) reset clears modified addresses back to unmapped_byte
    bus.clear();
    assert_eq!(bus.read_byte_debug(0x0000), 0x00);
    assert_eq!(bus.read_word_debug(0x2002), 0x0000);
}

#[test]
fn test_bus_cycle_transactions_and_contention() {
    let mut bus = TestMemoryBus::new_flat();
    bus.enable_transaction_recording(true);
    assert!(bus.recorded_transactions().is_some());

    // Bus cycle reads and writes
    let r1 = bus.read_byte(0x1000);
    assert_eq!(r1, BusResult::Ready(0x00));

    let w1 = bus.write_word(0x2000, 0xBEEF);
    assert_eq!(w1, BusResult::Ready(()));
    assert_eq!(bus.read_word_debug(0x2000), 0xBEEF);

    let txs = bus.recorded_transactions().unwrap();
    assert_eq!(txs.len(), 2);
    assert_eq!(txs[0].addr, 0x1000);
    assert_eq!(txs[0].size, BusAccessSize::Byte);
    assert_eq!(txs[1].addr, 0x2000);
    assert_eq!(txs[1].size, BusAccessSize::Word);
    assert_eq!(txs[1].data, 0xBEEF);
    assert_eq!(
        txs[0],
        RecordedTransaction {
            is_read: true,
            addr: 0x1000,
            size: BusAccessSize::Byte,
            data: 0,
        }
    );

    // Contention lock on Chip RAM
    bus.set_address_type(0x1000, MemoryType::ChipRam);
    bus.set_address_type(0x200000, MemoryType::FastRam);
    assert!(bus.is_chip_ram_target(0x1000));
    assert!(!bus.is_chip_ram_target(0x200000));

    bus.chip_ram_blocked = true;
    assert!(bus.chip_ram_blocked);

    // Chip RAM target stalls with WaitState
    let wait_res = bus.read_word(0x1000);
    assert_eq!(wait_res, BusResult::WaitState);

    // Fast RAM target is unaffected by Chip RAM contention lock
    let fast_res = bus.read_word(0x200000);
    assert_eq!(fast_res, BusResult::Ready(0x0000));

    // Release lock
    bus.chip_ram_blocked = false;
    assert!(!bus.chip_ram_blocked);
    let ok_res = bus.read_word(0x1000);
    assert_eq!(ok_res, BusResult::Ready(0x0000));
}
