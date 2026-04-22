use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use anyhow::Result;
use gas::prelude::*;
use rivet_types::runner_configs::{RunnerConfig, RunnerConfigKind};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct MockMetadataState {
	expose_protocol_version: AtomicBool,
}

async fn run_mock_metadata_server(
	listener: tokio::net::TcpListener,
	state: Arc<MockMetadataState>,
) {
	loop {
		let Ok((mut socket, _)) = listener.accept().await else {
			return;
		};
		let state = state.clone();
		tokio::spawn(async move {
			let mut buf = [0; 1024];
			let _ = socket.read(&mut buf).await;

			let body = if state.expose_protocol_version.load(Ordering::SeqCst) {
				format!(
					r#"{{"runtime":"rivetkit","version":"1","envoyProtocolVersion":{}}}"#,
					rivet_envoy_protocol::PROTOCOL_VERSION
				)
			} else {
				r#"{"runtime":"rivetkit","version":"1"}"#.to_string()
			};

			let response = format!(
				"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
				body.len(),
				body
			);
			let _ = socket.write_all(response.as_bytes()).await;
			let _ = socket.shutdown().await;
		});
	}
}

#[tokio::test]
async fn refresh_metadata_purges_runner_config_protocol_cache() -> Result<()> {
	let test_deps = rivet_test_deps::TestDeps::new().await?;
	let cache = rivet_cache::CacheInner::from_env(&test_deps.config, test_deps.pools.clone())?;
	let ctx = StandaloneCtx::new(
		db::DatabaseKv::new(test_deps.config.clone(), test_deps.pools.clone()).await?,
		test_deps.config.clone(),
		test_deps.pools.clone(),
		cache,
		"runner_config_refresh_metadata_test",
		Id::new_v1(test_deps.config.dc_label()),
		Id::new_v1(test_deps.config.dc_label()),
	)?;

	let state = Arc::new(MockMetadataState {
		expose_protocol_version: AtomicBool::new(false),
	});
	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
	let mock_addr = listener.local_addr()?;
	let server_handle = tokio::spawn(run_mock_metadata_server(listener, state.clone()));

	let namespace_id = Id::new_v1(test_deps.config.dc_label());
	let runner_name = "metadata-refresh-cache-test".to_string();
	let headers = std::collections::HashMap::new();
	let url = format!("http://{mock_addr}");

	let runner_config = RunnerConfig {
		kind: RunnerConfigKind::Serverless {
			url: url.clone(),
			headers: headers.clone(),
			request_lifespan: 30,
			max_concurrent_actors: 10,
			drain_grace_period: 5,
			slots_per_runner: 1,
			min_runners: 0,
			max_runners: 0,
			runners_margin: 0,
			metadata_poll_interval: None,
		},
		metadata: None,
		drain_on_version_upgrade: true,
	};
	ctx.udb()?
		.run(|tx| {
			let runner_name = runner_name.clone();
			let runner_config = runner_config.clone();
			async move {
				let tx = tx.with_subspace(namespace::keys::subspace());
				tx.write(
					&pegboard::keys::runner_config::DataKey::new(namespace_id, runner_name),
					runner_config,
				)?;
				Ok(())
			}
		})
		.await?;

	let cached_before_refresh = ctx
		.op(pegboard::ops::runner_config::get::Input {
			runners: vec![(namespace_id, runner_name.clone())],
			bypass_cache: false,
		})
		.await?;
	assert_eq!(cached_before_refresh[0].protocol_version, None);

	state.expose_protocol_version.store(true, Ordering::SeqCst);

	let refresh_result = ctx
		.op(pegboard::ops::runner_config::refresh_metadata::Input {
			namespace_id,
			runner_name: runner_name.clone(),
			url,
			headers,
		})
		.await?;
	assert!(
		refresh_result.is_ok(),
		"metadata refresh failed: {refresh_result:?}"
	);

	tokio::time::timeout(Duration::from_millis(100), async {
		let cached_after_refresh = ctx
			.op(pegboard::ops::runner_config::get::Input {
				runners: vec![(namespace_id, runner_name.clone())],
				bypass_cache: false,
			})
			.await?;
		assert_eq!(
			cached_after_refresh[0].protocol_version,
			Some(rivet_envoy_protocol::PROTOCOL_VERSION)
		);

		Ok::<_, anyhow::Error>(())
	})
	.await
	.expect("metadata refresh should invalidate the old 5s runner-config cache")?;

	server_handle.abort();

	Ok(())
}
