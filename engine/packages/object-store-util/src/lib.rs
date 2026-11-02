//! Shared object-store protocols, embedded in their caller's ephemeral state machine.
//!
//! These helpers perform no I/O and create no runner, task, lease, or storage client.
//! Executors still use injected adapters and check current authority before dispatch.

pub mod conditional_write;
