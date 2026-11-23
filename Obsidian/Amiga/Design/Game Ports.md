---
title: "Game Ports"
aliases: ["Gameports", "Controller Ports", "Port 1", "Port 2"]
tags: ["amiga", "design", "game_ports", "input", "denise", "paula", "cia"]
category: "Design"
subsystem: "game_ports"
status: "active"
created: 2026-09-13
updated: 2026-11-23
related: ["[Mouse.md](Mouse.md)", "[Joystick.md](Joystick.md)", "[Denise.md](Denise.md)", "[Paula.md](Paula.md)", "[CIA.md](CIA.md)", "[Machine Loop.md](Machine%20Loop.md)", "[General Architecture.md](General%20Architecture.md)"]
tracked_paths:
  - "crates/game_ports"
---

# Game Ports

The two game ports represent the Amiga's controller sockets. Each can have
an emulated [mouse](Mouse.md), an emulated [joystick](Joystick.md) or no
device. The frontend turns host input into emulated device state. A host mouse
can supply mouse movement and buttons, and keyboard controls on the host can
map to joystick directions and buttons. The Amiga keyboard is a separate
peripheral.

The inputs and registers for a game port are spread across [Denise](Denise.md),
[Paula](Paula.md) and [CIA-A](CIA.md). The game ports model the devices and
their signals. Each chip owns its corresponding register state and behavior.

The `game_ports` crate owns the `mouse.rs` and `joystick.rs` device modules
and exposes their primary types. [Mouse](Mouse.md) and [Joystick](Joystick.md)
describe the two devices in more detail. Their signals pass through the
machine's connector routing. Denise retains its JOY counters, and Paula
retains its chip-side POT state.

[MachineLoop](Machine%20Loop.md) owns the game ports alongside the chips. It
samples the port signals and delivers them to the receiving chips, which
communicate with the devices without holding direct references to them.

## Host destinations

Every mouse or joystick event names a physical destination port. The input
helpers deliver the event only if that port has the matching device type.
A full host input release neutralizes both ports and the keyboard. A mouse
release clears its buttons and pending movement. Both operations preserve
device types and mouse quadrature position. The [GUI input rules](Application/GUI.md#input-and-output)
define when the frontend releases each control and how it selects host sources.
Serialized controller state holds no host handles for them.

The frontend uses the current committed controls to select each port's device
after exact core restoration and whenever it updates controls while stopped.
If a port keeps the same device type, the device retains its phase, its
contacts release and any queued motion clears. If the type changes, the replacement starts in a neutral state.
Denise, Paula and CIA keep their receiver state. When execution resumes, the
normal CCK connector phases deliver the new levels with no machine reset or
immediate chip-register write.

## Connector delivery

The serializable `GamePortsPins` state in [pins.rs](../../../crates/game_ports/src/pins.rs)
is exposed as `game_ports.pins`. It holds direction outputs and delivered
fire/POT drive levels separately from the state of the connected devices.

Each port's low nibble in `pins.direction_pins` carries the levels on
connector pins 1-4, which are high when released. The
[machine](Machine%20Loop.md#captured-connector-transport-and-pending-bus-words)
samples these external levels into Denise's `game_port_inputs` before engine
work in the current CCK. The event time already marks arrival at the receiver
pin, so the machine needs no extra pending copy. The machine delivers pin
levels and does not copy device-side counters into JOY registers.

The machine advances the devices before sampling their outputs. Mouse
movement queues quadrature transitions. Each axis advances by at most one
transition per CCK. Joystick contacts supply static active-low levels. Denise decodes the four wrapping byte counters
and keeps their low two bits consistent with the delivered pins. JOYTEST
changes only bits 7..2 of each axis on both ports. The former device-side
JOY register helpers are removed. The per-CCK waveform rate belongs to the
functional model. It provides neither a calibrated physical mouse speed
nor a half-CCK input multiplexer.

At the same input phase, the machine samples the pin 6 fire level directly
into CIA-A PA6/PA7. Drive-status bits on that CIA port are captured
independently. CIA direction and output levels travel back through
`pins.fire_output_mask`/`pins.fire_output_high`, including when software
sets a pin as an output. Additional buttons on pins 5/9 use Paula's POT
input boundary. The model retains the existing decoded POT endpoint, whose
conversion algorithm remains unverified here. The machine samples raw return
levels from the endpoint using its delivered drive state and passes them
directly into `paula.pins.pot_input_high`. The existing decoded POT counters
stay in `paula.pins.incoming` for next-CCK delivery and provide no evidence of
analog conversion timing. The direct digital inputs add no wire delay.
Committed CIA/Paula drive levels retain their existing connector delivery
phase.

Once POTGO commits, direction bits 9/11/13/15 determine which data
positions 8/10/12/14 are enabled as outputs in `pins.pot_output_mask`.
The same data positions supply the levels in `pins.pot_output_high`.
Together, these values drive the attached endpoint. An enabled
low output or a pressed button pulls a digital return low. Otherwise the
released input pulls high. POTGOR reports only the four data positions. FIRE
uses the same rule for resolving a low drive. Analog capacitor conversion
remains deferred.

Disconnected direction and fire inputs are released high. Denise determines
the decoded JOY value from what it receives. The motherboard does not write
zero for a disconnected port. Serialization includes device selection,
neutral and release events, and state for each port. External levels must
hold through the input phase. Later changes appear at the next input phase.
The interface leaves sub-CCK pulses and analog capacitor charging unmodeled.
Proportional devices and four-player adapters remain 12.2 work. The
[frontend](Application/GUI.md#input-and-output) now supplies physical mouse
and gamepad events through named host configurations. Physical-controller
diagnostics and keyboard matrix fidelity remain 6.2 work. Tests cover digital
direction, data and button combinations, quadrature wrapping, JOYTEST and
continuation of queued movement after snapshot restore.

## References

- [Hardware Reference Manual, Chapter 8](../Reference/Hardware%20Reference%20Manual/08%20-%20Chapter%208%20-%20Interface%20Hardware.md): Connector pin assignments, joystick XOR decoding, quadrature and POT direction/control behavior.

- [Game ports source](../../../crates/game_ports/src/game_ports.rs): Device slots and signal sampling.
- [Machine loop source](../../../crates/machine_loop/src/machine_loop.rs): Ownership and routing to Denise, Paula, and CIA-A.
- [General Architecture](General%20Architecture.md): Placement of game ports in the machine.
- [`game_ports/pins.rs`](../../../crates/game_ports/src/pins.rs): Serializable motherboard-facing signal state and its input boundary.
