---
title: "Amiga 500 Keyboard"
aliases: ["Keyboard", "A500 Keyboard"]
tags: ["amiga", "design", "keyboard", "input", "cia"]
category: "Design"
subsystem: "cia"
status: "active"
created: 2026-09-02
updated: 2026-11-17
related: ["[CIA.md](CIA.md)", "[Machine Loop.md](Machine%20Loop.md)", "[General Architecture.md](General%20Architecture.md)"]
tracked_paths:
  - "crates/keyboard"
---
# Amiga 500 Keyboard

The Amiga 500 keyboard has its own controller. The controller scans the
keyboard matrix and sends key events to the computer, so the CPU receives
events without scanning the keys itself.

## Connection and reading

The keyboard receives power through `+5V` and `GND`. Its `KCLK` clock line
connects to the `CNT` input of [CIA-A](CIA.md), and its bidirectional `KDAT`
data line connects to CIA-A's `SP` pin. The controller sends each key event as
a serial byte. The byte identifies a physical key and whether it was pressed
or released. After receiving eight bits, CIA-A exposes the byte in its serial data
register (`SDR`) and can request a level 2 interrupt. Software reads
`SDR` and acknowledges the byte by pulling `KDAT` low. The keyboard then
sends the next byte.

## Reset

The keyboard controller detects `Ctrl` + both `Amiga` keys independently of
the running software. The controller signals the motherboard reset circuit on a separate line.
That signal triggers a hard-wired reset of the A500 without a warning or a
wait for software. Some A1000/A2000 keyboards support a reset warning and
wait for software to finish. [Machine Loop.md](Machine%20Loop.md)
describes the warm start that follows.

## Transport contract and reconstruction boundary

The serializable `KeyboardPins` state in [pins.rs](../../../crates/keyboard/src/pins.rs)
is exposed as `keyboard.pins`. It groups KCLK/KDAT and reset line state
separately from the controller queue and transmission state.

On the keyboard side, `pins.clock_high`/`pins.data_high` carry the serial
signals. On the [CIA-A](CIA.md#motherboard-pin-boundary) side, the delivered
signals are held in the SP/CNT input latches. Released open-drain lines are
high. The [Machine Loop](Machine%20Loop.md#captured-connector-transport-and-pending-bus-words)
resolves the keyboard and enabled CIA drive levels, captures the result
after chip work and delivers it before the next receiver step. The keyboard
keeps the routed return in `pins.incoming_host_data_high` until the next
input phase. Sampling then updates the current `host_data_high`, exposed as
`pins.host_data_high`. The controller uses the sampled pin for the handshake,
with no direct read of CRA or assignment to SDR.

Reset uses a separate line and proceeds independently of serial
acknowledgment. The machine captures the asserted level with the other
connector outputs in its single `keyboard_reset_asserted` boolean. At the next CCK, the machine saves that value locally so subsystem work
can capture the next level. Before stepping the CPU, the machine checks the
saved value. A snapshot
preserves an assertion in flight even if the source subsequently releases.
Warm reset clears the pending assertion. This one-CCK connection uses no
separate sampled field or delay queue.

HRM Appendix H specifies bit order 6-5-4-3-2-1-0-7 with active-low data.
Ordinary keys send the release flag last. Caps Lock changes only on presses.
The receiver shifts eight SP levels on CNT rising edges, then exposes SDR
and a serial interrupt. The host acknowledges by pulling KDAT low and then
releasing it. Software must allow the documented 85-microsecond compatibility
pulse, and the keyboard must detect a pulse of at least one microsecond. The
protocol requires approximate 20-microsecond setup, low and recovery
portions and a 143-millisecond timeout. The coordinator's one-CCK transport
is a separate model choice. At power-up the protocol requires
resynchronization, self-test and the `$FD/$FE` held-key stream. On timeout,
recovery sends release-like resync bits and reports lost synchronization.
[`protocol.rs`](../../../crates/keyboard/src/protocol.rs) runs these state
machines.

The model uses the selected PAL/NTSC clock to round each microsecond
interval up to a whole number of CCKs. After a successful self-test, it sends `$FD`, the keys held
at initial synchronization and `$FE`. The model simulates no physical
controller fault. Once the keyboard recognizes an acknowledgment low, it
holds transmission until the host releases KDAT. After releasing its own
data driver, the receiver waits two CCKs before checking the return, so its
previously captured low cannot acknowledge a byte.

The controller queues input in a ten-keycode type-ahead ring. If an attempt to
queue an ordinary keycode finds the ring already full, the controller retains
the queued codes and reports `$FA` after draining them. It uses key state to
suppress duplicate physical transitions. The frontend releases held keys when
it loses input capture or after a successful user load. The release preserves
any active or retried byte. Releases that do not fit the ring stay pending.
They follow existing ring data and any due overflow report, or take newly
available ring entries before fresh host transitions so a release precedes a
new press. Ordinary events retain the ten-code overflow behavior.

Pending releases are part of the serializable controller state. Caps Lock
keeps its toggle state and produces alternating press and release codes on
presses only. Software handles key repeat. A500 reset asserts the separate
line directly and never queues the A1000/A2000 reset-warning protocol. The
old whole-byte step and acknowledgment shortcut is removed. The current
state includes partial wire phases, timeout and acknowledgment counters,
queued and retransmitted codes, and startup progress.

If the idle guard expires with no code available, the controller enters an
explicit empty phase. This skips repeated attempts to select a code. Adding
a code to the queue resumes selection in the same CCK with the guard
already elapsed. Snapshots save this phase for continuation.
Guest-visible keyboard integration is scheduled with
[input-dependent native cases](../../../ROADMAP.md#native-disk-verification).
The [frontend](Application/GUI.md#input-and-output) now supplies physical
key events and owns their configurable mappings. Its basic adapter forwards
the physical transitions it receives, leaving keyboard-matrix blocking and
ghosting unmodeled. Original A500 combination fidelity and host rollover
limits remain open in [6.2](../../../ROADMAP.md#task-6-2), independently
of how the adapter delivers events.

## Power LED and audio filter

CIA-A port A bit 1 (`/LED`) supplies the same control signal to the power LED
and the switchable audio low-pass filter. The filter is engaged when the
power LED is bright. When the LED dims, audio bypasses the filter. The
keyboard's Caps Lock LED is separate.

The machine can derive the control level on demand from committed CIA-A
PRA/DDRA. It keeps no separate fields for LED brightness or filter enable.

## Reference documentation and source

- [A500/A2000 Technical Reference Manual, Summary of Differences](../Reference/A500%20A2000%20Technical%20Reference%20Manual/01%20-%20Section%201%20Summary%20of%20Differences.md): distinguishes the A500 hard-wired reset and links the power LED to the audio filter.
- [Amiga Hardware Reference Manual, Chapter 8](../Reference/Hardware%20Reference%20Manual/08%20-%20Chapter%208%20-%20Interface%20Hardware.md): describes keyboard codes and the CIA serial interface.
- [Amiga Hardware Reference Manual, Appendix H](../Reference/Hardware%20Reference%20Manual/16%20-%20Appendix%20H%20-%20Keyboard.md): describes serial signaling and identifies the reset-warning protocol's model limitations.
- [`keyboard.rs`](../../../crates/keyboard/src/keyboard.rs): living keyboard-controller model.
- [`keyboard/pins.rs`](../../../crates/keyboard/src/pins.rs): Serializable keyboard signal state.
