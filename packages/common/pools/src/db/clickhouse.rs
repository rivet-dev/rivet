use rivet_config::Config;
use std::time::Duration;

use crate::Error;

pub type ClickHousePool = clickhouse::Client;

#[tracing::instrument(skip(config))]
pub fn setup(config: Config) -> Result<Option<ClickHousePool>, Error> {
	if let Some(clickhouse) = &config.server().map_err(Error::Global)?.clickhouse {
		tracing::debug!("clickhouse connecting");

		// Build HTTP connector
		let mut http_connector = hyper::client::connect::HttpConnector::new();
		http_connector.enforce_http(false);
		http_connector.set_keepalive(Some(Duration::from_secs(15)));

		// Build TLS connector
		let tls_connector = tokio_native_tls::native_tls::TlsConnector::builder()
			// HACK(RVT-4649): We don't have ClickHouse certs on the edge server, this is temporary
			// until we no longer need it on the edge
			.danger_accept_invalid_certs(true)
			.build()
			.unwrap();

		// Build HTTPs connector
		let https_connector =
			hyper_tls::HttpsConnector::from((http_connector, tls_connector.into()));

		let http_client = hyper::Client::builder()
			.pool_idle_timeout(Duration::from_secs(2))
			.build(https_connector);

		// Build ClickHouse client
		let mut client = clickhouse::Client::with_http_client(http_client)
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
