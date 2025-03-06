use std::{
	sync::{
		atomic::{AtomicBool, Ordering},
		Once,
	},
	time::Duration,
};

use tokio::process::Command;
use tracing_subscriber::prelude::*;

static SETUP_TRACING: Once = Once::new();
pub fn setup_tracing() {
	SETUP_TRACING.call_once(|| {
		tracing_subscriber::registry()
			.with(
				tracing_logfmt::builder()
					.layer()
					.with_filter(tracing_subscriber::filter::LevelFilter::INFO),
			)
			.init();
	});
}
