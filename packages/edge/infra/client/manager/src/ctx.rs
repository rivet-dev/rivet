use std::{
	collections::HashMap,
	net::SocketAddr,
	path::PathBuf,
	result::Result::{Err, Ok},
	sync::Arc,
	time::Duration,
};

use anyhow::*;
use futures_util::{
	stream::{SplitSink, SplitStream},
	SinkExt, StreamExt,
};
use indoc::indoc;
use nix::{sys::signal::Signal, unistd::Pid};
use pegboard::{protocol, system_info::SystemInfo};
use pegboard_config::{
	isolate_runner::Config as IsolateRunnerConfig, runner_protocol, Client, Config,
};
use sqlite_util::SqlitePoolExt;
use sqlx::{pool::PoolConnection, Acquire, Sqlite, SqlitePool};
use tokio::{
	fs,
	net::{TcpListener, TcpStream},
	sync::{Mutex, RwLock},
};
use tokio_tungstenite::{
	tungstenite::protocol::{
		frame::{coding::CloseCode, CloseFrame},
		Message,
	},
	MaybeTlsStream, WebSocketStream,
};
use url::Url;
use uuid::Uuid;

use crate::{
	actor::Actor, event_sender::EventSender, metrics, pull_addr_handler::PullAddrHandler, runner,
	utils,
};

const PING_INTERVAL: Duration = Duration::from_secs(1);

#[derive(thiserror::Error, Debug)]
pub enum RuntimeError {
	#[error("ws connection to {url} failed: {source}")]
	ConnectionFailed {
		url: Url,
		source: tokio_tungstenite::tungstenite::Error,
	},
	#[error("ws failed: {0}")]
	SocketFailed(tokio_tungstenite::tungstenite::Error),
	#[error("runner socket failed: {0}")]
	RunnerSocketListenFailed(std::io::Error),
	#[error("socket closed: {0}, {1}")]
	SocketClosed(CloseCode, String),
	#[error("stream closed")]
	StreamClosed,
}

#[derive(sqlx::FromRow)]
struct ActorRow {
	actor_id: Uuid,
	config: Vec<u8>,
	pid: Option<i32>,
	stop_ts: Option<i64>,
}

pub struct Ctx {
	config: Config,
	system: SystemInfo,

	// This requires a RwLock because of the reset functionality which reinitialized the entire database. It
	// should never be written to besides that.
	pool: RwLock<SqlitePool>,
	tx: Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
	event_sender: EventSender,
	pub(crate) pull_addr_handler: PullAddrHandler,

	pub(crate) actors: RwLock<HashMap<Uuid, Arc<Actor>>>,
	isolate_runner: RwLock<Option<runner::Handle>>,
}

impl Ctx {
	pub fn new(
		config: Config,
		system: SystemInfo,
		pool: SqlitePool,
		tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
	) -> Arc<Self> {
		Arc::new(Ctx {
			config,
			system,

			pool: RwLock::new(pool),
			tx: Mutex::new(tx),
			event_sender: EventSender::new(),
			pull_addr_handler: PullAddrHandler::new(),

			actors: RwLock::new(HashMap::new()),
			isolate_runner: RwLock::new(None),
		})
	}

	pub async fn sql(&self) -> std::result::Result<PoolConnection<Sqlite>, sqlx::Error> {
		self.pool
			.try_read()
			.expect("should not be locked")
			.conn()
			.await
	}

	pub(crate) async fn send_packet(&self, packet: protocol::ToServer) -> Result<()> {
		let buf = packet.serialize()?;
		self.tx.lock().await.send(Message::Binary(buf)).await?;

		metrics::PACKET_SEND_TOTAL.with_label_values(&[]).inc();

		Ok(())
	}

	async fn write_event(&self, event: &protocol::Event) -> Result<i64> {
		// Write event to db
		let event_json = serde_json::to_vec(event)?;

		// Fetch next idx
		let index = utils::sql::query(|| async {
			let mut conn = self.sql().await?;
			let mut tx = conn.begin().await?;

			let (index,) = sqlx::query_as::<_, (i64,)>(indoc!(
				"
				UPDATE state
				SET last_event_idx = last_event_idx + 1
				RETURNING last_event_idx
				",
			))
			.fetch_one(&mut *tx)
			.await?;

			sqlx::query(indoc!(
				"
				INSERT INTO events (
					idx,
					payload,
					create_ts
				)
				SELECT ?1, ?2, ?3
				",
			))
			.bind(index)
			.bind(&event_json)
			.bind(utils::now())
			.execute(&mut *tx)
			.await?;

			tx.commit().await?;

			Ok(index)
		})
		.await?;

		Ok(index)
	}

