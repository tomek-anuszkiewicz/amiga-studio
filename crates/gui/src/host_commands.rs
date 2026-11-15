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
    LoadRom(Vec<u8>),
    SaveSnapshot,
    LoadSnapshot(String),
    SaveQuickSlot(usize),
    LoadQuickSlot(usize),
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
    pub quick_slots: [bool; debugger::session::QUICK_SLOT_COUNT],
}

#[derive(Debug)]
pub enum HostValue {
    Done,
    Cpu(Box<CpuState>),
    Memory { address: u32, bytes: Vec<u8> },
    Snapshot(String),
    Inspection(Box<crate::host_inspection::HostInspection>),
}

#[derive(Debug)]
pub struct HostReply {
    pub id: u64,
    pub status: HostStatus,
    pub value: Result<HostValue, String>,
}
