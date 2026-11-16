#![allow(clippy::unwrap_used)]

use config::fatal_emulation_error;
use std::cell::Cell;

#[test]
fn fatal_error_retains_the_diagnostic_and_never_reaches_later_work() {
    let later_work = Cell::new(false);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if !later_work.get() {
            fatal_emulation_error(format_args!(
                "Chip RAM invariant at CCK {}: rejected write at ${:06X}",
                42, 0x1234
            ));
        }
        later_work.set(true);
    }));
    let payload = result.unwrap_err();
    assert_eq!(
        payload.downcast_ref::<String>().unwrap(),
        "Chip RAM invariant at CCK 42: rejected write at $001234"
    );
    assert!(!later_work.get());
}
