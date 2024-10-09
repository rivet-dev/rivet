use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use chirp_workflow::prelude::*;
use futures_util::{stream::SplitSink, SinkExt, StreamExt, TryStreamExt};
use serde_json::json;
use tokio::{
	net::{TcpListener, TcpStream},
	sync::RwLock,
};
use tokio_tungstenite::{tungstenite::protocol::Message, WebSocketStream};

type Connections = HashMap<Uuid, SplitSink<WebSocketStream<TcpStream>, Message>>;

#[tracing::instrument(skip_all)]
pub async fn run_from_env(pools: rivet_pools::Pools) -> GlobalResult<()> {
	let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("pegboard-ws");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = StandaloneCtx::new(
		chirp_workflow::compat::db_from_pools(&pools).await?,
		rivet_connection::Connection::new(client, pools, cache),
		"pegboard-ws",
	)
	.await?;

	let conns: Arc<RwLock<Connections>> = Arc::new(RwLock::new(HashMap::new()));

	tokio::try_join!(
		socket_thread(&ctx, conns.clone()),
		signal_thread(&ctx, conns.clone()),
	)?;

	Ok(())
}

async fn socket_thread(ctx: &StandaloneCtx, conns: Arc<RwLock<Connections>>) -> GlobalResult<()> {
	let addr = ("127.0.0.1", 8080);
	let listener = TcpListener::bind(&addr).await?;
	tracing::info!("Listening on: {:?}", addr);

	loop {
		match listener.accept().await {
			Ok((stream, addr)) => handle_connection(ctx, conns.clone(), stream, addr).await,
			Err(err) => tracing::error!(?err, "failed to connect websocket"),
		}
	}
}

async fn signal_thread(ctx: &StandaloneCtx, conns: Arc<RwLock<Connections>>) -> GlobalResult<()> {
	loop {
		let sig = ctx.listen::<Command>(&json!({})).await?;

		{
			let conns = conns.read().await;

			if let Some(write) = conns.get(sig.client_id) {
				write.send(sig);
			}
		}
	}
}

async fn handle_connection(
	ctx: &StandaloneCtx,
	conns: Arc<RwLock<Connections>>,
	raw_stream: TcpStream,
	addr: SocketAddr,
) {
	let ctx = ctx.clone();

	tokio::spawn(async move {
		let ws_stream = tokio_tungstenite::accept_async(raw_stream).await?;
		let (tx, mut rx) = ws_stream.split();

		let client_id = Uuid::new_v4();

		{
			let mut conns = conns.write().await;

			if let Some(mut old_tx) = conns.insert(client_id, tx) {
				tracing::error!(
					?client_id,
					"client already connected, overwriting old connection"
				);
				old_tx.send(Message::Close(None)).await?;
			}
		}

		// todo!("check if client exists in sql");

		while let Ok(msg) = rx.try_next().await {
			ctx.signal(ClientEvent {})
				.tag("client_id", client_id)
				.send()
				.await?;
		}

		GlobalResult::Ok(())
	});
}

#[signal("pegboard_command")]
pub struct Command {}

#[signal("pegboard_client_event")]
pub struct ClientEvent {}
