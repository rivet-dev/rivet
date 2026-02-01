//! Rust-based engine runner for Rivet.
//!
//! This library provides a pure Rust implementation of a Rivet runner that can be fully controlled
//! programmatically, allowing simulation of:
//! - Actor crashes with specific exit codes
//! - Protocol timing issues (delays, timeouts)
//! - Custom protocol events (sleep, alarms, etc.)
//! - Runner disconnection/reconnection scenarios
//!
//! # Example
//!
//! ```ignore
//! use rivet_engine_runner::{Runner, RunnerConfig, EchoActor};
//!
//! let config = RunnerConfig::builder()
//!     .endpoint("http://127.0.0.1:8080")
//!     .token("dev")
//!     .namespace("my-namespace")
//!     .runner_name("my-runner")
//!     .runner_key("unique-key")
//!     .build();
//!
//! let mut runner = Runner::new(config)?;
//! runner.register_actor("echo", |_| Box::new(EchoActor::new()));
//! runner.start().await?;
//! ```

mod actor;
mod behaviors;
mod protocol;
mod runner;

pub use actor::{ActorConfig, ActorEvent, ActorStartResult, ActorStopResult, KvRequest, TestActor};
pub use behaviors::{
	CountingCrashActor, CrashNTimesThenSucceedActor, CrashOnStartActor, CustomActor,
	CustomActorBuilder, DelayedStartActor, EchoActor, NotifyOnStartActor, SleepImmediatelyActor,
	StopImmediatelyActor, TimeoutActor, VerifyInputActor,
};
pub use protocol::PROTOCOL_VERSION;
pub use runner::{
	ActorLifecycleEvent, Runner, RunnerBuilder, RunnerBuilderLegacy, RunnerConfig,
	RunnerConfigBuilder,
};

// Re-export commonly used types from the protocol
pub use rivet_runner_protocol::mk2 as protocol_types;
