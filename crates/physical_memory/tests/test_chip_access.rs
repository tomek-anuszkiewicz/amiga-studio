#![allow(clippy::unwrap_used)]

use config::chip_ram::ChipRamAccess;
use physical_memory::{A500Config, BusResult, PhysicalMemory, VideoStandard};

#[test]
fn every_second_read_or_write_fails_before_storage_changes() {
    for first_write in [false, true] {
        for second_write in [false, true] {
            let mut memory = PhysicalMemory::new();
            memory.set_cck(17);
            memory.write_chip_word_debug(0x20, 0x1122);
            memory.write_chip_word_debug(0x40, 0x3344);
            if first_write {
                memory.write_chip_word(0x20, 0x5566);
            } else {
                assert_eq!(memory.read_chip_word(0x20), 0x1122);
            }
            let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if second_write {
                    memory.write_chip_word(0x40, 0xffff);
                } else {
                    memory.read_chip_word(0x40);
                }
            }))
            .unwrap_err();
            let message = failure.downcast_ref::<String>().unwrap();
            assert!(
                message.contains("previous CCK=17, current CCK=17"),
                "{message}"
            );
            assert!(message.contains("Chip RAM access invariant"));
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
        memory.set_cck(1);
        if byte {
            assert_eq!(memory.write_byte(0x20, 0x55), BusResult::Ready(()));
        } else {
            assert_eq!(memory.write_word(0x20, 0x5566), BusResult::Ready(()));
        }
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            memory.read_chip_word(0x40);
        }))
        .is_err());
    }
    let mut memory =
        PhysicalMemory::from_config(A500Config::expanded_power_user(VideoStandard::Pal));
    memory.map_chip_ram_to_low_memory();
    memory.set_cck(1);
    memory.write_chip_word(0x20, 0x1234);
    assert_eq!(memory.write_word(0x200000, 0xabcd), BusResult::Ready(()));
    assert_eq!(memory.read_word(0x200000), BusResult::Ready(0xabcd));
    memory.chip_ram_blocked = true;
    assert_eq!(memory.read_word(0x20), BusResult::WaitState);
    memory.set_cck(2);
    memory.chip_ram_blocked = false;
    assert_eq!(memory.read_word(0x20), BusResult::Ready(0x1234));
}

#[test]
fn active_budget_survives_reentry_clone_restore_reset_and_counter_wrap() {
    let mut memory = PhysicalMemory::new();
    memory.set_cck(u64::MAX);
    memory.write_chip_word(0x20, 0xabcd);
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
        copy.set_cck(u64::MAX);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            copy.write_chip_word(0x40, 0x7777);
        }))
        .is_err());
        assert_eq!(copy.read_chip_word_debug(0x40), 0);
    }
    // A distinct healthy instance advances through wrapping to a real new CCK.
    let mut wrapped = memory.clone();
    wrapped.set_cck(0);
    assert_eq!(wrapped.read_chip_word(0x20), 0xabcd);
}

#[test]
fn first_access_at_zero_and_consecutive_or_idle_ccks_need_no_clearing() {
    let mut memory = PhysicalMemory::new();
    memory.write_chip_word(0x20, 0x1234);
    memory.set_cck(1);
    assert_eq!(memory.read_chip_word(0x20), 0x1234);
    memory.set_cck(100);
    memory.set_cck(100);
    memory.write_chip_word(0x20, 0x5678);
    let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        memory.write_chip_word(0x20, 0xffff);
    }));
    assert!(failure.is_err());
    assert_eq!(memory.read_chip_word_debug(0x20), 0x5678);
}

#[test]
fn clock_rebasing_preserves_occupied_and_idle_access_intervals() {
    for origin in [0, 1, 17, u64::MAX] {
        for occupied in [false, true] {
            let mut memory = PhysicalMemory::new();
            memory.set_cck(origin.wrapping_sub(u64::from(!occupied)));
            memory.write_chip_word(0x20, 0x1234);
            memory.set_cck(origin);
            memory.rebase_cck(0);
            let saved = serde_json::to_string(&memory).unwrap();
            let mut restored: PhysicalMemory = serde_json::from_str(&saved).unwrap();
            if occupied {
                assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    restored.write_chip_word(0x20, 0xffff);
                }))
                .is_err());
                assert_eq!(restored.read_chip_word_debug(0x20), 0x1234);
                // Use a healthy copy for continued execution after a failure.
                let mut next = memory.clone();
                next.set_cck(1);
                assert_eq!(next.read_chip_word(0x20), 0x1234);
            } else {
                assert_eq!(restored.read_chip_word(0x20), 0x1234);
            }
        }
    }
    let mut unused = PhysicalMemory::new();
    unused.rebase_cck(u64::MAX);
    unused.write_chip_word(0x20, 0x1234);
    assert_eq!(unused.read_chip_word_debug(0x20), 0x1234);
}
