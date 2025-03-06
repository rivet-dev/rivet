use std::{
	sync::{
		atomic::{AtomicBool, Ordering},
		Once,
	},
	time::Duration,
};

use tokio::process::Command;
use tracing_subscriber::prelude::*;

pub async fn start_nats() {
	Command::new("docker")
		.arg("rm")
		.arg("test-nats")
		.arg("--force")
		.status()
		.await
		.unwrap();

	let status = Command::new("docker")
		.arg("run")
		.arg("--rm")
		.arg("-p")
		.arg("4222:4222")
		.arg("--name")
		.arg("test-nats")
		.arg("nats:latest")
		.status()
		.await
		.unwrap();

	assert!(status.success());
}

pub async fn start_redis() {
	Command::new("docker")
		.arg("rm")
		.arg("test-redis")
		.arg("--force")
		.status()
		.await
		.unwrap();

	let status = Command::new("docker")
		.arg("run")
		.arg("--rm")
		.arg("-p")
		.arg("6379:6379")
		.arg("--name")
		.arg("test-redis")
		.arg("redis:latest")
		.status()
		.await
		.unwrap();

	assert!(status.success());
}

pub async fn start_fdb() {
	Command::new("docker")
		.arg("rm")
		.arg("test-fdb")
		.arg("--force")
		.status()
		.await
		.unwrap();

	let status = Command::new("docker")
		.arg("run")
		.arg("--rm")
		.arg("--platform=linux/amd64")
		.arg("-p")
		.arg("4500:4500")
		.arg("--name")
		.arg("test-fdb")
		.arg("-e")
		.arg("FDB_CLUSTER_FILE_CONTENTS=fdb:fdb@127.0.0.1:4500")
		// See docs-internal/infrastructure/fdb/AVX.md
		.arg("foundationdb/foundationdb:7.1.60")
		.status()
		.await
		.unwrap();

	assert!(status.success());
}

pub async fn create_fdb_db() {
	loop {
		// Create db
		let status = Command::new("docker")
			.arg("exec")
			.arg("test-fdb")
			.arg("fdbcli")
			.arg("--exec")
			.arg(r#"configure new single ssd"#)
			.status()
			.await
			.unwrap();

		if status.success() {
			break;
		} else {
			tracing::error!("failed to create fdb database");
		}

		tokio::time::sleep(Duration::from_secs(1)).await;
	}
}

static SETUP_DEPENDENCIES: AtomicBool = AtomicBool::new(false);

pub async fn setup_dependencies(fdb: bool) {
	if !SETUP_DEPENDENCIES.swap(true, Ordering::SeqCst) {
		tokio::spawn(start_nats());
		tokio::spawn(start_redis());

		if fdb {
			tokio::spawn(start_fdb());
			create_fdb_db().await;
		} else {
			tokio::time::sleep(std::time::Duration::from_secs(1)).await;
		}
	}
}

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
