//! Complete machine ownership and host pacing, without thread or browser handles.

use crate::host_commands::{
    HistoryCommand, HostCommand, HostReply, HostRequest, HostStatus, HostValue,
    MAX_INSPECTION_BYTES,
};
use crate::host_video::{FrameInfo, VideoExchange, VideoExchangeError};
use crate::layout::left_dock::registers::EditRegister;
use crate::playback::Playback;
use debugger::DebuggerSession;
use std::time::Duration;

#[derive(Debug)]
pub struct MachineOwner {
    session: Box<DebuggerSession>,
    playback: Playback,
    generation: u64,
    publication: u64,
    pending_image: bool,
    shutdown: bool,
    revision: u64,
}

impl MachineOwner {
    pub fn video_storage_len(&self) -> usize {
        self.session.machine.denise.frame_builder.storage_len()
    }

    pub fn new(session: Box<DebuggerSession>) -> Self {
        Self {
            session,
            playback: Playback::default(),
            generation: 1,
            publication: 0,
            pending_image: true,
            shutdown: false,
            revision: 0,
        }
    }

    pub fn status(&self) -> HostStatus {
        HostStatus {
            revision: self.revision,
            generation: self.generation,
            cck: self.session.machine.cck,
            field_number: self.session.machine.denise.frame_builder.frame_number,
            publication: self.publication,
            running: self.session.is_running,
            continuous_run: self.session.continuous_run(),
            pause_pending: self.session.pause_pending(),
            speed: self.playback.speed(),
            fps: self.playback.fps(),
            battery: self.session.machine.rtc.battery_data(),
            rtc_time: self.session.machine.rtc.time(),
            shutdown: self.shutdown,
            halted: self.session.machine.cpu.state.is_halted(),
            stopped: self.session.machine.cpu.state.is_stopped(),
            quick_slots: std::array::from_fn(|index| self.session.has_quick_slot(index + 1)),
        }
    }

    pub fn execute(&mut self, request: HostRequest) -> HostReply {
        if !matches!(
            &request.command,
            HostCommand::SupplyUtc(_)
                | HostCommand::Inspect
                | HostCommand::ReadCpu
                | HostCommand::ReadMemory { .. }
                | HostCommand::SaveSnapshot
        ) {
            self.revision = self.revision.wrapping_add(1);
        }
        let value = if request.generation != self.generation {
            Err("Command belongs to an obsolete runtime generation".into())
        } else {
            self.apply(request.command)
        };
        if !self.session.is_running {
            self.playback.reset_timing();
        }
        HostReply {
            id: request.id,
            status: self.status(),
            value,
        }
    }

