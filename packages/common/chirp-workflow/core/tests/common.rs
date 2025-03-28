use std::sync::Once;

use tracing_subscriber::prelude::*;

static SETUP_TRACING: Once = Once::new();
pub fn setup_tracing() {
	SETUP_TRACING.call_once(|| {
		tracing_subscriber::registry()
			.with(
				tracing_logfmt::builder()
					.layer()
					.with_filter(tracing_subscriber::filter::LevelFilter::DEBUG),
			)
			.init();
	});
}