	pub async fn event(&self, event: protocol::Event) -> Result<()> {
		let index = self.write_event(&event).await?;

		self.event_sender.send(self, event, index).await
	}

	pub async fn run(
		self: &Arc<Self>,
		rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
	) -> Result<()> {
		// Rebuild isolate runner from db before starting runner socket
		self.rebuild_isolate_runner().await?;

		// Start runner socket
		let self2 = self.clone();
		let runner_socket: tokio::task::JoinHandle<Result<()>> = tokio::spawn(async move {
			tracing::info!(port=%self2.config().runner.port(), "listening for runner sockets");

			let listener = TcpListener::bind(("0.0.0.0", self2.config().runner.port()))
				.await
				.map_err(RuntimeError::RunnerSocketListenFailed)?;

			loop {
				match listener.accept().await {
					Ok((stream, _)) => {
						let mut ws_stream = tokio_tungstenite::accept_async(stream).await?;

						tracing::info!("received new socket");

						if let Some(runner) = &*self2.isolate_runner.read().await {
							runner.attach_socket(ws_stream).await?;
						} else {
							tracing::error!("killing unknown runner");

							metrics::UNKNOWN_ISOLATE_RUNNER.with_label_values(&[]).inc();

							ws_stream
								.send(Message::Binary(serde_json::to_vec(
									&runner_protocol::ToRunner::Terminate,
								)?))
								.await?;

							let close_frame = CloseFrame {
								code: CloseCode::Error,
								reason: "unknown runner".into(),
							};
							ws_stream.send(Message::Close(Some(close_frame))).await?;
						}
					}
					Err(err) => tracing::error!(?err, "failed to connect websocket"),
				}
			}
		});

		// Start ping thread
		let self2 = self.clone();
		let ping_thread: tokio::task::JoinHandle<Result<()>> = tokio::spawn(async move {
			loop {
				tokio::time::sleep(PING_INTERVAL).await;
				self2
					.tx
					.lock()
					.await
					.send(Message::Ping(Vec::new()))
					.await?;
			}
		});

		// Send init packet
		{
			let (last_command_idx, last_workflow_id) = utils::sql::query(|| async {
				sqlx::query_as::<_, (i64, Option<Uuid>)>(indoc!(
					"
					SELECT last_command_idx, last_workflow_id FROM state
					",
				))
				.fetch_one(&mut *self.sql().await?)
				.await
			})
			.await?;

			self.send_packet(protocol::ToServer::Init {
				last_command_idx,
				last_workflow_id,
				config: self.config.build_client_config(),
				system: self.system.clone(),
			})
			.await?;
		}

		tokio::try_join!(
			async { runner_socket.await? },
			async { ping_thread.await? },
			self.receive_messages(rx),
		)?;

		Ok(())
	}

	async fn receive_messages(
		self: &Arc<Self>,
		mut rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
	) -> Result<()> {
		while let Some(msg) = rx.next().await {
			match msg.map_err(RuntimeError::SocketFailed)? {
				Message::Binary(buf) => {
					metrics::PACKET_RECV_TOTAL.with_label_values(&[]).inc();

					let packet = protocol::ToClient::deserialize(&buf)?;

					self.process_packet(packet).await?;
				}
				Message::Pong(_) => tracing::debug!("received pong"),
				Message::Close(Some(close_frame)) => {
					return Err(RuntimeError::SocketClosed(
						close_frame.code,
						close_frame.reason.to_string(),
					)
					.into())
				}
				Message::Close(None) => {
					return Err(RuntimeError::SocketClosed(
						CloseCode::Abnormal,
						"no close frame".to_string(),
					)
					.into())
				}
				msg => {
					tracing::warn!(?msg, "unexpected message");
				}
			}
		}

		Err(RuntimeError::StreamClosed.into())
	}

	async fn process_packet(self: &Arc<Self>, packet: protocol::ToClient) -> Result<()> {
		tracing::debug!(?packet, "received packet");

		match packet {
			protocol::ToClient::Init {
				last_event_idx,
				workflow_id,
			} => {
				// Reset all state if workflow id changed
				self.reset(workflow_id).await?;

				// Send out all missed events
				self.rebroadcast(last_event_idx).await?;

				// Rebuild state only after the init packet is received and processed so that we don't emit
				// any new events before the missed events are rebroadcast
				self.rebuild(workflow_id).await?;
			}
			protocol::ToClient::Commands(commands) => {
				for command in commands {
					self.process_command(command).await?;
				}
			}
			protocol::ToClient::PrewarmImage {
				image_id,
				image_artifact_url_stub,
			} => {
				let self2 = self.clone();

				tokio::spawn(async move {
					utils::prewarm_image(&self2, image_id, &image_artifact_url_stub).await
				});
			}
		}

		Ok(())
	}

