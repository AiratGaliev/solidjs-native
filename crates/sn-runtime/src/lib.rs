//! Reusable QuickJS-backed JavaScript runtime for SolidJS Native.
//!
//! This package builds a Rust library and a separate CLI smoke binary. The
//! library exposes [`EngineSession`] and its Rust-only command/configuration
//! types; the binary consumes that public facade instead of owning runtime
//! implementation code.
//!
//! Each [`EngineSession`] creates one QuickJS Runtime and Context on a
//! dedicated engine thread for one Surface lifecycle. QuickJS values and
//! handles remain inside private implementation modules and never cross the
//! worker boundary. Platform callers interact through bounded commands and
//! plain Rust values.
//!
//! Native hosts should create configurations with
//! [`EngineSessionConfig::for_surface`] so that each live Surface lifecycle
//! has its actual non-zero identity and generation. [`EngineSessionConfig::default`]
//! uses the reserved `1/1` identity for headless smoke tests and examples.
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
