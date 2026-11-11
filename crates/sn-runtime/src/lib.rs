//! Reusable QuickJS-backed JavaScript runtime for SolidJS Native.
//!
//! Each EngineSession creates and owns its QuickJS Runtime and Context on a
//! dedicated engine thread. Host/platform callers interact only through
//! bounded commands and plain Rust values; QuickJS handles never cross threads.

pub mod clock;
mod host;
mod scheduler;
mod session;

pub use clock::{ManualClock, MonotonicClock, RuntimeClock};
pub use session::{
    DiagnosticKind, EngineSession, EngineSessionConfig, EngineSessionHandle,
    MAX_EXTERNAL_COMPLETION_BYTES, RuntimeDiagnostic, RuntimeSnapshot, SessionError, TaskTicket,
};
pub use sn_core::{SessionGeneration, SurfaceId};
