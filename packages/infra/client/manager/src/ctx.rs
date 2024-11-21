use std::{
	collections::HashMap,
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
use nix::unistd::Pid;
use pegboard::{protocol, system_info::SystemInfo};
use pegboard_config::{isolate_runner::Config as IsolateRunnerConfig, Client, Config};
use sqlx::{pool::PoolConnection, Sqlite, SqlitePool};
use tokio::{
	fs,
	net::{TcpListener, TcpStream},
	sync::{Mutex, RwLock},
};
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
use url::Url;
use uuid::Uuid;

use crate::{actor::Actor, metrics, runner, utils};

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
	#[error("socket closed")]
	SocketClosed,
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
	pool: SqlitePool,
	tx: Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,

	system: SystemInfo,
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
			pool,
			tx: Mutex::new(tx),

			system,
			actors: RwLock::new(HashMap::new()),
			isolate_runner: RwLock::new(None),
		})
	}

	pub async fn sql(&self) -> std::result::Result<PoolConnection<Sqlite>, sqlx::Error> {
		// Attempt to use an existing connection
		if let Some(conn) = self.pool.try_acquire() {
			std::result::Result::Ok(conn)
		} else {
			// Create a new connection
			self.pool.acquire().await.map_err(Into::into)
		}
	}

	async fn send_packet(&self, packet: protocol::ToServer) -> Result<()> {
		let buf = packet.serialize()?;
		self.tx.lock().await.send(Message::Binary(buf)).await?;

		metrics::PACKET_SEND_TOTAL.with_label_values(&[]).inc();

		Ok(())
	}

	async fn write_event(&self, event: &protocol::Event) -> Result<i64> {
		// Fetch next idx
		let (index,) = utils::query(|| async {
			sqlx::query_as::<_, (i64,)>(indoc!(
				"
				UPDATE state
				SET last_event_idx = last_event_idx + 1
				RETURNING last_event_idx
				",
			))
			.fetch_one(&mut *self.sql().await?)
			.await
		})
		.await?;

		// Write event to db
		let event_json = serde_json::to_vec(event)?;
		utils::query(|| async {
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
			.execute(&mut *self.sql().await?)
			.await
		})
		.await?;

		Ok(index)
	}

	pub async fn event(&self, event: protocol::Event) -> Result<()> {
		let index = self.write_event(&event).await?;

		let wrapped_event = protocol::EventWrapper {
			index,
			inner: protocol::Raw::new(&event)?,
		};

		self.send_packet(protocol::ToServer::Events(vec![wrapped_event]))
			.await
	}

	pub async fn run(
		self: &Arc<Self>,
		rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
	) -> Result<()> {
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

		// Rebuild isolate runner from db
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
						let mut socket = tokio_tungstenite::accept_async(stream).await?;

						if let Some(runner) = &*self2.isolate_runner.read().await {
							runner.attach_socket(socket).await?;
						} else {
							socket.close(None).await?;
							bail!("no isolate runner to attach socket to");
						}
					}
					Err(err) => tracing::error!(?err, "failed to connect websocket"),
				}
			}
		});

		// Send init packet
		{
			let (last_command_idx,) = utils::query(|| async {
				sqlx::query_as::<_, (i64,)>(indoc!(
					"
					SELECT last_command_idx FROM state
					",
				))
				.fetch_one(&mut *self.sql().await?)
				.await
			})
			.await?;

			self.send_packet(protocol::ToServer::Init {
				last_command_idx,
				config: self.config.build_client_config(),
				system: self.system.clone(),
			})
			.await?;
		}

		tokio::try_join!(
			async { ping_thread.await? },
			async { runner_socket.await? },
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
				Message::Close(_) => return Err(RuntimeError::SocketClosed.into()),
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
			protocol::ToClient::Init { last_event_idx } => {
				// Send out all missed events
				self.rebroadcast(last_event_idx).await?;

				// Rebuild state only after the init packet is received and processed so that we don't emit
				// any new events before the missed events are rebroadcast
				self.rebuild().await?;
			}
			protocol::ToClient::Commands(commands) => {
				for command in commands {
					self.process_command(command).await?;
				}
			}
			protocol::ToClient::FetchStateRequest {} => todo!(),
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
				persist_state,
			} => {
				if let Some(actor) = self.actors.read().await.get(&actor_id) {
					actor
						.signal(&self, signal.try_into()?, persist_state)
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
			utils::query(|| async {
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
			utils::query(|| async {
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
				.bind(&command.inner)
				.bind(utils::now())
				.execute(&mut *self.sql().await?)
				.await
			}),
		)?;

		Ok(())
	}

	/// Sends all events after the given idx.
	async fn rebroadcast(&self, last_event_idx: i64) -> Result<()> {
		// Fetch all missed events
		let events = utils::query(|| async {
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
	async fn rebuild(self: &Arc<Self>) -> Result<()> {
		let actor_rows = utils::query(|| async {
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
		.await?;

		let isolate_runner = { self.isolate_runner.read().await.clone() };

		// NOTE: Sqlite doesn't support arrays, can't parallelize this easily
		// Emit stop events
		for row in &actor_rows {
			if row.pid.is_none() && row.stop_ts.is_none() {
				tracing::error!(actor_id=?row.actor_id, "actor has no pid, stopping");

				utils::query(|| async {
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
				use std::result::Result::Err;

				if let Err(err) = actor.observe(&self2).await {
					tracing::error!(actor_id=?row.actor_id, ?err, "observe failed");
				}

				// Cleanup afterwards
				actor.cleanup(&self2).await
			});
		}

		Ok(())
	}
}

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
				runner_addr: SocketAddr::from([127, 0, 0, 1], self.config().runner.port()),
			};

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
				self.config().runner.use_cgroup(),
			)?;
			let pid = runner.pid();

			self.observe_isolate_runner(&runner);

			// Save runner pid
			utils::query(|| async {
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
		tracing::info!("observing isolate runner");

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

			tracing::error!(?exit_code, "isolate runner exited");

			// Update in-memory state
			let mut guard = self2.isolate_runner.write().await;
			*guard = None;

			// Update db state
			let res = utils::query(|| async {
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

	/// Fetches isolate runner state from the db. Should be called before the manager's runner websocket opens.
	async fn rebuild_isolate_runner(self: &Arc<Self>) -> Result<()> {
		let (isolate_runner_pid,) = utils::query(|| async {
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

			tracing::info!(?isolate_runner_pid, "found old isolate runner");

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
}

impl Ctx {
	pub fn config(&self) -> &Client {
		&self.config.client
	}

	pub fn fdb_cluster_path(&self) -> PathBuf {
		self.config().runtime.data_dir().join("fdb.cluster")
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
