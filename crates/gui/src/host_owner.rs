//! Complete machine ownership and host pacing, without thread or browser handles.

use crate::host_commands::{
    HistoryCommand, HostCommand, HostObservation, HostReply, HostRequest, HostStatus, HostValue,
    InputObservation, RestoreMedia, MAX_INSPECTION_BYTES,
};
use crate::host_video::{FrameInfo, VideoExchange, VideoExchangeError};
use crate::layout::left_dock::registers::EditRegister;
use crate::playback::Playback;
use debugger::{launch::DetachedDisks, DebuggerSession};
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
    association: debugger::SaveAssociation,
    quick_associations: [debugger::SaveAssociation; debugger::session::QUICK_SLOT_COUNT],
    disk_session: u64,
    quick_disk_sessions: [u64; debugger::session::QUICK_SLOT_COUNT],
    detached_disks: DetachedDisks,
}

impl MachineOwner {
    pub fn video_storage_len(&self) -> usize {
        self.session.machine.denise.frame_builder.storage_len()
    }

    pub fn new(session: Box<DebuggerSession>) -> Self {
        let generation = 1;
        Self {
            session,
            playback: Playback::default(),
            generation,
            publication: 0,
            pending_image: true,
            shutdown: false,
            revision: 0,
            association: debugger::SaveAssociation::default(),
            quick_associations: std::array::from_fn(|_| debugger::SaveAssociation::default()),
            disk_session: generation,
            quick_disk_sessions: [generation; debugger::session::QUICK_SLOT_COUNT],
            detached_disks: DetachedDisks::default(),
        }
    }

    pub fn status(&self) -> HostStatus {
        HostStatus {
            revision: self.revision,
            generation: self.generation,
            cck: self.session.machine.cck,
            input_deliveries: self.playback.input_deliveries,
            last_input_cck: self.playback.last_input_cck,
            input_poll_seconds: f64::from(self.session.machine.agnus.field_ccks())
                / f64::from(
                    self.session
                        .machine
                        .config
                        .video_standard()
                        .color_clock_hz(),
                )
                / f64::from(crate::host_input::DELIVERIES_PER_FIELD),
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
            ports: debugger::launch::port_kinds(&self.session),
            drive_reset_sequence: self.session.machine.disk_activity_generation,
            drives: std::array::from_fn(|index| {
                let drive = &self.session.machine.floppy.drives[index];
                crate::host_commands::DriveStatus {
                    connected: drive.connected,
                    inserted: drive.disk_inserted,
                    activity_sequence: self.session.machine.paula.disk_activity.sequences[index],
                    sustained_activity: drive.connected && drive.selected && drive.motor_on,
                }
            }),
            quick_slots: std::array::from_fn(|index| self.session.has_quick_slot(index + 1)),
        }
    }

    pub fn execute(&mut self, request: HostRequest) -> HostReply {
        let observing = matches!(&request.command, HostCommand::Observe(_));
        if !matches!(
            &request.command,
            HostCommand::SupplyUtc(_)
                | HostCommand::Inspect
                | HostCommand::ReadCpu
                | HostCommand::ReadMemory { .. }
                | HostCommand::SaveSnapshot
                | HostCommand::Observe(_)
        ) {
            self.revision = self.revision.wrapping_add(1);
        }
        let value = if request.generation != self.generation {
            Err("Command belongs to an obsolete runtime generation".into())
        } else if let HostCommand::Observe(observation) = request.command {
            self.observe(observation)
        } else {
            self.apply(request.command)
        };
        if !observing && !self.session.is_running {
            self.playback.reset_timing();
        }
        HostReply {
            id: request.id,
            status: self.status(),
            value,
        }
    }