	async fn process_command(self: &Arc<Self>, command: protocol::CommandWrapper) -> Result<()> {
		match command.inner.deserialize()? {
			protocol::Command::StartActor { actor_id, config } => {
				// Insert actor
				let mut actors = self.actors.write().await;
				let actor = actors
					.entry(actor_id)
					.or_insert_with(|| Actor::new(actor_id, *config));

				// Spawn actor
				actor.start(&self).await?;
			}
			protocol::Command::SignalActor {
				actor_id,
				signal,
				persist_storage,
				ignore_future_state: _,
			} => {
				if let Some(actor) = self.actors.read().await.get(&actor_id) {
					actor
						.signal(&self, signal.try_into()?, persist_storage)
						.await?;
				} else {
					tracing::warn!(
						?actor_id,
						"received stop actor command for actor that doesn't exist (likely already stopped)"
					);
				}
			}
		}

		// Ack command
		tokio::try_join!(
			utils::sql::query(|| async {
				sqlx::query(indoc!(
					"
					UPDATE state
					SET last_command_idx = ?1
					",
				))
				.bind(command.index)
				.execute(&mut *self.sql().await?)
				.await
			}),
			utils::sql::query(|| async {
				sqlx::query(indoc!(
					"
					INSERT INTO commands (
						idx,
						payload,
						ack_ts
					)
					VALUES(?1, ?2, ?3)
					",
				))
				.bind(command.index)
				// `Raw` is encodable on its own but we need it to be written as a BLOB and not TEXT
				.bind(command.inner.get().as_bytes())
				.bind(utils::now())
				.execute(&mut *self.sql().await?)
				.await
			}),
		)?;

		Ok(())
	}
}

// MARK: Isolate runner
impl Ctx {
	pub(crate) async fn get_or_spawn_isolate_runner(self: &Arc<Self>) -> Result<runner::Handle> {
		let mut guard = self.isolate_runner.write().await;

		if let Some(runner) = &*guard {
			Ok(runner.clone())
		} else {
			tracing::info!("spawning new isolate runner");

			let working_path = self.isolate_runner_path();

			let config = IsolateRunnerConfig {
				actors_path: self.actors_path(),
				fdb_cluster_path: self.fdb_cluster_path(),
				runner_addr: SocketAddr::from(([127, 0, 0, 1], self.config().runner.port())),
				foundationdb: self.config.client.foundationdb.clone(),
			};

			// Delete existing exit code
			if let Err(err) = fs::remove_file(working_path.join("exit-code")).await {
				if err.kind() != std::io::ErrorKind::NotFound {
					return Err(err.into());
				}
			}

			// Write isolate runner config
			fs::write(
				working_path.join("config.json"),
				serde_json::to_vec(&config)?,
			)
			.await?;

			let runner = runner::Handle::spawn_orphaned(
				runner::Comms::socket(),
				&self.config().runner.isolate_runner_binary_path(),
				working_path,
				&[],
			)?;
			let pid = runner.pid();

			self.observe_isolate_runner(&runner);

			// Save runner pid
			utils::sql::query(|| async {
				sqlx::query(indoc!(
					"
					UPDATE state
					SET isolate_runner_pid = ?1
					",
				))
				.bind(pid.as_raw())
				.execute(&mut *self.sql().await?)
				.await
			})
			.await?;

			*guard = Some(runner.clone());

			Ok(runner)
		}
	}

	fn observe_isolate_runner(self: &Arc<Self>, runner: &runner::Handle) {
		tracing::info!(pid=?runner.pid(), "observing isolate runner");

		// Observe runner
		let self2 = self.clone();
		let runner2 = runner.clone();
		tokio::spawn(async move {
			let exit_code = match runner2.observe().await {
				Ok(exit_code) => exit_code,
				Err(err) => {
					// TODO: This should hard error the manager
					tracing::error!(%err, "failed to observe isolate runner");
					return;
				}
			};

			tracing::error!(pid=?runner2.pid(), ?exit_code, "isolate runner exited");

			// Update in-memory state
			let mut guard = self2.isolate_runner.write().await;
			*guard = None;

			// Update db state
			let res = utils::sql::query(|| async {
				sqlx::query(indoc!(
					"
					UPDATE state
					SET isolate_runner_pid = NULL
					",
				))
				.execute(&mut *self2.sql().await?)
				.await
			})
			.await;

			if let Err(err) = res {
				// TODO: This should hard error the manager
				tracing::error!(%err, "failed to write isolate runner");
			}
		});
	}
}

