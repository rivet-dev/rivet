//! OTLP export of actor spans.
//!
//! Add [`layer`] to the application's `tracing` subscriber. It exports only when
//! `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` is set, and returns `None` otherwise.
//!
//! ```ignore
//! use tracing_subscriber::prelude::*;
//!
//! tracing_subscriber::registry()
//! 	.with(rivetkit::telemetry::layer()?)
//! 	.init();
//! ```
//!
//! [`Registry::start`](crate::Registry::start) flushes pending spans before it
//! returns. Call [`shutdown`] after [`Registry::serve`](crate::Registry::serve)
//! returns when driving the registry directly.

pub use rivetkit_core::telemetry::export::{layer, shutdown_best_effort as shutdown};
