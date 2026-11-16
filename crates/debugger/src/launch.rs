//! Atomic host launch preparation, outside CCK execution.

use crate::DebuggerSession;
use config::A500Config;
use machine_loop::game_ports::{Joystick, Mouse, PortDevice};
use serde::{Deserialize, Serialize};

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
        for (index, kind) in self.ports.into_iter().enumerate() {
            if let Some(device) = session.machine.game_ports.device_mut(index) {
                *device = match kind {
                    DeviceKind::Disconnected => PortDevice::None,
                    DeviceKind::Mouse => PortDevice::Mouse(Mouse::new()),
                    DeviceKind::Joystick => PortDevice::Joystick(Joystick::new()),
                };
            }
        }
        session.machine.floppy.set_connections(self.drives)?;
        for (drive, disk) in self.disks.into_iter().enumerate() {
            mount(&mut session, drive, disk.as_deref())?;
        }
        Ok(session)
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
    let drive = session
        .machine
        .floppy
        .drives
        .get_mut(drive)
        .ok_or("Invalid drive number")?;
    if !drive.connected {
        return if bytes.is_some() {
            Err("Drive is disconnected; edit the configuration".into())
        } else {
            Ok(())
        };
    }
    if let Some(bytes) = bytes {
        if !drive.insert_disk(bytes) {
            return Err(format!("Unsupported ADF length: {} bytes", bytes.len()));
        }
    } else {
        drive.eject_disk();
    }
    Ok(())
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