// MARK: State re-initialization
impl Ctx {
	/// Fetches isolate runner state from the db. Should be called before the manager's runner websocket opens.
	async fn rebuild_isolate_runner(self: &Arc<Self>) -> Result<()> {
		let (isolate_runner_pid,) = utils::sql::query(|| async {
			sqlx::query_as::<_, (Option<i32>,)>(indoc!(
				"
				SELECT isolate_runner_pid
				FROM state
				",
			))
			.fetch_one(&mut *self.sql().await?)
			.await
		})
		.await?;

		// Recreate isolate runner handle
		if let Some(isolate_runner_pid) = isolate_runner_pid {
			let mut guard = self.isolate_runner.write().await;

			tracing::info!(?isolate_runner_pid, "found existing isolate runner");

			let runner = runner::Handle::from_pid(
				runner::Comms::socket(),
				Pid::from_raw(isolate_runner_pid),
				self.isolate_runner_path(),
			);
			self.observe_isolate_runner(&runner);

			*guard = Some(runner);
		}

		Ok(())
	}

	/// Destroys all active actors and runners and resets the database.
	async fn reset(self: &Arc<Self>, workflow_id: Uuid) -> Result<()> {
		let ((last_workflow_id,), actor_rows) = tokio::try_join!(
			// There should not be any database operations going on at this point so it is safe to read this
			// value
			utils::sql::query(|| async {
				sqlx::query_as::<_, (Option<Uuid>,)>(indoc!(
					"
					SELECT last_workflow_id FROM state
					",
				))
				.fetch_one(&mut *self.sql().await?)
				.await
			}),
			utils::sql::query(|| async {
				sqlx::query_as::<_, ActorRow>(indoc!(
					"
					SELECT actor_id, config, pid, stop_ts
					FROM actors
					WHERE exit_ts IS NULL
					",
				))
				.fetch_all(&mut *self.sql().await?)
				.await
			})
		)?;

		let Some(last_workflow_id) = last_workflow_id else {
			return Ok(());
		};

		if workflow_id == last_workflow_id {
			return Ok(());
		}

		tracing::info!(
			?last_workflow_id,
			new_workflow_id=?workflow_id,
			"manager is resetting due to a workflow change"
		);

		let isolate_runner = { self.isolate_runner.read().await.clone() };

		// Kill isolate runner
		if let Some(isolate_runner) = &isolate_runner {
			isolate_runner.signal(Signal::SIGKILL)?;
		}

		for row in actor_rows {
			let Some(pid) = row.pid else {
				continue;
			};

			let config = serde_json::from_slice::<protocol::ActorConfig>(&row.config)?;

			match &isolate_runner {
				Some(isolate_runner) if pid == isolate_runner.pid().as_raw() => {}
				_ => {
					// Create a basic runner handle regardless of what the runner actually is (were just going to
					// kill it).
					let runner = runner::Handle::from_pid(
						runner::Comms::Basic,
						Pid::from_raw(pid),
						self.actor_path(row.actor_id),
					);

					// Kill runner
					runner.signal(Signal::SIGKILL)?;
				}
			}

			// Clean up actor. We run `cleanup_setup` instead of `cleanup` because `cleanup` publishes events.
			let actor = Actor::new(row.actor_id, config);
			actor.cleanup_setup(self).await;
		}

		// Stop any pending db operations
		let mut pool = self.pool.try_write().expect("should not be locked");
		pool.close().await;

		let db_path = self.config().data_dir().join("db");

		// Move database files to archive
		let archive_path = db_path.join("archive").join(last_workflow_id.to_string());
		fs::create_dir_all(&archive_path).await?;

		for file in ["database.db", "database.db-shm", "database.db-wal"] {
			let src = db_path.join(file);
			let dest = archive_path.join(file);

			if let Err(err) = fs::rename(&src, &dest).await {
				if err.kind() != std::io::ErrorKind::NotFound {
					return Err(err.into());
				}
			}
		}

		// Reinitialize db
		*pool = utils::init_sqlite_db(&self.config).await?;

		Ok(())
	}

	/// Sends all events after the given idx.
	async fn rebroadcast(&self, last_event_idx: i64) -> Result<()> {
		// Fetch all missed events
		let events = utils::sql::query(|| async {
			sqlx::query_as::<_, (i64, Vec<u8>)>(indoc!(
				"
				SELECT idx, payload
				FROM events
				WHERE idx > ?1
				",
			))
			.bind(last_event_idx)
			.fetch_all(&mut *self.sql().await?)
			.await
		})
		.await?
		.into_iter()
		.map(|(index, payload)| {
			Ok(protocol::EventWrapper {
				index,
				inner: protocol::Raw::from_string(String::from_utf8_lossy(&payload).into())?,
			})
		})
		.collect::<Result<Vec<_>>>()?;

		if events.is_empty() {
			return Ok(());
		}

		self.send_packet(protocol::ToServer::Events(events)).await
	}

