//! Rust-based test runner for deep actor lifecycle testing.
//!
//! This module provides a pure Rust implementation of a runner that can be fully controlled
//! from tests, allowing simulation of:
//! - Actor crashes with specific exit codes
//! - Protocol timing issues (delays, timeouts)
//! - Custom protocol events (sleep, alarms, etc.)
//! - Runner disconnection/reconnection scenarios

mod actor;
mod behaviors;
mod protocol;
mod runner;

pub use actor::{ActorConfig, ActorStartResult, ActorStopResult, TestActor};
pub use behaviors::{
	CrashNTimesThenSucceedActor, CrashOnStartActor, CustomActor, CustomActorBuilder,
	DelayedStartActor, EchoActor, NotifyOnStartActor, SleepImmediatelyActor, StopImmediatelyActor,
	TimeoutActor, VerifyInputActor,
};
pub use runner::{ActorLifecycleEvent, TestRunner, TestRunnerBuilder};
