use std::{env, time::Duration};

use hyper::client::connect::HttpConnector;
use hyper_tls::HttpsConnector;

use crate::error::Error;

const TCP_KEEPALIVE: Duration = Duration::from_secs(60);
const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(2);

pub fn client() -> Result<clickhouse::Client, Error> {
	let mut http = HttpConnector::new();
	http.enforce_http(false);
	http.set_keepalive(Some(TCP_KEEPALIVE));
	let https = HttpsConnector::new_with_connector(http);
	let client = hyper::Client::builder()
		.pool_idle_timeout(POOL_IDLE_TIMEOUT)
		.build(https);

	let clickhouse_url = env::var("CLICKHOUSE_URL").map_err(Error::Env)?;
	let client = clickhouse::Client::with_http_client(client).with_url(clickhouse_url);

	Ok(client)
}
