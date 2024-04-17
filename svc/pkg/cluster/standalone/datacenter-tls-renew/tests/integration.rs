use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

use ::cluster_datacenter_tls_renew::run_from_env;

#[tokio::test(flavor = "multi_thread")]
async fn basic() {
	if !util::feature::server_provision() {
		return;
	}

	tracing_subscriber::fmt()
		.json()
		.with_max_level(tracing::Level::INFO)
		.with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
		.init();

	let ctx = TestCtx::from_env("cluster-gc-test").await.unwrap();
	let pools = rivet_pools::from_env("cluster-gc-test").await.unwrap();

	// TODO:
}