    fn new_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.pending_image = !self.session.is_running || self.session.at_frame_boundary();
        self.playback.reset_timing();
    }

    fn apply(&mut self, command: HostCommand) -> Result<HostValue, String> {
        match command {
            HostCommand::Run => {
                if !self.session.is_running {
                    self.session.toggle_run();
                }
            }
            HostCommand::Pause => self.session.request_pause(),
            HostCommand::Slower => self.playback.slower(),
            HostCommand::Faster => self.playback.faster(),
            HostCommand::NormalSpeed => self.playback.normal_speed(),
            HostCommand::SetDebuggerEnabled(enabled) => self.session.set_debugger_enabled(enabled),
            HostCommand::Reset => {
                self.session.reset();
                self.new_generation();
            }
            HostCommand::ResetWarm => {
                self.session.reset_warm();
                self.new_generation();
            }
            HostCommand::SupplyUtc(utc) => self.session.machine.rtc.supply_host_utc(utc),
            HostCommand::LoadRtcBattery(battery) => self.session.machine.rtc.load_battery(battery),
            HostCommand::Shutdown => {
                self.session.is_running = false;
                self.shutdown = true;
            }
            HostCommand::SaveSnapshot => {
                return self
                    .session
                    .save_state_to_json()
                    .map(HostValue::Snapshot)
                    .map_err(|error| error.to_string());
            }
            HostCommand::LoadSnapshot(json) => {
                self.session
                    .load_state_from_json(&json)
                    .map_err(|error| error.to_string())?;
                self.new_generation();
            }
            HostCommand::SaveQuickSlot(slot) => self
                .session
                .save_quick_slot(slot)
                .map_err(|error| error.to_string())?,
            HostCommand::LoadQuickSlot(slot) => {
                self.session
                    .load_quick_slot(slot)
                    .map_err(|error| error.to_string())?;
                self.new_generation();
            }
            HostCommand::LoadRom(bytes) => {
                self.session
                    .load_rom(&bytes)
                    .map_err(|error| error.to_string())?;
                self.new_generation();
            }
            HostCommand::LoadBinary {
                address,
                bytes,
                prime,
            } => {
                self.session
                    .load_binary(address, &bytes, prime)
                    .map_err(|error| error.to_string())?;
                self.new_generation();
            }
            paused => {
                if self.session.is_running {
                    return Err("Pause execution before inspecting or editing the machine".into());
                }
                return self.apply_paused(paused);
            }
        }
        Ok(HostValue::Done)
    }

    fn apply_paused(&mut self, command: HostCommand) -> Result<HostValue, String> {
        match command {
            HostCommand::Inspect => {
                return Ok(HostValue::Inspection(Box::new(
                    crate::host_inspection::HostInspection::capture(&self.session),
                )));
            }
            HostCommand::History(command) => match command {
                HistoryCommand::Backward => self.session.step_backward(),
                HistoryCommand::Forward => self.session.step_forward(),
                HistoryCommand::BackwardField => self.session.step_backward_frame(),
                HistoryCommand::ForwardField => self.session.step_forward_frame(),
                HistoryCommand::Live => self.session.jump_to_live_head(),
                HistoryCommand::Index(index) => self.session.scrub_to_frame(index),
                HistoryCommand::Cck(cck) => self.session.jump_to_cck(cck),
                HistoryCommand::ToggleRecording => {
                    self.session.temporal.toggle_recording();
                }
                HistoryCommand::Capacity(capacity) => self.session.temporal.set_capacity(capacity),
            },
            HostCommand::ClearTrace => self.session.debugger.trace.clear(),
            HostCommand::SetBreakpoints(breakpoints) => {
                self.session.debugger.breakpoints = breakpoints
            }
            HostCommand::SetMemoryBase(base) => self.session.capture_memory_snapshot(base),
            HostCommand::ToggleStatusFlag(mask) => {
                let state = &mut self.session.machine.cpu.state;
                state.set_sr(state.sr() ^ mask);
            }
            HostCommand::StepInstruction => self.session.step_instruction(),
            HostCommand::StepCck => self.session.step_cck(),
            HostCommand::StepField => {
                self.session.step_frame();
                return Ok(HostValue::Done);
            }
            HostCommand::ReadCpu => {
                return Ok(HostValue::Cpu(Box::new(
                    self.session.machine.cpu.state.clone(),
                )))
            }
            HostCommand::ReadMemory { address, length } => {
                if length > MAX_INSPECTION_BYTES {
                    return Err("Memory inspection range is too large".into());
                }
                let bytes = (0..length)
                    .map(|offset| {
                        self.session.machine.physical_memory.read_byte_debug(
                            address.wrapping_add(offset as u32)
                                & physical_memory::MASK_PHYSICAL_ADDRESS,
                        )
                    })
                    .collect();
                return Ok(HostValue::Memory { address, bytes });
            }
            HostCommand::WriteMemory {
                address,
                bytes,
                restart_instruction,
            } => {
                for (offset, byte) in bytes.into_iter().enumerate() {
                    self.session.machine.physical_memory.write_byte_debug(
                        address.wrapping_add(offset as u32)
                            & physical_memory::MASK_PHYSICAL_ADDRESS,
                        byte,
                    );
                }
                if restart_instruction {
                    self.session.invalidate_instruction_tracking();
                }
            }
            HostCommand::SetRegister { register, value } => {
                let machine = &mut self.session.machine;
                match register {
                    EditRegister::D(index) | EditRegister::A(index)
                        if index >= cpu::state::REGISTER_COUNT =>
                    {
                        return Err("Invalid CPU register index".into());
                    }
                    EditRegister::D(index) => machine.cpu.state.set_d_long(index, value),
                    EditRegister::A(index) => machine.cpu.state.set_a_long(index, value),
                    EditRegister::SR => machine.cpu.state.set_sr(value as u16),
                    EditRegister::USP => machine.cpu.state.set_usp(value),
                    EditRegister::SSP => machine.cpu.state.set_ssp(value),
                    EditRegister::PC => {
                        machine.cpu.set_pc_and_prime_prefetch(
                            value & physical_memory::MASK_PHYSICAL_ADDRESS,
                            &mut machine.physical_memory,
                        );
                        self.session.invalidate_instruction_tracking();
                    }
                }
            }
            _ => return Err("Invalid paused machine command".into()),
        }
        self.pending_image = true;
        Ok(HostValue::Done)
    }

    /// Runs due bounded work, preserving Playback's selected-image batch policy.
    pub fn advance(&mut self, now: f64, elapsed: impl Fn() -> f64) {
        if self.session.is_running && !self.pending_image && !self.shutdown {
            self.pending_image = self.playback.advance(now, &mut self.session, elapsed);
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// Seeds/switches the owned allocation only after the exchange grants access.
    pub fn publish(&mut self, exchange: &VideoExchange) -> Result<bool, VideoExchangeError> {
        if !self.pending_image {
            return Ok(false);
        }
        let complete = self.session.at_frame_boundary();
        exchange.try_publish(|spare| {
            let frame = &mut self.session.machine.denise.frame_builder;
            if !frame.exchange_buffer(spare) {
                return None;
            }
            self.publication = self.publication.wrapping_add(1);
            self.revision = self.revision.wrapping_add(1);
            self.pending_image = false;
            Some(FrameInfo {
                width: frame.width as usize,
                height: frame.height as usize,
                row_stride: frame.row_stride(),
                field_number: frame.frame_number,
                publication: self.publication,
                cck: self.session.machine.cck,
                generation: self.generation,
                interlaced: frame.interlaced,
                field_odd: frame.field_odd,
                complete,
            })
        })
    }

    pub fn next_delay(&self, now: f64) -> Option<Duration> {
        if self.pending_image || !self.session.is_running || self.shutdown {
            None
        } else {
            Some(self.playback.worker_delay(now))
        }
    }
}
