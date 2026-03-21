//! Rust-based envoy client for Rivet.
//!
//! This library provides a pure Rust implementation of a Rivet envoy that can be fully controlled
//! programmatically, allowing simulation of:
//! - Actor crashes with specific exit codes
//! - Protocol timing issues (delays, timeouts)
//! - Custom protocol events (sleep, alarms, etc.)
//! - Envoy disconnection/reconnection scenarios
//!
//! # Example
//!
//! ```ignore
//! use rivet_envoy_client::{Envoy, EnvoyConfig, EchoActor};
//!
//! let config = EnvoyConfig::builder()
//!     .endpoint("http://127.0.0.1:8080")
//!     .token("dev")
//!     .namespace("my-namespace")
//!     .pool_name("my-pool")
//!     .build();
//!
//! let mut envoy = Envoy::new(config)?;
//! envoy.register_actor("echo", |_| Box::new(EchoActor::new()));
//! envoy.start().await?;
//! ```

mod actor;
mod behaviors;
mod envoy;
mod utils;

pub use actor::{ActorConfig, ActorEvent, ActorStartResult, ActorStopResult, KvRequest, TestActor};
pub use behaviors::{
	CountingCrashActor, CrashNTimesThenSucceedActor, CrashOnStartActor, CustomActor,
	CustomActorBuilder, DelayedStartActor, EchoActor, NotifyOnStartActor, SleepImmediatelyActor,
	StopImmediatelyActor, TimeoutActor, VerifyInputActor,
};
pub use envoy::{ActorLifecycleEvent, Envoy, EnvoyBuilder, EnvoyConfig, EnvoyConfigBuilder};

// Re-export commonly used types from the protocol
pub use rivet_envoy_protocol as protocol;
