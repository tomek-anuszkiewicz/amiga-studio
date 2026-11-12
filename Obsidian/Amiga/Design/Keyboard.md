---
title: "Amiga 500 Keyboard"
aliases: ["Keyboard", "A500 Keyboard"]
tags: ["amiga", "design", "keyboard", "input", "cia"]
category: "Design"
subsystem: "cia"
status: "active"
created: 2026-09-02
updated: 2026-11-12
related: ["[CIA.md](CIA.md)", "[Machine Loop.md](Machine%20Loop.md)", "[General Architecture.md](General%20Architecture.md)"]
tracked_paths:
  - "crates/keyboard"
---
# Amiga 500 Keyboard

The Amiga 500 keyboard is a separate device with its own controller. It scans the keys and sends key events to the computer; the CPU does not scan the key matrix.

## Connection and reading

The keyboard receives power through `+5V` and `GND`. Its `KCLK` clock line reaches the `CNT` input of [CIA-A](CIA.md), and its bidirectional `KDAT` data line reaches CIA-A's `SP` pin. The controller sends each key event as a serial byte. After eight bits, CIA-A makes the byte available in its serial data register (`SDR`) and can request a level 2 interrupt. Software reads `SDR` and acknowledges the byte by pulling `KDAT` low; the keyboard then sends the next byte. The byte identifies a physical key and whether it was pressed or released.

## Reset

The keyboard controller detects `Ctrl` + both `Amiga` keys independently of the running software. Its separate reset signal reaches the motherboard reset circuit, causing a hard-wired reset of the A500. Unlike the reset-warning protocol available on some A1000/A2000 keyboards, the A500 does not send a shutdown warning and wait for software to finish. The machine's subsequent warm-start behavior is described in [Machine Loop.md](Machine%20Loop.md).

## Transport contract and reconstruction boundary

The serializable `KeyboardPins` state in [pins.rs](../../../crates/keyboard/src/pins.rs)
is exposed as `keyboard.pins`. It groups KCLK/KDAT and reset line state
separately from the controller queue and transmission state.

The serial boundary is `pins.clock_high`/`pins.data_high` on the keyboard and delivered
SP/CNT input latches on [CIA-A](CIA.md#motherboard-pin-boundary). Released
open-drain lines are high. [Machine Loop](Machine%20Loop.md#captured-connector-transport-and-pending-bus-words)
resolves keyboard and enabled CIA drive levels, captures them after chip work
and delivers them before the next receiver step. The resolved return is also
retained as `pins.host_data_high` on the keyboard. That input, rather than a direct
read of CRA or an SDR assignment, is the controller's handshake input.
The keyboard retains the routed return in `pins.incoming_host_data_high` until
the next input phase; current `host_data_high` changes only when sampled.
Reset remains a separate line and does not wait for serial acknowledgment.
Its asserted level is captured with the other connector outputs in the machine's
single `keyboard_reset_asserted` boolean. On the next CCK the machine retains
that value locally before subsystem work captures the next level, then checks it
before stepping the CPU. A snapshot retains an assertion in flight even if the
source subsequently releases; warm reset clears the pending assertion. This
one-CCK connection uses no separate sampled field or delay queue.

HRM Appendix H specifies bit order 6-5-4-3-2-1-0-7 and active-low data. Ordinary
keys carry the release flag last; Caps Lock changes only on presses. The
receiver shifts eight SP levels on CNT rising edges, then exposes SDR and a
serial interrupt. The host acknowledges by pulling KDAT low and releasing it;
software must allow the documented 85-microsecond compatibility pulse, while
the keyboard must detect at least one microsecond. Approximate 20-microsecond
setup/low/recovery portions and the 143-millisecond timeout are device protocol
requirements, not the coordinator's one-CCK transport choice. Power-up requires
resynchronization, self-test and the `$FD/$FE` held-key stream. Timeout recovery
sends release-like resync bits and reports lost synchronization. These state
machines run in [`protocol.rs`](../../../crates/keyboard/src/protocol.rs).
Microsecond intervals are rounded up to whole CCKs for the selected PAL/NTSC
clock. The successful self-test path sends `$FD`, the keys held at initial
synchronization and `$FE`; no physical controller fault is simulated. A
recognized low holds transmission until the host releases KDAT. The receiver
waits two CCKs after releasing its own data driver before examining the return,
so its own previously captured low cannot acknowledge a byte.

The controller has a ten-keycode type-ahead ring. Full input retains queued
codes and reports `$FA` after draining them. Key state suppresses duplicate
physical transitions; software owns repeat. Caps Lock produces alternating
press/release codes on presses only. A500 reset directly asserts the separate
line and never queues the A1000/A2000 reset-warning protocol. The old whole-byte
step/acknowledgment shortcut is removed. Current state includes partial wire
phases, timeout/acknowledgment counters, queued and retransmitted codes, and
startup progress. Complete guest exchanges belong to 2.5; the frontend key
adapter remains 6.2 work.

## Power LED and audio filter

The power LED and the switchable audio low-pass filter share the control signal from CIA-A port A bit 1 (`/LED`). When the power LED is bright, the filter is engaged; dimming the LED bypasses that filter. This is the power LED, separate from the keyboard's Caps Lock LED.

The control level can be derived on demand from committed CIA-A PRA/DDRA.
The machine does not retain separate LED brightness or filter-enable fields.

## Reference documentation and source

- [A500/A2000 Technical Reference Manual, Summary of Differences](../Reference/A500%20A2000%20Technical%20Reference%20Manual/01%20-%20Section%201%20Summary%20of%20Differences.md): distinguishes the A500 hard-wired reset and links the power LED to the audio filter.
- [Amiga Hardware Reference Manual, Chapter 8](../Reference/Hardware%20Reference%20Manual/08%20-%20Chapter%208%20-%20Interface%20Hardware.md): describes keyboard codes and the CIA serial interface.
- [Amiga Hardware Reference Manual, Appendix H](../Reference/Hardware%20Reference%20Manual/16%20-%20Appendix%20H%20-%20Keyboard.md): describes serial signaling and identifies the reset-warning protocol's model limitations.
- [`keyboard.rs`](../../../crates/keyboard/src/keyboard.rs): living keyboard-controller model.
- [`keyboard/pins.rs`](../../../crates/keyboard/src/pins.rs): Serializable keyboard signal state.
