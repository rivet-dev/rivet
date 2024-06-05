use std::sync::Arc;

use anyhow::*;
use serde::{Deserialize, Serialize};
use tokio::time::Duration;
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};
use uuid::Uuid;
use wf::*;

pub fn setup() {
	tracing_subscriber::fmt()
		.pretty()
		.with_env_filter(EnvFilter::new("trace"))
		.with_span_events(FmtSpan::CLOSE)
		.init();
}

// MARK: Activity
#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct MyActivityInput {
	pub x: i64,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct MyActivityOutput {
	pub y: i64,
}

#[macros::activity(MyActivity)]
pub fn my_activity(my_ctx: &mut ActivityCtx, input: &MyActivityInput) -> Result<MyActivityOutput> {
	util::inject_fault()?;
	Ok(MyActivityOutput { y: input.x * 2 })
}
