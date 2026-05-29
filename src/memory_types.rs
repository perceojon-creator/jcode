pub use jcode_memory_types::*;

// Runtime-oriented activity types (MemoryActivity, PipelineState, Step*, MemoryState,
// MemoryEvent*, InjectedMemoryItem + impls using std::time::Instant) lifted here via
// re-export from src/memory/activity.rs (Ola 2 Agent 3). This is the thin compatibility
// shim so that TUI (info_widget* etc) + other call sites using `crate::memory_types::*`
// continue to work without edits. jcode-memory-types no longer exports these (purity).
pub use crate::memory::{
    InjectedMemoryItem, MemoryActivity, MemoryEvent, MemoryEventKind, MemoryState,
    PipelineState, StepResult, StepStatus,
};
