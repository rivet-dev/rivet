use chirp_worker::prelude::*;
use chirp_worker::ManagerError;

mod workers;
use workers::*;

fn main() -> Result<(), ManagerError> {
	// Start runtime
	rivet_runtime::run(async move {
		{
			let clickhouse_url = std::env::var("CLICKHOUSE_URL").unwrap();
			let res = reqwest::get(clickhouse_url)
			.await;

			match res {
				Ok(r) => {
					let res = r.text().await;
					tracing::info!(?res, "---------------------");
				}
				Err(e) => {
					tracing::info!(%e, "------------------");
				}
			}
		}

		worker_group![event_create].await?;

		Ok(())
	})?
}
