use std::{future::Future, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::{Request, Response, body::Incoming, service::service_fn};
use hyper_util::rt::TokioIo;
use rivet_guard_core::ResponseBody;
use rivet_guard_core::WebSocketHandle;
use rivet_guard_core::custom_serve::CustomServeTrait;
use rivet_guard_core::proxy_service::ProxyServiceFactory;
use rivet_guard_core::request_context::RequestContext;
use rivet_guard_core::{RouteConfig, RouteTarget, RoutingFn, RoutingOutput};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::frame::CloseFrame;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tokio_util::task::AbortOnDropHandle;

struct TestServer {
	addr: SocketAddr,
	_task: AbortOnDropHandle<()>,
}

impl TestServer {
	async fn start<F, Fut>(handler: F) -> Self
	where
		F: Fn(Request<Incoming>, SocketAddr) -> Fut + Clone + Send + 'static,
		Fut: Future<Output = Result<Response<ResponseBody>>> + Send + 'static,
	{
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let addr = listener.local_addr().unwrap();
		let task = tokio::spawn(async move {
			let mut connections = JoinSet::new();
			loop {
				tokio::select! {
					accepted = listener.accept() => {
						let (stream, remote) = accepted.unwrap();
						let handler = handler.clone();
						connections.spawn(async move {
							let service = service_fn(move |request| handler(request, remote));
							let _ = hyper::server::conn::http1::Builder::new()
								.serve_connection(TokioIo::new(stream), service)
								.with_upgrades().await;
						});
					}
					_ = connections.join_next(), if !connections.is_empty() => {}
				}
			}
		});
		Self {
			addr,
			_task: AbortOnDropHandle::new(task),
		}
	}
}

struct QuietWebSocket;

#[async_trait]
impl CustomServeTrait for QuietWebSocket {
	async fn handle_request(
		&self,
		_request: Request<Full<Bytes>>,
		_context: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
		anyhow::bail!("only WebSocket requests are supported");
	}

	async fn handle_websocket(
		&self,
		_context: &mut RequestContext,
		websocket: WebSocketHandle,
		_after_hibernation: bool,
	) -> Result<Option<CloseFrame>> {
		let receiver = websocket.recv();
		while let Some(message) = receiver.lock().await.next().await {
			if message?.is_close() {
				break;
			}
		}
		Ok(None)
	}
}

async fn assert_idle_pings(routing: RoutingFn) {
	let _ = rustls::crypto::ring::default_provider().install_default();
	let config = rivet_config::Config::from_root(rivet_config::config::Root {
		guard: Some(rivet_config::config::guard::Guard {
			websocket_keepalive_interval_ms: Some(25),
			..Default::default()
		}),
		..Default::default()
	});
	let factory = Arc::new(ProxyServiceFactory::new(
		config,
		routing,
		Arc::new(|_| Ok(0)),
	));
	let server = TestServer::start({
		let factory = factory.clone();
		move |request, remote| {
			let service = factory.create_service(remote);
			async move { service.process(request).await }
		}
	})
	.await;
	let mut request = format!("ws://{}/quiet", server.addr)
		.into_client_request()
		.unwrap();
	request
		.headers_mut()
		.insert("sec-websocket-protocol", "rivet".parse().unwrap());
	let (mut client, _) = connect_async(request).await.unwrap();
	for _ in 0..2 {
		let message = tokio::time::timeout(Duration::from_secs(2), client.next())
			.await
			.expect("Guard should ping without application traffic")
			.expect("connection should stay open")
			.expect("read ping");
		assert!(matches!(message, Message::Ping(_)));
		client.flush().await.unwrap();
	}
	client.close(None).await.unwrap();
	tokio::time::timeout(Duration::from_secs(2), factory.wait_idle())
		.await
		.expect("closing the client should stop Guard's WebSocket task");
}

#[tokio::test]
async fn custom_serve_idle_websocket_receives_keepalive_pings() {
	let routing: RoutingFn =
		Arc::new(|_| Box::pin(async { Ok(RoutingOutput::CustomServe(Arc::new(QuietWebSocket))) }));
	assert_idle_pings(routing).await;
}

#[tokio::test]
async fn proxied_idle_websocket_receives_keepalive_pings() {
	let upstream = TestServer::start(|request, _| async move {
		let (mut response, upgrade) = hyper_tungstenite::upgrade(request, None).unwrap();
		response
			.headers_mut()
			.insert("sec-websocket-protocol", "rivet".parse().unwrap());
		tokio::spawn(async move {
			let mut socket = upgrade.await.unwrap();
			while let Some(Ok(message)) = socket.next().await {
				if message.is_close() {
					break;
				}
				socket.flush().await.unwrap();
			}
		});
		Ok(response.map(ResponseBody::Full))
	})
	.await;
	let address = upstream.addr;
	let routing: RoutingFn = Arc::new(move |_| {
		Box::pin(async move {
			Ok(RoutingOutput::Route(RouteConfig {
				targets: vec![RouteTarget {
					host: address.ip().to_string(),
					port: address.port(),
					path: "/quiet".to_owned(),
				}],
			}))
		})
	});
	assert_idle_pings(routing).await;
}

#[tokio::test]
async fn keepalive_lives_until_the_last_handle_is_dropped() {
	let (handles_tx, mut handles_rx) = mpsc::unbounded_channel();
	let server = TestServer::start(move |request, _| {
		let handles_tx = handles_tx.clone();
		async move {
			let (response, upgrade) = hyper_tungstenite::upgrade(request, None).unwrap();
			tokio::spawn(async move {
				let handle = WebSocketHandle::new_with_keepalive(
					upgrade,
					Duration::from_millis(25),
					Duration::from_secs(1),
				)
				.await
				.unwrap();
				handles_tx.send(handle).unwrap();
			});
			Ok(response.map(ResponseBody::Full))
		}
	})
	.await;
	let (mut client, _) = connect_async(format!("ws://{}/", server.addr))
		.await
		.unwrap();
	let handle = handles_rx.recv().await.unwrap();
	let clone = handle.clone();
	drop(handle);

	let message = tokio::time::timeout(Duration::from_secs(2), client.next())
		.await
		.unwrap()
		.unwrap()
		.unwrap();
	assert!(matches!(message, Message::Ping(_)));
	client.flush().await.unwrap();
	drop(clone);

	// A background task retaining the split writer would keep the TCP socket open.
	tokio::time::timeout(Duration::from_secs(2), async {
		while let Some(Ok(_)) = client.next().await {}
	})
	.await
	.expect("dropping the final handle must release the keepalive writer");
}
