#![allow(clippy::unwrap_used)]

use config::chip_ram::{ChipRamAccess, ChipRamOwner};
use physical_memory::{A500Config, BusResult, PhysicalMemory, VideoStandard};

#[test]
fn every_second_read_or_write_fails_before_storage_changes() {
    for first_write in [false, true] {
        for second_write in [false, true] {
            let mut memory = PhysicalMemory::new();
            memory.begin_cck(17, 0x1234);
            memory.write_chip_word_debug(0x20, 0x1122);
            memory.write_chip_word_debug(0x40, 0x3344);
            if first_write {
                memory.write_chip_word(0x20, 0x5566, ChipRamOwner::Blitter);
            } else {
                assert_eq!(memory.read_chip_word(0x20, ChipRamOwner::Copper), 0x1122);
            }
            let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if second_write {
                    memory.write_chip_word(0x40, 0xffff, ChipRamOwner::Disk);
                } else {
                    memory.read_chip_word(0x40, ChipRamOwner::Audio(3));
                }
            }))
            .unwrap_err();
            let message = failure.downcast_ref::<String>().unwrap();
            assert!(message.contains("CCK=17, CPU PC=$001234"), "{message}");
            assert!(message.contains("first=") && message.contains("rejected="));
            assert_eq!(
                memory.read_chip_word_debug(0x20),
                if first_write { 0x5566 } else { 0x1122 }
            );
            assert_eq!(memory.read_chip_word_debug(0x40), 0x3344);
        }
    }
}

#[test]
fn byte_word_and_cpu_dma_routes_share_exactly_one_budget() {
    for byte in [false, true] {
        let mut memory = PhysicalMemory::new();
        memory.map_chip_ram_to_low_memory();
        memory.begin_cck(1, 0);
        if byte {
            assert_eq!(memory.write_byte(0x20, 0x55), BusResult::Ready(()));
        } else {
            assert_eq!(memory.write_word(0x20, 0x5566), BusResult::Ready(()));
        }
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            memory.read_chip_word(0x40, ChipRamOwner::Bitplane(1));
        }))
        .is_err());
    }
    let mut memory =
        PhysicalMemory::from_config(A500Config::expanded_power_user(VideoStandard::Pal));
    memory.map_chip_ram_to_low_memory();
    memory.begin_cck(1, 0);
    memory.write_chip_word(0x20, 0x1234, ChipRamOwner::Sprite(2));
    assert_eq!(memory.write_word(0x200000, 0xabcd), BusResult::Ready(()));
    assert_eq!(memory.read_word(0x200000), BusResult::Ready(0xabcd));
    memory.chip_ram_blocked = true;
    assert_eq!(memory.read_word(0x20), BusResult::WaitState);
    memory.begin_cck(2, 0);
    memory.chip_ram_blocked = false;
    assert_eq!(memory.read_word(0x20), BusResult::Ready(0x1234));
}

#[test]
fn active_budget_survives_reentry_clone_restore_reset_and_counter_wrap() {
    let mut memory = PhysicalMemory::new();
    memory.begin_cck(u64::MAX, 0x1000);
    memory.write_chip_word(0x20, 0xabcd, ChipRamOwner::Disk);
    let json = serde_json::to_string(&memory).unwrap();
    for variant in 0..4 {
        let mut copy = if variant == 0 {
            memory.clone()
        } else {
            serde_json::from_str::<PhysicalMemory>(&json).unwrap()
        };
        if variant == 2 {
            copy.reset();
        }
        if variant == 3 {
            copy.reset_warm();
        }
        copy.begin_cck(u64::MAX, 0x1000);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            copy.write_chip_word(0x40, 0x7777, ChipRamOwner::Blitter);
        }))
        .is_err());
        assert_eq!(copy.read_chip_word_debug(0x40), 0);
    }
    // A distinct healthy instance advances through wrapping to a real new CCK.
    let mut wrapped = memory.clone();
    wrapped.begin_cck(0, 0);
    assert_eq!(wrapped.read_chip_word(0x20, ChipRamOwner::Copper), 0xabcd);
}
