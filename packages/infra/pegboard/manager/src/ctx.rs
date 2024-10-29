use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use anyhow::*;
use futures_util::{
	stream::{SplitSink, SplitStream},
	SinkExt, StreamExt,
};
use indoc::indoc;
use nix::unistd::Pid;
use pegboard::protocol;
use sqlx::{pool::PoolConnection, Sqlite, SqlitePool};
use sysinfo::System;
use tokio::{
	net::{TcpListener, TcpStream},
	sync::{Mutex, RwLock},
};
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
use uuid::Uuid;

use crate::{actor::Actor, config::Config, metrics, runner, utils};

const PING_INTERVAL: Duration = Duration::from_secs(1);
/// TCP Port for runners to connect to.
const RUNNER_PORT: u16 = 54321;

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

	pub(crate) system: System,
	pub(crate) actors: RwLock<HashMap<Uuid, Arc<Actor>>>,
	isolate_runner: RwLock<Option<runner::Handle>>,
}

impl Ctx {
	pub fn new(
		config: Config,
		system: System,
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

	pub async fn start(
		self: &Arc<Self>,
		rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
	) -> Result<()> {
		// Start ping thread
		let self2 = self.clone();
		let ping_thread = tokio::spawn(async move {
			loop {
				tokio::time::sleep(PING_INTERVAL).await;
				self2
					.tx
					.lock()
					.await
					.send(Message::Ping(Vec::new()))
					.await?;
			}

			#[allow(unreachable_code)]
			Ok(())
		});

		// Start runner socket
		let self2 = self.clone();
		let runner_socket = tokio::spawn(async move {
			tracing::info!(port=%RUNNER_PORT, "listening for runner sockets");
			let listener = TcpListener::bind(("0.0.0.0", RUNNER_PORT)).await?;

			loop {
				use std::result::Result::{Err, Ok};
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

			#[allow(unreachable_code)]
			Ok(())
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
				system: protocol::SystemInfo {
					// Sum of cpu frequency
					cpu: self
						.system
						.cpus()
						.iter()
						.fold(0, |s, cpu| s + cpu.frequency()),
					memory: self.system.total_memory() / (1024 * 1024),
				},
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
			match msg? {
				Message::Binary(buf) => {
					metrics::PACKET_RECV_TOTAL.with_label_values(&[]).inc();

					let packet = protocol::ToClient::deserialize(&buf)?;

					self.process_packet(packet).await?;
				}
				Message::Pong(_) => tracing::debug!("received pong"),
				Message::Close(_) => {
					bail!("socket closed");
				}
				msg => {
					tracing::warn!(?msg, "unexpected message");
				}
			}
		}

		bail!("stream closed");
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
			protocol::Command::SignalActor { actor_id, signal } => {
				if let Some(actor) = self.actors.read().await.get(&actor_id) {
					actor.signal(&self, signal.try_into()?).await?;
				} else {
					tracing::warn!(
						?actor_id,
						"received stop actor command for actor that doesn't exist (likely already stopped)"
					);
				}
			}
		}

		// Ack command
		use std::result::Result::{Err, Ok};
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

	// Should not be called before `rebuild`.
	pub(crate) async fn get_or_spawn_isolate_runner(self: &Arc<Self>) -> Result<runner::Handle> {
		let mut guard = self.isolate_runner.write().await;

		if let Some(runner) = &*guard {
			Ok(runner.clone())
		} else {
			tracing::info!("spawning new isolate runner");

			let env = vec![
				(
					"VECTOR_SOCKET_ADDR",
					self.config.vector_socket_addr.to_string(),
				),
				(
					"ACTORS_PATH",
					self.actors_path().to_str().context("bad path")?.to_string(),
				),
			];
			let working_path = self.isolate_runner_path();

			let runner = runner::Handle::spawn_orphaned(
				runner::Comms::socket(),
				&self.config.isolate_runner_binary_path,
				working_path,
				&env,
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
			use std::result::Result::{Err, Ok};

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
				tracing::error!(%err, "failed to write isolate runner pid");
			}
		});
	}

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

	// Rebuilds state from DB
	async fn rebuild(self: &Arc<Self>) -> Result<()> {
		use std::result::Result::{Err, Ok};
		let (actor_rows, isolate_runner_pid) = tokio::try_join!(
			utils::query(|| async {
				sqlx::query_as::<_, ActorRow>(indoc!(
					"
					SELECT actor_id, config, pid, stop_ts
					FROM actors
					WHERE exit_ts IS NULL
					",
				))
				.fetch_all(&mut *self.sql().await?)
				.await
			}),
			utils::query(|| async {
				sqlx::query_as::<_, (Option<i32>,)>(indoc!(
					"
					SELECT isolate_runner_pid
					FROM state
					",
				))
				.fetch_one(&mut *self.sql().await?)
				.await
				.map(|x| x.0)
			}),
		)?;

		// Recreate isolate runner handle
		let isolate_runner = if let Some(isolate_runner_pid) = isolate_runner_pid {
			let mut guard = self.isolate_runner.write().await;

			let runner = runner::Handle::from_pid(
				runner::Comms::socket(),
				Pid::from_raw(isolate_runner_pid),
				self.isolate_runner_path(),
			);
			self.observe_isolate_runner(&runner);

			*guard = Some(runner.clone());

			Some(runner)
		} else {
			None
		};

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
					state: protocol::ActorState::Stopped,
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

			// Clone isolate runner handle
			let runner = if Some(pid) == isolate_runner_pid {
				isolate_runner.clone().expect("isolate runner must exist")
			} else {
				match config.image.kind {
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
				}
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
	pub fn config(&self) -> &Config {
		&self.config
	}

	pub fn actors_path(&self) -> PathBuf {
		self.config.working_path.join("actors")
	}

	pub fn actor_path(&self, actor_id: Uuid) -> PathBuf {
		self.actors_path().join(actor_id.to_string())
	}

	pub fn isolate_runner_path(&self) -> PathBuf {
		self.config.working_path.join("runner")
	}
}

// Test bindings
#[cfg(feature = "test")]
impl Ctx {
	pub fn actors(&self) -> &RwLock<HashMap<Uuid, Arc<Actor>>> {
		&self.actors
	}
}
