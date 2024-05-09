use std::time::{Duration, Instant};

use rivet_metrics::{prometheus::*, REGISTRY};
use tokio::sync::OnceCell;

lazy_static::lazy_static! {
	pub static ref CHIRP_UPTIME: Gauge =
		register_gauge_with_registry!("chirp_uptime", "Uptime of the service.", *REGISTRY).unwrap();
	pub static ref CHIRP_CLIENT_ACTIVE: IntGauge =
		register_int_gauge_with_registry!("chirp_client_active", "Total number of active Chirp clients.", *REGISTRY).unwrap();
}

static UPDATE_UPTIME: OnceCell<()> = OnceCell::const_new();

#[tracing::instrument]
pub async fn start_update_uptime() {
	UPDATE_UPTIME
		.get_or_init(|| async {
			let spawn_res = tokio::task::Builder::new()
				.name("chirp_client::update_uptime")
				.spawn(update_uptime_loop());
			if let Err(err) = spawn_res {
				tracing::error!(?err, "failed to spawn update_uptime task");
			}
		})
		.await;
}

#[tracing::instrument]
async fn update_uptime_loop() {
	let start = Instant::now();

	let ctrl_c = tokio::signal::ctrl_c();
	tokio::pin!(ctrl_c);

	let mut interval = tokio::time::interval(Duration::from_secs(1));
	loop {
		tokio::select! {
			_ = &mut ctrl_c => {
				tracing::info!("stopping loop");
				break;
			}
			_ = interval.tick() => {
				let duration = Instant::now().duration_since(start);
				CHIRP_UPTIME.set(duration.as_secs_f64());
			}
		}
	}
}
