//! Fatal internal invariant failures, handled only at the host owner boundary.

/// Immediately interrupts the failed operation. The host must never resume the
/// partially advanced machine. Call only inside an invariant failure branch so
/// diagnostic formatting and allocation stay outside successful execution.
#[cold]
#[inline(never)]
#[allow(
    clippy::panic,
    reason = "Fatal internal invariants terminate the machine owner at the host boundary"
)]
pub fn fatal_emulation_error(details: core::fmt::Arguments<'_>) -> ! {
    panic!("{details}")
}
