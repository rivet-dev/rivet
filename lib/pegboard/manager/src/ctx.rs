use std::{
	collections::HashMap,
	net::Ipv4Addr,
	path::{Path, PathBuf},
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
use pegboard::protocol;
use sqlx::{pool::PoolConnection, Sqlite, SqlitePool};
use sysinfo::System;
use tokio::{
	fs,
	net::TcpStream,
	process::Command,
	sync::{Mutex, RwLock},
};
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
use url::Url;
use uuid::Uuid;

use crate::{container::Container, metrics, utils};

const PING_INTERVAL: Duration = Duration::from_secs(1);

#[derive(sqlx::FromRow)]
struct ContainerRow {
	container_id: Uuid,
	config: Vec<u8>,
	pid: Option<i32>,
	stop_ts: Option<i64>,
}

pub struct Ctx {
	path: PathBuf,
	pool: SqlitePool,
	tx: Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,

	pub(crate) network_ip: Ipv4Addr,
	pub(crate) system: System,
	pub(crate) api_endpoint: RwLock<Option<String>>,
	pub(crate) containers: RwLock<HashMap<Uuid, Arc<Container>>>,
}

impl Ctx {
	pub fn new(
		path: PathBuf,
		network_ip: Ipv4Addr,
		system: System,
		pool: SqlitePool,
		tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
	) -> Arc<Self> {
		Arc::new(Ctx {
			path,
			pool,
			tx: Mutex::new(tx),

			network_ip,
			system,
			api_endpoint: RwLock::new(None),
			containers: RwLock::new(HashMap::new()),
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
		mut rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
	) -> Result<()> {
		// Start ping thread
		let self2 = self.clone();
		tokio::spawn(async move {
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

		// Receive messages from socket
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
			protocol::ToClient::Init {
				last_event_idx,
				api_endpoint,
			} => {
				{
					let mut guard = self.api_endpoint.write().await;
					*guard = Some(api_endpoint);
				}

				// Send out all missed events
				self.rebroadcast(last_event_idx).await?;

				// Rebuild state only after the init packet is received and processed
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
			protocol::Command::StartContainer {
				container_id,
				config,
			} => {
				// Insert container
				let mut containers = self.containers.write().await;
				let container = containers
					.entry(container_id)
					.or_insert_with(|| Container::new(container_id, *config));

				// Spawn container
				container.start(&self).await?;
			}
			protocol::Command::SignalContainer {
				container_id,
				signal,
			} => {
				if let Some(container) = self.containers.read().await.get(&container_id) {
					container.signal(&self, signal.try_into()?).await?;
				} else {
					tracing::warn!(
						?container_id,
						"received stop container command for container that doesn't exist (likely already stopped)"
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
		let container_rows = utils::query(|| async {
			sqlx::query_as::<_, ContainerRow>(indoc!(
				"
				SELECT container_id, config, pid, stop_ts
				FROM containers
				WHERE exit_ts IS NULL
				",
			))
			.fetch_all(&mut *self.sql().await?)
			.await
		})
		.await?;

		// NOTE: Sqlite doesn't support arrays, can't parallelize this easily
		// Emit stop events
		for row in &container_rows {
			if row.pid.is_none() && row.stop_ts.is_none() {
				tracing::error!(container_id=?row.container_id, "container has no pid, stopping");

				utils::query(|| async {
					sqlx::query(indoc!(
						"
						UPDATE containers
						SET stop_ts = ?2
						WHERE container_id = ?1
						",
					))
					.bind(row.container_id)
					.bind(utils::now())
					.execute(&mut *self.sql().await?)
					.await
				})
				.await?;

				self.event(protocol::Event::ContainerStateUpdate {
					container_id: row.container_id,
					state: protocol::ContainerState::Stopped,
				})
				.await?;
			}
		}

		// Start container observers
		let mut containers_guard = self.containers.write().await;
		for row in container_rows {
			let Some(pid) = row.pid else {
				continue;
			};

			let pid = Pid::from_raw(pid);
			let config = serde_json::from_slice(&row.config)?;
			let container = Container::with_pid(row.container_id, config, pid);
			let container = containers_guard
				.entry(row.container_id)
				.or_insert(container);

			let container = container.clone();
			let self2 = self.clone();
			tokio::spawn(async move {
				use std::result::Result::Err;

				if let Err(err) = container.observe(&self2, pid).await {
					tracing::error!(container_id=?row.container_id, ?err, "observe failed");
				}

				// Cleanup afterwards
				container.cleanup(&self2).await
			});
		}

		Ok(())
	}
}

impl Ctx {
	pub async fn fetch_container_runner(
		&self,
		container_runner_binary_url: &str,
	) -> Result<PathBuf> {
		let url = Url::parse(container_runner_binary_url)?;
		let path_stub = utils::get_s3_path_stub(&url, true)?;
		let path = self.runner_binaries_path().join(&path_stub);

		// Check file doesn't exist
		if fs::metadata(&path).await.is_err() {
			let parent = path_stub
				.parent()
				.filter(|x| x.components().next().is_some())
				.context("no parent path in runner url")?;

			fs::create_dir(self.runner_binaries_path().join(parent)).await?;

			tracing::info!(%container_runner_binary_url, "downloading");
			utils::download_file(container_runner_binary_url, &path).await?;

			let cmd_out = Command::new("chmod").arg("+x").arg(&path).output().await?;
			ensure!(
				cmd_out.status.success(),
				"failed chmod command\n{}",
				std::str::from_utf8(&cmd_out.stderr)?
			);
		}

		Ok(path.to_path_buf())
	}
}

impl Ctx {
	pub fn working_path(&self) -> &Path {
		self.path.as_path()
	}

	pub fn container_path(&self, container_id: Uuid) -> PathBuf {
		self.working_path()
			.join("containers")
			.join(container_id.to_string())
	}

	pub fn runner_binaries_path(&self) -> PathBuf {
		self.working_path().join("bin")
	}
}

// Test bindings
#[cfg(feature = "test")]
impl Ctx {
	pub fn containers(&self) -> &RwLock<HashMap<Uuid, Arc<Container>>> {
		&self.containers
	}
}
