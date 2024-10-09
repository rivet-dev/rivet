use std::{
	collections::HashMap,
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
use pegboard::protocol;
use sqlx::{pool::PoolConnection, Sqlite, SqlitePool};
use tokio::{
	fs,
	net::TcpStream,
	process::Command,
	sync::{Mutex, RwLock},
};
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
use url::Url;
use uuid::Uuid;

use crate::{container::Container, utils};

const PING_INTERVAL: Duration = Duration::from_secs(1);

pub struct Ctx {
	path: PathBuf,
	pool: SqlitePool,
	tx: Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,

	pub(crate) api_endpoint: RwLock<Option<String>>,
	pub(crate) containers: RwLock<HashMap<Uuid, Arc<Container>>>,
}

impl Ctx {
	pub fn new(
		path: PathBuf,
		pool: SqlitePool,
		tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
	) -> Arc<Self> {
		Arc::new(Ctx {
			path,
			pool,
			tx: Mutex::new(tx),

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

		Ok(())
	}

	async fn write_event(&self, event: &protocol::Event) -> Result<i64> {
		// Write event to db
		let event_json = serde_json::to_vec(event)?;
		let (index,) = utils::query(|| async {
			sqlx::query_as::<_, (i64,)>(indoc!(
				"
				WITH
					last_idx AS (
						UPDATE state
						SET last_event_idx = last_event_idx + 1
						RETURNING last_event_idx - 1
					),
					insert_event AS (
						INSERT INTO events (
							index,
							payload,
							create_ts
						)
						SELECT last_idx.last_event_idx, ?1, ?2
						FROM last_idx
						LIMIT 1
						RETURNING 1
					)
				SELECT last_event_idx FROM last_idx
				",
			))
			.bind(&event_json)
			.bind(utils::now())
			.fetch_one(&mut *self.sql().await?)
			.await
		})
		.await?;

		Ok(index)
	}

	pub async fn event(&self, event: protocol::Event) -> Result<()> {
		let index = self.write_event(&event).await?;

		let wrapped_event = protocol::EventWrapper {
			index,
			inner: protocol::Raw::serialize(&event)?,
		};

		self.send_packet(protocol::ToServer::Events(vec![wrapped_event]))
			.await
	}

	// Rebuilds state from DB
	pub async fn rebuild(self: &Arc<Self>, last_event_idx: i64) -> Result<()> {
		todo!();
	}

	pub async fn start(
		self: &Arc<Self>,
		mut rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
	) -> Result<()> {
		// Start ping thread
		let s = self.clone();
		tokio::spawn(async move {
			loop {
				tokio::time::sleep(PING_INTERVAL).await;
				s.tx.lock().await.send(Message::Ping(Vec::new())).await?;
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

			self.send_packet(protocol::ToServer::Init { last_command_idx })
				.await?;
		}

		// Receive messages from socket
		while let Some(msg) = rx.next().await {
			match msg? {
				Message::Binary(buf) => {
					let packet = protocol::ToClient::deserialize(&buf)?;

					self.clone().process_packet(packet).await?;
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

		// TODO: Implement pinging

		bail!("stream closed");
	}

	async fn process_packet(self: &Arc<Self>, packet: protocol::ToClient) -> Result<()> {
		match packet {
			protocol::ToClient::Init {
				last_event_idx,
				api_endpoint,
			} => {
				{
					let mut guard = self.api_endpoint.write().await;
					*guard = Some(api_endpoint);
				}

				self.rebuild(last_event_idx).await?;
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
					.or_insert_with(|| Container::new(container_id));

				// Spawn container
				container.start(&self, *config).await?;
			}
			protocol::Command::StopContainer { container_id } => {
				if let Some(container) = self.containers.read().await.get(&container_id) {
					container.stop(&self).await?;
				} else {
					tracing::warn!(
						?container_id,
						"received stop container command for container that doesn't exist (likely already stopped)"
					);
				}
			}
		}

		// Ack command
		utils::query(|| async {
			sqlx::query(indoc!(
				"
				WITH
					update_last_idx AS (
						UPDATE state
						SET last_command_idx = ?2
						RETURNING 1
					),
					insert_event AS (
						INSERT INTO commands (
							index,
							payload,
							create_ts
						)
						VALUES(?1, ?2, ?3)
						RETURNING 1
					)
				SELECT 1
				",
			))
			.bind(command.index)
			.bind(&command.inner)
			.bind(utils::now())
			.execute(&mut *self.sql().await?)
			.await
		})
		.await?;

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
		let path = self.runner_binaries_path().join(path_stub);

		// Check file doesn't exist
		if fs::metadata(&path).await.is_err() {
			utils::download_file(container_runner_binary_url, &path).await?;

			Command::new("chmod").arg("+x").arg(&path).output().await?;
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
