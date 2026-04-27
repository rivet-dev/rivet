use std::time::Duration;

use anyhow::Result;
use gas::prelude::*;
use pegboard::ops::serverless_metadata::fetch::ServerlessMetadataError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn build_ctx(test_name: &str) -> Result<StandaloneCtx> {
	let test_deps = rivet_test_deps::TestDeps::new().await?;
	let cache = rivet_cache::CacheInner::from_env(&test_deps.config, test_deps.pools.clone())?;

	Ok(StandaloneCtx::new(
		db::DatabaseKv::new(test_deps.config.clone(), test_deps.pools.clone()).await?,
		test_deps.config.clone(),
		test_deps.pools.clone(),
		cache,
		test_name,
		Id::new_v1(test_deps.config.dc_label()),
		Id::new_v1(test_deps.config.dc_label()),
	)?)
}

async fn spawn_metadata_server(body: String) -> Result<(String, tokio::task::JoinHandle<()>)> {
	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
	let addr = listener.local_addr()?;

	let handle = tokio::spawn(async move {
		let Ok((mut socket, _)) = listener.accept().await else {
			return;
		};

		let mut buf = [0; 1024];
		let _ = socket.read(&mut buf).await;

		let response = format!(
			"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
			body.len(),
			body
		);
		let _ = socket.write_all(response.as_bytes()).await;
		let _ = socket.shutdown().await;
	});

	Ok((format!("http://{addr}"), handle))
}

#[tokio::test]
async fn invalid_json_returns_invalid_response_json() -> Result<()> {
	let ctx = build_ctx("serverless_metadata_fetch_invalid_json").await?;
	let body = r#"{"runtime":"rivetkit","version":"1""#.to_string();
	let (url, handle) = spawn_metadata_server(body.clone()).await?;

	let result = ctx
		.op(pegboard::ops::serverless_metadata::fetch::Input {
			url,
			headers: Default::default(),
		})
		.await?;

	handle.abort();

	let Err(ServerlessMetadataError::InvalidResponseJson {
		body: actual_body,
		parse_error,
	}) = result
	else {
		panic!("expected invalid_response_json, got {result:?}");
	};

	assert_eq!(actual_body, body);
	assert!(!parse_error.is_empty(), "parse_error should not be empty");

	Ok(())
}

#[tokio::test]
async fn invalid_schema_returns_invalid_response_schema_for_runtime_and_version() -> Result<()> {
	let cases = [
		(
			"serverless_metadata_fetch_invalid_runtime",
			r#"{"runtime":"sandboxed-node","version":"1"}"#,
			"sandboxed-node",
			"1",
		),
		(
			"serverless_metadata_fetch_empty_version",
			r#"{"runtime":"rivetkit","version":"   "}"#,
			"rivetkit",
			"   ",
		),
	];

	for (test_name, body, expected_runtime, expected_version) in cases {
		let ctx = build_ctx(test_name).await?;
		let (url, handle) = spawn_metadata_server(body.to_string()).await?;

		let result = ctx
			.op(pegboard::ops::serverless_metadata::fetch::Input {
				url,
				headers: Default::default(),
			})
			.await?;

		handle.abort();

		let Err(ServerlessMetadataError::InvalidResponseSchema { runtime, version }) = result
		else {
			panic!("expected invalid_response_schema, got {result:?}");
		};

		assert_eq!(runtime, expected_runtime);
		assert_eq!(version, expected_version);
	}

	Ok(())
}

#[tokio::test]
async fn invalid_envoy_protocol_version_returns_explicit_error() -> Result<()> {
	let ctx = build_ctx("serverless_metadata_fetch_invalid_envoy_protocol_version").await?;
	let invalid_version = rivet_envoy_protocol::PROTOCOL_VERSION + 1;
	let body = format!(
		r#"{{"runtime":"rivetkit","version":"1","envoyProtocolVersion":{invalid_version}}}"#
	);
	let (url, handle) = spawn_metadata_server(body).await?;

	let result = tokio::time::timeout(
		Duration::from_secs(10),
		ctx.op(pegboard::ops::serverless_metadata::fetch::Input {
			url,
			headers: Default::default(),
		}),
	)
	.await??;

	handle.abort();

	let Err(ServerlessMetadataError::InvalidEnvoyProtocolVersion { version }) = result else {
		panic!("expected invalid_envoy_protocol_version, got {result:?}");
	};

	assert_eq!(version, invalid_version);

	Ok(())
}
