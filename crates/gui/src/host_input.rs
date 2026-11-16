//! Ordered host input, separate from serialized peripheral state.

use debugger::DebuggerSession;
use std::collections::VecDeque;

pub const INPUT_QUEUE_CAPACITY: usize = 1024;
pub const DELIVERIES_PER_FIELD: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostInputEvent {
    Key {
        code: u8,
        pressed: bool,
    },
    MouseMotion {
        port: usize,
        dx: i32,
        dy: i32,
    },
    MouseButtons {
        port: usize,
        left: bool,
        right: bool,
        middle: bool,
    },
    Joystick {
        port: usize,
        up: bool,
        down: bool,
        left: bool,
        right: bool,
        fire1: bool,
        fire2: bool,
    },
    ReleaseCapture {
        joysticks: [Option<[bool; 6]>; 2],
    },
    ReleaseAll,
}

/// Overflow cancels the backlog and releases controls, rather than losing a release.
#[derive(Debug, Default)]
pub struct InputQueue {
    events: VecDeque<HostInputEvent>,
    pub overflow_count: u64,
}

impl InputQueue {
    pub fn push(&mut self, event: HostInputEvent) {
        if matches!(event, HostInputEvent::ReleaseCapture { .. }) {
            let release = if self.events.contains(&HostInputEvent::ReleaseAll) {
                HostInputEvent::ReleaseAll
            } else {
                event
            };
            self.events.clear();
            self.events.push_back(release);
            return;
        }
        if self.events.len() == INPUT_QUEUE_CAPACITY || event == HostInputEvent::ReleaseAll {
            self.events.clear();
            self.overflow_count = self
                .overflow_count
                .wrapping_add(u64::from(event != HostInputEvent::ReleaseAll));
            self.events.push_back(HostInputEvent::ReleaseAll);
            return;
        }
        self.events.push_back(event);
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn first(&self) -> Option<HostInputEvent> {
        self.events.front().copied()
    }

    pub fn pop(&mut self) -> Option<HostInputEvent> {
        self.events.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// Only the machine owner applies events. Each button edge reaches a CCK sample.
#[derive(Debug, Default)]
pub struct InputConsumer {
    pub pending: InputQueue,
}

impl InputConsumer {
    pub fn release(&mut self, session: &mut DebuggerSession) {
        self.pending.clear();
        Self::release_controls(session);
    }

    fn release_controls(session: &mut DebuggerSession) {
        session.machine.keyboard.release_keys();
        session.machine.game_ports.release_controls();
    }

    pub fn run_slice(&mut self, session: &mut DebuggerSession, budget: usize) {
        let field = session.machine.denise.frame_builder.frame_number;
        let start = session.machine.cck;
        while session.is_running && session.machine.cck.wrapping_sub(start) < budget as u64 {
            let Some(event) = self.pending.pop() else {
                break;
            };
            match event {
                HostInputEvent::Key { code, pressed } => {
                    if pressed {
                        session.machine.keyboard.key_down(code);
                    } else {
                        session.machine.keyboard.key_up(code);
                    }
                }
                HostInputEvent::MouseMotion { port, dx, dy } => {
                    session.machine.game_ports.apply_mouse_delta(port, dx, dy)
                }
                HostInputEvent::MouseButtons {
                    port,
                    left,
                    right,
                    middle,
                } => session
                    .machine
                    .game_ports
                    .set_mouse_buttons(port, left, right, middle),
                HostInputEvent::Joystick {
                    port,
                    up,
                    down,
                    left,
                    right,
                    fire1,
                    fire2,
                } => session
                    .machine
                    .game_ports
                    .set_joystick(port, [up, down, left, right, fire1, fire2]),
                HostInputEvent::ReleaseCapture { joysticks } => {
                    Self::release_controls(session);
                    // Restore controller contacts before the next electrical sample.
                    for (port, contacts) in joysticks.into_iter().enumerate() {
                        if let Some(contacts) = contacts {
                            session.machine.game_ports.set_joystick(port, contacts);
                        }
                    }
                }
                HostInputEvent::ReleaseAll => Self::release_controls(session),
            }
            let previous_cck = session.machine.cck;
            session.run_frame_slice(1);
            if session.is_running && session.machine.cck.wrapping_sub(previous_cck) != 1 {
                self.pending.clear();
                return;
            }
            if session.machine.denise.frame_builder.frame_number != field {
                return;
            }
        }
        let used = session.machine.cck.wrapping_sub(start) as usize;
        if used < budget {
            session.run_frame_slice(budget - used);
        }
    }
}
