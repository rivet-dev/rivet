use std::{
	collections::HashMap,
	net::SocketAddr,
	sync::{Arc, RwLock},
};

use chirp_workflow::prelude::*;
use futures_util::FutureExt;
// use tokio::net::{TcpListener, TcpStream};
// use tokio_tungstenite::tungstenite::protocol::Message;

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {}

#[workflow]
pub async fn pegboard_ws(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	// let addr = "127.0.0.1:8080";
	// let listener = TcpListener::bind(&addr).await?;
	// println!("Listening on: {}", addr);

	let conns = Arc::new(RwLock::new(HashMap::<(), ()>::new()));

	ctx.try_join((
		closure(|ctx| socket_thread(ctx, conns.clone()).boxed()),
		closure(|ctx| signal_thread(ctx, conns.clone()).boxed()),
	))
	.await?;

	Ok(())
}

async fn socket_thread(
	ctx: &mut WorkflowCtx,
	conns: Arc<RwLock<HashMap<(), ()>>>,
) -> GlobalResult<()> {
	ctx.repeat(|ctx| {
		async move {
			if let Ok((stream, addr)) = listener.accept().await {
				handle_connection(stream, addr).await;
			} else {
				tracing::error!("failed to connect websocket");
			}

			Ok(Loop::Continue)
		}.boxed()
	)

	Ok(())
}

async fn signal_thread(
	ctx: &mut WorkflowCtx,
	conns: Arc<RwLock<HashMap<(), ()>>>,
) -> GlobalResult<()> {
	Ok(())
}

async fn handle_connection(ctx, raw_stream: TcpStream, addr: SocketAddr) {
	ctx.spawn(|ctx| async move {
		let ws_stream = tokio_tungstenite::accept_async(raw_stream).await?;
		let (mut write, mut read) = ws_stream.split();

		println!("New WebSocket connection: {}", addr);

		while let Some(Ok(msg)) = read.next().await {
			if msg.is_text() || msg.is_binary() {
				write.send(msg).await?;
			}
		}

		Ok(())
	}.boxed()).await
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct FooInput {}

#[activity(Foo)]
async fn foo(ctx: &ActivityCtx, input: &FooInput) -> GlobalResult<()> {
	Ok(())
}