	/// Rebuilds state from DB upon restart.
	async fn rebuild(self: &Arc<Self>, workflow_id: Uuid) -> Result<()> {
		let ((last_event_idx,), actor_rows) = tokio::try_join!(
			// There should not be any database operations going on at this point so it is safe to read this
			// value
			utils::sql::query(|| async {
				sqlx::query_as::<_, (i64,)>(indoc!(
					"
					UPDATE state
					SET	last_workflow_id = ?
					RETURNING last_event_idx
					",
				))
				.bind(workflow_id)
				.fetch_one(&mut *self.sql().await?)
				.await
			}),
			utils::sql::query(|| async {
				sqlx::query_as::<_, ActorRow>(indoc!(
					"
					SELECT actor_id, config, pid, stop_ts
					FROM actors
					WHERE exit_ts IS NULL
					",
				))
				.fetch_all(&mut *self.sql().await?)
				.await
			})
		)?;

		self.event_sender.set_idx(last_event_idx + 1);

		let isolate_runner = { self.isolate_runner.read().await.clone() };

		// NOTE: Sqlite doesn't support arrays, can't parallelize this easily
		// Emit stop events
		for row in &actor_rows {
			if row.pid.is_none() && row.stop_ts.is_none() {
				tracing::error!(actor_id=?row.actor_id, "actor has no pid, stopping");

				utils::sql::query(|| async {
					sqlx::query(indoc!(
						"
						UPDATE actors
						SET stop_ts = ?2
						WHERE actor_id = ?1
						",
					))
					.bind(row.actor_id)
					.bind(utils::now())
					.execute(&mut *self.sql().await?)
					.await
				})
				.await?;

				self.event(protocol::Event::ActorStateUpdate {
					actor_id: row.actor_id,
					state: protocol::ActorState::Lost,
				})
				.await?;
			}
		}

		// Start actor observers
		let mut actors_guard = self.actors.write().await;
		for row in actor_rows {
			let Some(pid) = row.pid else {
				continue;
			};

			let config = serde_json::from_slice::<protocol::ActorConfig>(&row.config)?;

			let runner = match &isolate_runner {
				// We have to clone the existing isolate runner handle instead of creating a new one so it
				// becomes a shared reference
				Some(isolate_runner) if pid == isolate_runner.pid().as_raw() => {
					isolate_runner.clone()
				}
				_ => match config.image.kind {
					protocol::ImageKind::DockerImage | protocol::ImageKind::OciBundle => {
						runner::Handle::from_pid(
							runner::Comms::Basic,
							Pid::from_raw(pid),
							self.actor_path(row.actor_id),
						)
					}
					protocol::ImageKind::JavaScript => runner::Handle::from_pid(
						runner::Comms::socket(),
						Pid::from_raw(pid),
						self.actor_path(row.actor_id),
					),
				},
			};

			let actor = Actor::with_runner(row.actor_id, config, runner);
			let actor = actors_guard.entry(row.actor_id).or_insert(actor);

			let actor = actor.clone();
			let self2 = self.clone();
			tokio::spawn(async move {
				if let Err(err) = actor.observe(&self2).await {
					tracing::error!(actor_id=?row.actor_id, ?err, "observe failed");
				}

				// Cleanup afterwards
				if let Err(err) = actor.cleanup(&self2).await {
					tracing::error!(actor_id=?row.actor_id, ?err, "cleanup failed");
				}
			});
		}

		Ok(())
	}
}

// MARK: Utils
impl Ctx {
	pub fn config(&self) -> &Client {
		&self.config.client
	}

	pub fn fdb_cluster_path(&self) -> PathBuf {
		self.config().data_dir().join("fdb.cluster")
	}

	pub fn actors_path(&self) -> PathBuf {
		self.config().data_dir().join("actors")
	}

	pub fn actor_path(&self, actor_id: Uuid) -> PathBuf {
		self.actors_path().join(actor_id.to_string())
	}

	pub fn isolate_runner_path(&self) -> PathBuf {
		self.config().data_dir().join("runner")
	}
}

// Test bindings
#[cfg(feature = "test")]
impl Ctx {
	pub fn actors(&self) -> &RwLock<HashMap<Uuid, Arc<Actor>>> {
		&self.actors
	}
}