    fn observe(&self, observation: HostObservation) -> Result<HostValue, String> {
        let machine = &self.session.machine;
        match observation {
            #[cfg(not(target_arch = "wasm32"))]
            HostObservation::MachineState => {
                Ok(HostValue::MachineState(Box::new(machine.save_state())))
            }
            HostObservation::Input { key } => {
                let ports = &machine.game_ports;
                Ok(HostValue::Input(InputObservation {
                    ciaa_port_a_inputs: ports.sample_ciaa_port_a_inputs(),
                    potgor: ports.potgor(),
                    direction_pins: ports.pins.direction_pins,
                    fire1_pressed: [ports.fire1_port1(), ports.fire1_port2()],
                    key_held: key.map(|code| machine.keyboard.is_key_held(code)),
                    caps_lock_active: machine.keyboard.caps_lock_active,
                }))
            }
            HostObservation::Execution {
                data_register,
                word_address,
            } => {
                if data_register >= cpu::state::REGISTER_COUNT {
                    return Err("Invalid CPU register index".into());
                }
                Ok(HostValue::Execution {
                    data_register: machine.cpu.state.d_long(data_register),
                    memory_word: word_address.map(|address| {
                        machine
                            .physical_memory
                            .read_word_debug(address & physical_memory::MASK_PHYSICAL_ADDRESS)
                    }),
                })
            }
            HostObservation::Drive {
                drive,
                expected_adf,
            } => {
                let drive = machine
                    .floppy
                    .drives
                    .get(drive)
                    .ok_or_else(|| "Invalid floppy drive index".to_string())?;
                let tracks_match = expected_adf
                    .map(|adf| {
                        drive
                            .tracks_match_adf(&adf)
                            .ok_or_else(|| "Invalid ADF size".to_string())
                    })
                    .transpose()?;
                Ok(HostValue::Drive {
                    inserted: drive.disk_inserted,
                    tracks_match,
                })
            }
        }
    }

    pub fn collect_input(&mut self, event: crate::host_input::HostInputEvent) {
        if self.session.is_running && !self.session.pause_pending() {
            if let crate::host_input::HostInputEvent::ReleaseMouse { port } = event {
                self.playback.input.pending.discard_mouse(port);
            }
            self.playback.collected_input.push(event);
        }
    }

    fn new_generation(&mut self) {
        self.playback.collected_input.clear();
        self.playback.input.release(&mut self.session);
        self.generation = self.generation.wrapping_add(1);
        self.pending_image = !self.session.is_running || self.session.at_frame_boundary();
        self.playback.reset_timing();
    }

    fn validate_game(&self, association: &debugger::SaveAssociation) -> Result<(), String> {
        if self.association.game_id.is_some() && self.association.game_id != association.game_id {
            return Err("This save belongs to a different game".into());
        }
        Ok(())
    }

    fn finish_user_load(&mut self, ports: [debugger::launch::DeviceKind; 2]) {
        self.new_generation();
        debugger::launch::reconcile_ports(&mut self.session, ports);
    }

    fn start_disk_session(&mut self, generation: u64) {
        self.disk_session = generation;
        self.detached_disks = DetachedDisks::default();
    }

