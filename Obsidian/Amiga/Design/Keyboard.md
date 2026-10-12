---
title: "Amiga 500 Keyboard"
aliases: ["Keyboard", "A500 Keyboard"]
tags: ["amiga", "design", "keyboard", "input", "cia"]
category: "Design"
subsystem: "cia"
status: "active"
created: 2026-09-02
updated: 2026-10-12
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

## Power LED and audio filter

The power LED and the switchable audio low-pass filter share the control signal from CIA-A port A bit 1 (`/LED`). When the power LED is bright, the filter is engaged; dimming the LED bypasses that filter. This is the power LED, separate from the keyboard's Caps Lock LED.

## Reference documentation and source

- [A500/A2000 Technical Reference Manual, Summary of Differences](../Reference/A500%20A2000%20Technical%20Reference%20Manual/01%20-%20Section%201%20Summary%20of%20Differences.md): distinguishes the A500 hard-wired reset and links the power LED to the audio filter.
- [Amiga Hardware Reference Manual, Chapter 8](../Reference/Hardware%20Reference%20Manual/08%20-%20Chapter%208%20-%20Interface%20Hardware.md): describes keyboard codes and the CIA serial interface.
- [Amiga Hardware Reference Manual, Appendix H](../Reference/Hardware%20Reference%20Manual/16%20-%20Appendix%20H%20-%20Keyboard.md): describes serial signaling and identifies the reset-warning protocol's model limitations.
- [`keyboard.rs`](../../../crates/keyboard/src/keyboard.rs): living keyboard-controller model.
