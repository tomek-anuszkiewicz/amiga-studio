//! Atomic host launch preparation, outside CCK execution.

use crate::DebuggerSession;
use config::{A500Config, FLOPPY_DRIVE_COUNT};
use machine_loop::floppy::{FloppyDisk, FloppyDrive};
use machine_loop::game_ports::{Joystick, Mouse, PortDevice};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceKind {
    Disconnected,
    Mouse,
    Joystick,
}

#[derive(Debug)]
pub struct MachineSetup {
    pub config: A500Config,
    pub ports: [DeviceKind; 2],
    pub drives: [bool; 4],
    pub rom: Vec<u8>,
    pub disks: [Option<Vec<u8>>; 4],
}

impl MachineSetup {
    /// Construction and all image preparation succeed before a live owner changes.
    pub fn prepare(self) -> Result<Box<DebuggerSession>, String> {
        let mut session = Box::new(DebuggerSession::from_config(self.config));
        session.load_rom(&self.rom).map_err(|e| e.to_string())?;
        reconcile_ports(&mut session, self.ports);
        session.machine.floppy.set_connections(self.drives)?;
        for (drive, disk) in self.disks.into_iter().enumerate() {
            mount(&mut session, drive, disk.as_deref())?;
        }
        Ok(session)
    }
}

/// Replaces changed endpoints only; receiver registers and device phases stay local.
pub fn reconcile_ports(session: &mut DebuggerSession, kinds: [DeviceKind; 2]) {
    let previous = port_kinds(session);
    for (index, kind) in kinds.into_iter().enumerate() {
        if previous[index] != kind {
            if let Some(device) = session.machine.game_ports.device_mut(index) {
                *device = match kind {
                    DeviceKind::Disconnected => PortDevice::None,
                    DeviceKind::Mouse => PortDevice::Mouse(Mouse::new()),
                    DeviceKind::Joystick => PortDevice::Joystick(Joystick::new()),
                };
            }
        }
    }
}

/// Reject unsupported image layouts before preparing or replacing a drive.
pub fn disk_image_is_supported(bytes: &[u8]) -> bool {
    bytes.len() == machine_loop::floppy::FORMATTED_DISK_BYTES
}

pub fn mount(
    session: &mut DebuggerSession,
    drive: usize,
    bytes: Option<&[u8]>,
) -> Result<(), String> {
    if let Some(drive) = drive_for_mount(session, drive, bytes.is_some())? {
        if let Some(bytes) = bytes {
            if !drive.insert_disk(bytes) {
                return Err(unsupported_image(bytes));
            }
        } else {
            drive.eject_disk();
        }
        session.machine.refresh_disk_activity_context();
    }
    Ok(())
}

fn drive_for_mount(
    session: &mut DebuggerSession,
    index: usize,
    inserting: bool,
) -> Result<Option<&mut FloppyDrive>, String> {
    let drive = session
        .machine
        .floppy
        .drives
        .get_mut(index)
        .ok_or("Invalid drive number")?;
    if inserting && !drive.connected {
        return Err("Drive is disconnected; edit the configuration".into());
    }
    Ok(drive.connected.then_some(drive))
}

fn prepare_disk(bytes: Option<&[u8]>) -> Result<Option<FloppyDisk>, String> {
    bytes
        .map(|bytes| FloppyDisk::from_adf(bytes).ok_or_else(|| unsupported_image(bytes)))
        .transpose()
}

fn unsupported_image(bytes: &[u8]) -> String {
    format!("Unsupported ADF length: {} bytes", bytes.len())
}

/// Player disks kept outside the machine while ejected, indexed by last drive and ID.
#[derive(Debug, Default)]
pub struct DetachedDisks([HashMap<String, FloppyDisk>; FLOPPY_DRIVE_COUNT]);

impl DetachedDisks {
    fn available_drives<'a>(&'a self, id: &'a str) -> impl Iterator<Item = usize> + 'a {
        self.0
            .iter()
            .enumerate()
            .filter_map(move |(index, disks)| disks.contains_key(id).then_some(index))
    }

    fn unique_drive(&self, id: &str) -> Result<Option<usize>, String> {
        let mut drives = self.available_drives(id);
        let first = drives.next();
        if drives.next().is_some() {
            return Err(
                "Several ejected copies of this disk are available; use a previous drive".into(),
            );
        }
        Ok(first)
    }

    pub fn mount(
        &mut self,
        session: &mut DebuggerSession,
        index: usize,
        mounted_id: Option<&str>,
        requested_id: Option<&str>,
        bytes: Option<&[u8]>,
    ) -> Result<(), String> {
        if mounted_id.is_none() && requested_id.is_none() {
            return mount(session, index, bytes);
        }
        let Some(drive) =
            drive_for_mount(session, index, requested_id.is_some() || bytes.is_some())?
        else {
            return Ok(());
        };
        let next = if let Some(id) = requested_id {
            if mounted_id == Some(id) && drive.disk_inserted {
                return Ok(());
            }
            let previous_drive = if self.0[index].contains_key(id) {
                Some(index)
            } else {
                self.unique_drive(id)?
            };
            match previous_drive {
                Some(previous_drive) => self.0[previous_drive].remove(id),
                None => prepare_disk(Some(bytes.ok_or("Disk is unavailable; return to setup")?))?,
            }
        } else {
            prepare_disk(bytes)?
        };

        if let (Some(id), Some(ejected)) = (mounted_id, drive.replace_disk(next)) {
            self.0[index].insert(id.to_owned(), ejected);
        }
        session.machine.refresh_disk_activity_context();
        Ok(())
    }

    /// Keeps live disks the slot does not replace and gives a matching slot copy precedence.
    pub fn retain_before_quick_load(
        &mut self,
        session: &mut DebuggerSession,
        mounted: &[Option<String>; FLOPPY_DRIVE_COUNT],
        restored: [Option<&str>; FLOPPY_DRIVE_COUNT],
    ) {
        for (index, (id, drive)) in mounted
            .iter()
            .zip(&mut session.machine.floppy.drives)
            .enumerate()
        {
            if let Some(id) = id {
                if let Some(disk) = drive.replace_disk(None) {
                    self.0[index].insert(id.clone(), disk);
                }
            }
        }
        let unmatched: [_; FLOPPY_DRIVE_COUNT] = std::array::from_fn(|index| {
            restored[index].filter(|id| self.0[index].remove(*id).is_none())
        });
        for id in unmatched.iter().flatten() {
            // Match across drives only when both sides identify a single copy.
            if unmatched
                .iter()
                .filter(|entry| **entry == Some(*id))
                .count()
                == 1
            {
                if let Ok(Some(index)) = self.unique_drive(id) {
                    self.0[index].remove(*id);
                }
            }
        }
    }
}

pub fn port_kinds(session: &DebuggerSession) -> [DeviceKind; 2] {
    state_port_kinds(&session.machine.game_ports)
}

pub fn state_port_kinds(ports: &machine_loop::game_ports::GamePorts) -> [DeviceKind; 2] {
    [&ports.state.port1, &ports.state.port2].map(|device| match device {
        PortDevice::None => DeviceKind::Disconnected,
        PortDevice::Mouse(_) => DeviceKind::Mouse,
        PortDevice::Joystick(_) => DeviceKind::Joystick,
    })
}
