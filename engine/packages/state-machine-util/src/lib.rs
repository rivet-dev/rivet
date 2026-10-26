//! Shared transition helpers and executor dispatch gates for state machines.
pub mod backoff;

pub mod dispatch_guard;

pub mod io_call;

pub mod lease_watch;

pub use backoff::Backoff;

pub use dispatch_guard::DispatchGuard;

pub use rivet_state_machine::Effects;

pub use io_call::{IoCall, IoTimer};

pub use lease_watch::{LeaseChange, LeaseWatch};
