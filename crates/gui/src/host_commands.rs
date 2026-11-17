//! Owned commands and coherent results crossing the host machine-owner boundary.

use crate::layout::left_dock::registers::EditRegister;
use cpu::CpuState;

pub const HOST_QUEUE_CAPACITY: usize = 64;
pub const MAX_INSPECTION_BYTES: usize = 4096;

#[derive(Debug)]
pub enum HistoryCommand {
    Backward,
    Forward,
    BackwardField,
    ForwardField,
    Live,
    Index(usize),
    Cck(u64),
    ToggleRecording,
    Capacity(usize),
}

#[derive(Debug)]
pub enum HostCommand {
    Run,
    Pause,
    StepInstruction,
    StepCck,
    StepField,
    Slower,
    Faster,
    NormalSpeed,
    SetDebuggerEnabled(bool),
    Reset,
    ResetWarm,
    ReadCpu,
    Inspect,
    History(HistoryCommand),
    ClearTrace,
    SetBreakpoints(debugger::BreakpointManager),
    SetMemoryBase(u32),
    ToggleStatusFlag(u16),
    ReadMemory {
        address: u32,
        length: usize,
    },
    WriteMemory {
        address: u32,
        bytes: Vec<u8>,
        restart_instruction: bool,
    },
    SetRegister {
        register: EditRegister,
        value: u32,
    },
    LoadBinary {
        address: u32,
        bytes: Vec<u8>,
        prime: bool,
    },
    Launch {
        setup: debugger::launch::MachineSetup,
        association: debugger::SaveAssociation,
    },
    MountDisk {
        drive: usize,
        bytes: Option<Vec<u8>>,
        disk_id: Option<String>,
    },
    LoadRom(Vec<u8>),
    ReconfigurePorts([debugger::launch::DeviceKind; 2]),
    SaveSnapshot,
    RestorePlayer {
        json: String,
        game_id: Option<String>,
        ports: [debugger::launch::DeviceKind; 2],
        resume: bool,
    },
    SaveQuickSlot(usize),
    LoadQuickSlot {
        slot: usize,
        ports: [debugger::launch::DeviceKind; 2],
    },
    SupplyUtc(i64),
    LoadRtcBattery(rtc::RtcBattery),
    Shutdown,
}

#[derive(Debug)]
pub struct HostRequest {
    pub id: u64,
    pub generation: u64,
    pub command: HostCommand,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HostStatus {
    pub revision: u64,
    pub generation: u64,
    pub cck: u64,
    pub input_deliveries: u64,
    pub last_input_cck: u64,
    pub input_poll_seconds: f64,
    pub field_number: u64,
    pub publication: u64,
    pub running: bool,
    pub continuous_run: bool,
    pub pause_pending: bool,
    pub speed: f64,
    pub fps: f64,
    pub battery: rtc::RtcBattery,
    pub rtc_time: i64,
    pub shutdown: bool,
    pub halted: bool,
    pub stopped: bool,
    pub drive_reset_sequence: u64,
    pub drives: [DriveStatus; config::FLOPPY_DRIVE_COUNT],
    pub ports: [debugger::launch::DeviceKind; 2],
    pub quick_slots: [bool; debugger::session::QUICK_SLOT_COUNT],
}

#[derive(Debug)]
pub enum HostValue {
    Done,
    Cpu(Box<CpuState>),
    Memory { address: u32, bytes: Vec<u8> },
    Snapshot(String),
    PlayerRestored(debugger::SaveAssociation),
    Inspection(Box<crate::host_inspection::HostInspection>),
}

#[derive(Debug)]
pub struct HostReply {
    pub id: u64,
    pub status: HostStatus,
    pub value: Result<HostValue, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriveStatus {
    pub connected: bool,
    pub inserted: bool,
    pub activity_sequence: u64,
    pub sustained_activity: bool,
}
