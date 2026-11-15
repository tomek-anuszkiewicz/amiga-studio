//! Prepared inspector contents, owned by the GUI and retained during continuous Run.

use crate::layout::left_dock::{
    disassembly::DisassemblyPresentation, engine_status::EnginePresentation,
    microcode::MicrocodePresentation, registers::RegisterPresentation,
};
use crate::layout::main_viewport::temporal_bar::TemporalPresentation;
use crate::layout::right_dock::{
    breakpoints_panel::BreakpointsPresentation, memory_hex::MemoryPresentation,
    trace_log::TracePresentation,
};

#[derive(Debug, Default)]
pub struct Inspection {
    pub(crate) ready: bool,
    pub(crate) frozen: bool,
    pub(crate) registers: RegisterPresentation,
    pub(crate) engine: EnginePresentation,
    pub(crate) microcode: MicrocodePresentation,
    pub(crate) disassembly: DisassemblyPresentation,
    pub(crate) memory: MemoryPresentation,
    pub(crate) breakpoints: BreakpointsPresentation,
    pub(crate) trace: TracePresentation,
    pub(crate) temporal: TemporalPresentation,
    pub(crate) cck_label: String,
    pub(crate) instruction_label: String,
    pub(crate) halted: bool,
    pub(crate) stopped: bool,
}

impl Inspection {
    pub(crate) fn refresh_status(&mut self, session: &crate::host_inspection::HostInspection) {
        self.cck_label = format!("CCK: {}", session.cck);
        self.instruction_label = format!("Inst: {}", session.instructions_executed);
        self.halted = session.cpu.is_halted();
        self.stopped = session.cpu.is_stopped();
    }
}
