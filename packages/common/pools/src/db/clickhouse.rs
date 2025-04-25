use rivet_config::Config;
use std::time::Duration;

use crate::Error;

pub type ClickHousePool = clickhouse::Client;

#[tracing::instrument(skip(config))]
pub fn setup(config: Config) -> Result<Option<ClickHousePool>, Error> {
	if let Some(clickhouse) = &config.server().map_err(Error::Global)?.clickhouse {
		tracing::debug!("clickhouse connecting");

		// Build HTTP client
		let mut http_connector = hyper::client::connect::HttpConnector::new();
		http_connector.enforce_http(false);
		http_connector.set_keepalive(Some(Duration::from_secs(15)));
		let https_connector = hyper_tls::HttpsConnector::new_with_connector(http_connector);
		let http_client = hyper::Client::builder()
			.pool_idle_timeout(Duration::from_secs(2))
			.build(https_connector);

		// Build ClickHouse client
		let mut client = clickhouse::Client::with_http_client(http_client)
			.with_option("max_execution_time", "10")
			.with_option("max_memory_usage", "100000000") // Limit memory usage to 100MB
			.with_option("max_rows_to_read", "10000") // Limit to 10k rows for log queries
			.with_option("max_bytes_to_read", "10000000") // Limit to 10MB of data
			.with_option("regexp_max_matches_per_row", "10") // Prevent against DDoS
			.with_option("timeout_before_checking_execution_speed", "1") // Check speed after 1 second
			.with_url(clickhouse.http_url.to_string())
			.with_user(&clickhouse.username);
		if let Some(password) = &clickhouse.password {
			client = client.with_password(password.read());
		}

		Ok(Some(client))
	} else {
		Ok(None)
	}
}
