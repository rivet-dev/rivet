use std::sync::Arc;

use anyhow::*;
use futures_util::{
	stream::{SplitSink, SplitStream},
	SinkExt, StreamExt,
};
use pegboard::protocol;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::{tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};

use crate::{container::Container, ctx::Ctx};

pub struct Client {
	ctx: Ctx,
	tx: Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
}

impl Client {
	pub fn new(tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>) -> Arc<Self> {
		Arc::new(Client {
			ctx: Ctx::new(),
			tx: Mutex::new(tx),
		})
	}

	pub async fn write(self: Arc<Self>, packet: protocol::ToServer) -> Result<()> {
		let buf = packet.serialize()?;
		self.tx.lock().await.send(Message::Binary(buf)).await?;

		Ok(())
	}

	pub async fn start(
		self: Arc<Self>,
		mut rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
	) -> Result<()> {
		// Receive messages from socket
		while let Some(msg) = rx.next().await {
			match msg? {
				Message::Binary(buf) => {
					let packet = protocol::ToClient::deserialize(&buf)?;

					self.clone().process_packet(packet).await?;
				}
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

	async fn process_packet(self: Arc<Self>, packet: protocol::ToClient) -> Result<()> {
		match packet {
			protocol::ToClient::Init { .. } => todo!(),
			protocol::ToClient::Commands(commands) => {
				for command in commands {
					match command {
						protocol::Command::StartContainer {
							container_id,
							image_artifact_url,
							container_runner_binary_url,
							stakeholder,
						} => {
							let container = Container::new(
								container_id,
								image_artifact_url,
								container_runner_binary_url,
								stakeholder,
							);

							container.start(&self.ctx).await?;
						}
					}
				}
			}
			protocol::ToClient::FetchStateRequest {} => todo!(),
		}

		Ok(())
	}
}