    fn restored_disk_ids<'a>(
        state: &debugger::A500State,
        association: &'a debugger::SaveAssociation,
    ) -> [Option<&'a str>; config::FLOPPY_DRIVE_COUNT] {
        std::array::from_fn(|index| {
            if state.floppy.drives[index].disk_inserted {
                association.mounted[index].as_deref()
            } else {
                None
            }
        })
    }

    fn restore_player(
        &mut self,
        json: &str,
        media: RestoreMedia,
        game_id: Option<&str>,
        ports: [debugger::launch::DeviceKind; 2],
        resume: bool,
        quick_load: bool,
    ) -> Result<HostValue, String> {
        let saved = debugger::SaveState::from_json(json).map_err(|e| e.to_string())?;
        let mut association = saved.header.association.clone();
        if association.game_id.as_deref() != game_id
            || (quick_load && self.association.game_id.as_deref() != game_id)
        {
            return Err("This save belongs to a different game".into());
        }
        let sources = self.resolve_file_media(&saved, &media, &mut association);
        let state = saved.restore(&sources).map_err(|error| error.to_string())?;
        if quick_load {
            self.detached_disks.retain_before_quick_load(
                &mut self.session,
                &self.association.mounted,
                Self::restored_disk_ids(&state, &association),
            );
        }
        self.session.load_file_state(state, sources);
        self.association = association;
        self.finish_user_load(ports);
        if !quick_load {
            self.start_disk_session(self.generation);
        }
        if resume {
            self.session.toggle_run();
        }
        Ok(HostValue::PlayerRestored(self.association.clone()))
    }

    fn resolve_file_media(
        &self,
        saved: &debugger::SaveState,
        candidates: &RestoreMedia,
        association: &mut debugger::SaveAssociation,
    ) -> debugger::MediaSources {
        let current = self.session.file_sources();
        let roms = candidates.roms.iter().chain(&current.rom);
        let rom = if saved.synthetic_rom() {
            None
        } else {
            roms.clone()
                .find(|image| saved.rom_matches(&image.bytes))
                .or_else(|| roms.clone().next())
                .cloned()
        };
        let supplied = candidates
            .disks
            .iter()
            .map(|(id, image)| (Some(id.as_str()), image));
        let mounted = self
            .association
            .mounted
            .iter()
            .zip(&current.disks)
            .filter_map(|(id, image)| image.as_ref().map(|image| (id.as_deref(), image)));
        let detached = self
            .detached_disks
            .file_sources()
            .map(|(id, image)| (Some(id), image));
        let available: Vec<_> = supplied.chain(mounted).chain(detached).collect();
        let disks = std::array::from_fn(|drive| {
            if !saved.disk_inserted(drive) {
                return None;
            }
            let saved_id = association.mounted[drive].as_deref();
            let selected = available
                .iter()
                .copied()
                .filter(|(id, _)| id.is_some() && *id == saved_id)
                .find(|(_, image)| saved.disk_matches(drive, &image.bytes))
                .or_else(|| {
                    available
                        .iter()
                        .copied()
                        .find(|(_, image)| saved.disk_matches(drive, &image.bytes))
                });
            if let Some((id, image)) = selected {
                if candidates.remap_manual_ids && association.game_id.is_none() {
                    association.mounted[drive] = id.map(str::to_owned);
                }
                return Some(image.clone());
            }
            // Let the codec describe an incorrect authorized candidate precisely.
            available
                .iter()
                .copied()
                .find(|(id, image)| {
                    (id.is_some() && *id == saved_id)
                        || image
                            .name
                            .as_deref()
                            .is_some_and(|name| saved.disk_name(drive) == Some(name))
                })
                .map(|(_, image)| image.clone())
        });
        debugger::MediaSources { rom, disks }
    }

    fn apply(&mut self, command: HostCommand) -> Result<HostValue, String> {
        match command {
            HostCommand::Run => {
                if !self.session.is_running {
                    self.session.toggle_run();
                }
            }
            HostCommand::Pause => {
                self.playback.collected_input.clear();
                self.playback.input.release(&mut self.session);
                self.session.request_pause();
            }
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
            HostCommand::ResetPlayer(startup) => {
                if let Some(startup) = startup {
                    if self.association.game_id.as_ref() != Some(&startup.game_id) {
                        return Err("Startup disks belong to a different game".into());
                    }
                    self.detached_disks.restore_startup(
                        &mut self.session,
                        &self.association.mounted,
                        &startup.disks,
                    )?;
                    self.association.mounted = startup.disks.map(|disk| disk.map(|disk| disk.id));
                }
                self.session.reset();
                self.new_generation();
                self.session.toggle_run();
                return Ok(HostValue::PlayerReset(self.association.mounted.clone()));
            }
            HostCommand::SupplyUtc(utc) => self.session.machine.rtc.supply_host_utc(utc),
            HostCommand::LoadRtcBattery(battery) => self.session.machine.rtc.load_battery(battery),
            HostCommand::Shutdown => {
                self.session.is_running = false;
                self.shutdown = true;
            }
            HostCommand::SaveSnapshot => {
                let mut saved = debugger::SaveState::capture(
                    &self.session.machine,
                    self.session.file_sources(),
                )
                .map_err(|error| error.to_string())?;
                saved.header.timestamp = crate::host_clock::utc_now().max(0) as u64;
                saved.header.association = self.association.clone();
                return saved
                    .to_json()
                    .map(HostValue::Snapshot)
                    .map_err(|e| e.to_string());
            }
            HostCommand::RestorePlayer {
                json,
                media,
                game_id,
                ports,
                resume,
            } => {
                return self.restore_player(&json, media, game_id.as_deref(), ports, resume, false)
            }
            HostCommand::LoadPlayerQuick {
                json,
                media,
                game_id,
                ports,
            } => {
                return self.restore_player(&json, media, Some(&game_id), ports, true, true);
            }
            HostCommand::SaveQuickSlot(slot) => {
                self.session
                    .save_quick_slot(slot)
                    .map_err(|e| e.to_string())?;
                self.quick_associations[slot - 1] = self.association.clone();
                self.quick_disk_sessions[slot - 1] = self.disk_session;
            }
            HostCommand::LoadQuickSlot { slot, ports } => {
                let index = slot.wrapping_sub(1);
                if let Some(association) = self.quick_associations.get(index) {
                    self.validate_game(association)?;
                    if self.quick_disk_sessions[index] == self.disk_session {
                        if let Some(state) = &self.session.quick_slots[index] {
                            let restored = Self::restored_disk_ids(state, association);
                            self.detached_disks.retain_before_quick_load(
                                &mut self.session,
                                &self.association.mounted,
                                restored,
                            );
                        }
                    }
                }
                self.session
                    .load_quick_slot(slot)
                    .map_err(|e| e.to_string())?;
                self.association = self.quick_associations[index].clone();
                self.finish_user_load(ports);
                let disk_session = self.quick_disk_sessions[index];
                if disk_session != self.disk_session {
                    self.start_disk_session(disk_session);
                }
                return Ok(HostValue::PlayerRestored(self.association.clone()));
            }
            HostCommand::Launch {
                setup,
                association,
                rom_name,
                disk_names,
            } => {
                for (drive, id) in association.mounted.iter().enumerate() {
                    if let Some(id) = id {
                        if setup.disks[drive].is_none() {
                            return Err(format!("ADF for DF{drive} ({id}) is unavailable"));
                        }
                    }
                }
                let mut replacement = setup.prepare()?;
                replacement.name_file_sources(rom_name, disk_names);
                replacement
                    .machine
                    .rtc
                    .supply_host_utc(crate::host_clock::utc_now());
                replacement
                    .machine
                    .rtc
                    .load_battery(self.session.machine.rtc.battery_data());
                replacement.set_debugger_enabled(false);
                replacement.toggle_run();
                self.session = replacement;
                self.association = association;
                self.new_generation();
                self.start_disk_session(self.generation);
            }
            HostCommand::MountDisk {
                drive,
                bytes,
                disk_id,
                name,
            } => {
                let mounted_id = self
                    .association
                    .mounted
                    .get(drive)
                    .and_then(Option::as_deref);
                self.detached_disks.mount(
                    &mut self.session,
                    drive,
                    mounted_id,
                    disk_id.as_deref(),
                    bytes.as_deref(),
                    name.as_deref(),
                )?;
                self.association.mounted[drive] = disk_id;
            }
            HostCommand::LoadRom(source) => {
                self.session
                    .load_named_rom(source)
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
            HostCommand::ReconfigurePorts(ports) => {
                self.new_generation();
                debugger::launch::reconcile_ports(&mut self.session, ports);
            }
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
            if self.playback.reset_seen {
                self.new_generation();
            }
            if !self.session.is_running {
                self.playback.collected_input.clear();
                self.playback.input.release(&mut self.session);
            }
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
