// TODO:

// #[cfg(test)]
// mod tests {
// 	use std::{
// 		collections::HashMap,
// 		os::fd::AsRawFd,
// 		path::{Path, PathBuf},
// 		result::Result::{Err, Ok},
// 		sync::Arc,
// 		thread::JoinHandle,
// 		time::Duration,
// 	};

// 	use anyhow::*;
// 	use deno_core::JsRuntime;
// 	use deno_runtime::worker::MainWorkerTerminateHandle;
// 	use foundationdb as fdb;
// 	use futures_util::{stream::SplitStream, SinkExt, StreamExt};
// 	use tokio::{
// 		fs,
// 		net::TcpStream,
// 		sync::{mpsc, RwLock},
// 	};
// 	use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
// 	use tracing_subscriber::prelude::*;
// 	use uuid::Uuid;

// 	use super::run_inner;
// 	use crate::config::*;
// 	use crate::utils::{self, var};

// 	const THREAD_STATUS_POLL_INTERVAL: Duration = Duration::from_millis(500);

// 	#[tokio::test]
// 	async fn test_isolate() -> Result<()> {
// 		tracing_subscriber::registry()
// 			.with(
// 				tracing_logfmt::builder()
// 					.with_ansi_color(true)
// 					.layer()
// 					.with_filter(tracing_subscriber::filter::LevelFilter::INFO),
// 			)
// 			.init();

// 		// Start FDB network thread
// 		let _network = unsafe { fdb::boot() };
// 		tokio::spawn(utils::fdb_health_check());

// 		// For receiving the terminate handle
// 		let (terminate_tx, _terminate_rx) =
// 			tokio::sync::mpsc::channel::<MainWorkerTerminateHandle>(1);

// 		let test_dir = Path::new("/tmp/pegboard-isolate-v8-runner-test/");
// 		let actors_path = test_dir.join("actors");
// 		let actor_id = Uuid::nil();

// 		unsafe {
// 			std::env::set_var(
// 				"FDB_CLUSTER_PATH",
// 				&test_dir.join("fdb.cluster").display().to_string(),
// 			)
// 		};

// 		let config = Config {
// 			resources: Resources {
// 				memory: 26843545600,
// 				memory_max: 26843545600,
// 			},
// 			ports: Default::default(),
// 			env: Default::default(),
// 			stakeholder: Stakeholder::DynamicServer {
// 				server_id: String::new(),
// 			},
// 			vector_socket_addr: Default::default(),
// 		};

// 		run_inner(
// 			actors_path.join(actor_id.to_string()).to_path_buf(),
// 			actor_id,
// 			terminate_tx,
// 			None,
// 			config,
// 		)
// 		.await?;

// 		Ok(())
// 	}
// }
