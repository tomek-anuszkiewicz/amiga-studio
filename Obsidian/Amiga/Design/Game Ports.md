---
title: "Game Ports"
aliases: ["Gameports", "Controller Ports", "Port 1", "Port 2"]
tags: ["amiga", "design", "game_ports", "input", "denise", "paula", "cia"]
category: "Design"
subsystem: "game_ports"
status: "active"
created: 2026-09-13
updated: 2026-11-11
related: ["[Mouse.md](Mouse.md)", "[Joystick.md](Joystick.md)", "[Denise.md](Denise.md)", "[Paula.md](Paula.md)", "[CIA.md](CIA.md)", "[Machine Loop.md](Machine%20Loop.md)", "[General Architecture.md](General%20Architecture.md)"]
tracked_paths:
  - "crates/game_ports"
---

# Game Ports

The two game ports represent the Amiga's controller sockets. Each port can host an emulated [mouse](Mouse.md), [joystick](Joystick.md), or no device. The frontend translates input from host hardware into the state of those emulated devices. A host mouse can supply mouse movement and buttons; host keyboard controls can be mapped to joystick directions and buttons. The Amiga keyboard itself is a separate peripheral.

Game port signals do not belong to a single chip. The corresponding registers and input lines are distributed across [Denise](Denise.md), [Paula](Paula.md), and [CIA-A](CIA.md). The game ports model the connected devices and their signals; those chips retain their own register state and behavior.

[MachineLoop](Machine%20Loop.md) owns the game ports alongside the chips. It samples the port signals and passes them to the appropriate chip, so the devices and chips communicate through the machine rather than through direct references to one another.

## Connector delivery

The serializable `GamePortsPins` state in [pins.rs](../../../crates/game_ports/src/pins.rs)
is exposed as `game_ports.pins`. It groups direction outputs and delivered
fire/POT drive levels separately from plugged device state.

Each port's low nibble in `pins.direction_pins` carries connector pins 1-4, high
when released. The [machine](Machine%20Loop.md#captured-connector-transport-and-pending-bus-words)
samples these external levels into Denise's `game_port_inputs` before engine
work in the current CCK, without retaining another pending copy. The input time
already denotes arrival at the receiver pin. It does not overwrite JOY registers
with device-side counters.
Quadrature decoding, joystick contact interpretation and JOYTEST effects stay
inside the reconstructed receiver/device models in 2.4. Existing mouse and
joystick helpers are preceding-attempt code, not independent signal evidence.

Pin 6 fire is sampled directly into CIA-A PA6/PA7 at that same input phase,
independently of the captured drive-status bits on the same port. CIA direction and output levels travel back
to `pins.fire_output_mask`/`pins.fire_output_high`, rather than disappearing when software
changes a pin to output. Additional buttons on pins 5/9 travel through Paula's
POT input boundary; the existing decoded POT endpoint is retained without
certifying its conversion algorithm. Raw return levels are sampled from the
endpoint using its delivered drive state and sampled directly into
`paula.pins.pot_input_high`. Existing decoded POT counters remain in
`paula.pins.incoming` for next-CCK delivery; they do not establish analog
conversion timing. The direct digital inputs add no wire delay, while the
committed CIA/Paula drive levels retain their existing connector delivery phase.
Committed POTGO direction bits 9/11/13/15
produce `pins.pot_output_mask` on data positions 8/10/12/14; those data positions
also supply `pins.pot_output_high`. These are electrical drive inputs to the
attached endpoint, not a claim that its response has been implemented.

Disconnected direction/fire inputs are released high; a disconnected decoded
JOY value is a receiver question, not a zero written by the motherboard. Device
selection, neutral/release events and per-port state are serialized. External
levels must hold through the input phase; later changes appear at the next input
phase. Sub-CCK pulses and analog
capacitor charging are not modeled by that interface. Proportional devices and
four-player adapters remain 12.2 work; physical host mouse/gamepad adapters
remain 6.2. Tests distinguish connector transport from those missing reactions.

## References

- [Hardware Reference Manual, Chapter 8](../Reference/Hardware%20Reference%20Manual/08%20-%20Chapter%208%20-%20Interface%20Hardware.md): Connector pin assignments, joystick XOR decoding, quadrature and POT direction/control behavior.

- [Game ports source](../../../crates/game_ports/src/game_ports.rs): Device slots and signal sampling.
- [Machine loop source](../../../crates/machine_loop/src/machine_loop.rs): Ownership and routing to Denise, Paula, and CIA-A.
- [General Architecture](General%20Architecture.md): Placement of game ports in the machine.
- [`game_ports/pins.rs`](../../../crates/game_ports/src/pins.rs): Serializable motherboard-facing signal state and its input boundary.
