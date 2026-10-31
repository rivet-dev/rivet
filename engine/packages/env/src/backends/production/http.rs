//! Production reqwest HTTP execution and Tokio/Axum socket listeners.

#![allow(
	clippy::disallowed_types,
	reason = "this production adapter owns TCP listeners and streams"
)]

use crate::capabilities::http::{
	AcceptConnection, Connection, HttpBody, HttpClient, HttpFuture, HttpListener, HttpServer,
	PeerLane, PeerRoute, Request, RequestNotSent, Response, ResponseTooLarge, ShutdownFuture,
};
use anyhow::Result;
use bytes::Bytes;
use futures_util::StreamExt;
use rivet_metrics::GaugeGuardExt;
use std::{net::SocketAddr, time::Duration};

lazy_static::lazy_static! {
	static ref SERVER_CONNECTIONS: rivet_metrics::prometheus::IntGauge =
		rivet_metrics::prometheus::register_int_gauge_with_registry!(
			rivet_metrics::prometheus::Opts::new(
				"http_server_connections",
				"Live HTTP connections this process's server holds open. Each peer keeps a few per lane, so a sudden drop means connections were reset.",
			),
			*rivet_metrics::REGISTRY
		)
		.unwrap();
	static ref PEER_CONNECTIONS_OPENED: rivet_metrics::prometheus::IntCounterVec =
		rivet_metrics::prometheus::register_int_counter_vec_with_registry!(
			rivet_metrics::prometheus::Opts::new(
				"http_peer_connections_opened_total",
				"Peer HTTP/2 connections opened. Each stays open while it is used, so a steady rise means connections are failing and being replaced.",
			),
			&["lane"],
			*rivet_metrics::REGISTRY
		)
		.unwrap();
	static ref SERVER_CONNECTIONS_CAPACITY: rivet_metrics::prometheus::IntGauge =
		rivet_metrics::prometheus::register_int_gauge_with_registry!(
			rivet_metrics::prometheus::Opts::new(
				"http_server_connections_capacity",
				"Configured maximum live HTTP connections across active listeners.",
			),
			*rivet_metrics::REGISTRY
		).unwrap();
	static ref SERVER_CONNECTIONS_REJECTED: rivet_metrics::prometheus::IntCounter =
		rivet_metrics::prometheus::register_int_counter_with_registry!(
			rivet_metrics::prometheus::Opts::new(
				"http_server_connections_rejected_total",
				"TCP connections closed at accept because the server connection cap was full.",
			),
			*rivet_metrics::REGISTRY
		).unwrap();
}

/// Peer connection sets. Each reqwest client keeps one HTTP/2 connection per peer, so a set
/// of N clients gives N connections to every peer with no custom pool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeerTransport {
	pub request_connections: usize,
	pub stream_connections: usize,
	pub request_connection_window_bytes: u32,
	pub request_stream_window_bytes: u32,
	pub stream_connection_window_bytes: u32,
	pub stream_stream_window_bytes: u32,
}

impl Default for PeerTransport {
	fn default() -> Self {
		Self {
			request_connections: 4,
			stream_connections: 2,
			request_connection_window_bytes: 16 * 1024 * 1024,
			request_stream_window_bytes: 4 * 1024 * 1024,
			stream_connection_window_bytes: PEER_CONNECTION_WINDOW_BYTES,
			stream_stream_window_bytes: 1024 * 1024,
		}
	}
}

pub struct ReqwestClient {
	buffered: reqwest::Client,
	/// Connections for unary requests, chosen by the route key.
	requests: Vec<reqwest::Client>,
	/// Connections for long-lived streams, chosen by the route key.
	streams: Vec<reqwest::Client>,
}

impl ReqwestClient {
	/// Trusted peer HTTP with no redirects or ambient request/read deadline.
	/// Build here rather than accepting preconfigured clients whose hidden timeouts
	/// can race the caller's injected clock. The caller must bound execute through Env.
	pub fn for_peers(user_agent: &str) -> Result<Self> {
		Self::for_peers_with(user_agent, &PeerTransport::default())
	}

	/// [`ReqwestClient::for_peers`] with explicit connection sets.
	pub fn for_peers_with(user_agent: &str, transport: &PeerTransport) -> Result<Self> {
		anyhow::ensure!(
			transport.request_connections > 0 && transport.stream_connections > 0,
			"peer connection sets must not be empty"
		);
		let requests = (0..transport.request_connections)
			.map(|_| {
				peer_client(
					user_agent,
					PeerLane::Request,
					transport.request_connection_window_bytes,
					transport.request_stream_window_bytes,
				)
			})
			.collect::<Result<Vec<_>>>()?;
		let streams = (0..transport.stream_connections)
			.map(|_| {
				peer_client(
					user_agent,
					PeerLane::Stream,
					transport.stream_connection_window_bytes,
					transport.stream_stream_window_bytes,
				)
			})
			.collect::<Result<Vec<_>>>()?;
		Ok(Self {
			buffered: reqwest::Client::builder()
				.user_agent(user_agent)
				.redirect(reqwest::redirect::Policy::none())
				.build()?,
			requests,
			streams,
		})
	}

	/// The client a streaming request uses. A request without a route takes the first
	/// stream connection.
	fn streaming(&self, route: Option<&PeerRoute>) -> &reqwest::Client {
		let (clients, key) = match route {
			Some(PeerRoute {
				lane: PeerLane::Request,
				key,
			}) => (&self.requests, *key),
			Some(PeerRoute {
				lane: PeerLane::Stream,
				key,
			}) => (&self.streams, *key),
			None => (&self.streams, 0),
		};
		&clients[(key % clients.len() as u64) as usize]
	}
}

/// How long a peer client waits for a TCP connection to a peer.
const PEER_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// One peer HTTP/2 client, which keeps a single connection to each peer.
fn peer_client(
	user_agent: &str,
	lane: PeerLane,
	connection_window_bytes: u32,
	stream_window_bytes: u32,
) -> Result<reqwest::Client> {
	Ok(reqwest::Client::builder()
		.user_agent(user_agent)
		.redirect(reqwest::redirect::Policy::none())
		.http2_prior_knowledge()
		// A blackholed peer would otherwise hold a connection attempt for the kernel's SYN
		// retry budget, about two minutes.
		.connect_timeout(PEER_CONNECT_TIMEOUT)
		.http2_keep_alive_interval(std::time::Duration::from_secs(30))
		.http2_keep_alive_timeout(std::time::Duration::from_secs(10))
		.http2_initial_connection_window_size(connection_window_bytes)
		.http2_initial_stream_window_size(stream_window_bytes)
		.connector_layer(tower::util::MapResponseLayer::new(move |connection| {
			PEER_CONNECTIONS_OPENED
				.with_label_values(&[lane.as_str()])
				.inc();
			connection
		}))
		.build()?)
}

impl HttpClient for ReqwestClient {
	fn execute_stream(&self, request: Request<HttpBody>) -> HttpFuture<'_, Response<HttpBody>> {
		Box::pin(async move {
			let client = self.streaming(request.extensions().get::<PeerRoute>());
			let mut request = reqwest::Request::try_from(request.map(reqwest::Body::wrap_stream))?;
			*request.timeout_mut() = None;
			let response = client.execute(request).await.map_err(|error| {
				if error.is_connect() {
					anyhow::Error::from(error).context(RequestNotSent)
				} else {
					error.into()
				}
			})?;
			anyhow::ensure!(
				response.version() == http::Version::HTTP_2,
				"streaming requires HTTP/2"
			);

			let mut builder = Response::builder()
				.status(response.status())
				.version(response.version());
			*builder.headers_mut().expect("valid HTTP response builder") =
				response.headers().clone();
			let body = response
				.bytes_stream()
				.map(|result| result.map_err(anyhow::Error::from))
				.boxed();
			Ok(builder.body(body)?)
		})
	}

	fn execute(
		&self,
		request: Request<Bytes>,
		max_response_bytes: usize,
	) -> HttpFuture<'_, Response<Bytes>> {
		Box::pin(async move {
			let mut request = reqwest::Request::try_from(request)?;
			// Clear any reqwest timeout extension carried by an interoperable HTTP request.
			*request.timeout_mut() = None;
			let mut response = self.buffered.execute(request).await?;
			let status = response.status();
			let mut builder = Response::builder()
				.status(status)
				.version(response.version());
			*builder.headers_mut().expect("valid HTTP response builder") =
				response.headers().clone();
			// Callers classify protocol failures by status, without waiting for an error body.
			if !status.is_success() {
				return Ok(builder.body(Bytes::new())?);
			}
			let mut body = Vec::new();
			while let Some(chunk) = response.chunk().await? {
				if body.len().saturating_add(chunk.len()) > max_response_bytes {
					return Err(ResponseTooLarge.into());
				}
				body.extend_from_slice(&chunk);
			}
			Ok(builder.body(Bytes::from(body))?)
		})
	}
}

pub struct AxumServer;

/// An HTTP listener with a configurable process connection limit.
pub struct BoundedAxumServer {
	capacity: usize,
}

impl AxumServer {
	pub fn with_connection_capacity(capacity: usize) -> BoundedAxumServer {
		BoundedAxumServer { capacity }
	}
}

struct Listener(tokio::net::TcpListener, usize);

impl HttpServer for AxumServer {
	fn bind(&self, address: SocketAddr) -> HttpFuture<'_, Box<dyn HttpListener>> {
		Box::pin(async move {
			let listener = tokio::net::TcpListener::bind(address).await?;
			Ok(Box::new(Listener(listener, 8192)) as Box<dyn HttpListener>)
		})
	}
}

impl HttpServer for BoundedAxumServer {
	fn bind(&self, address: SocketAddr) -> HttpFuture<'_, Box<dyn HttpListener>> {
		let capacity = self.capacity;
		Box::pin(async move {
			let listener = tokio::net::TcpListener::bind(address).await?;
			Ok(Box::new(Listener(listener, capacity)) as Box<dyn HttpListener>)
		})
	}
}

impl HttpListener for Listener {
	fn local_addr(&self) -> Result<SocketAddr> {
		Ok(self.0.local_addr()?)
	}

	fn serve(
		self: Box<Self>,
		router: axum::Router,
		shutdown: ShutdownFuture,
	) -> HttpFuture<'static, ()> {
		Box::pin(serve(
			self.0,
			router,
			shutdown,
			Duration::from_secs(30),
			Duration::from_secs(10),
			self.1,
		))
	}

	fn serve_connections(
		self: Box<Self>,
		mut accept: AcceptConnection,
		mut shutdown: ShutdownFuture,
	) -> HttpFuture<'static, ()> {
		let listener = self.0;
		Box::pin(async move {
			loop {
				crate::select_biased!("stop accepting at shutdown",
					_ = &mut shutdown => break,
					accepted = listener.accept() => match accepted {
						Ok((stream, remote)) => accept(Connection::tcp(stream), remote),
						Err(error) => {
							tracing::debug!(?error, "HTTP listener accept failed");
							tokio::time::sleep(Duration::from_secs(1)).await;
						}
					},
				);
			}
			Ok(())
		})
	}
}

/// Remote stream resets the peer server tolerates before the stream is accepted.
///
/// When a UPS caller's deadline passes, it drops the request and the client cancels the
/// stream with RST_STREAM. Under load the server often has not accepted that stream
/// yet. hyper's default limit of 20 such resets guards public servers against rapid
/// reset attacks, but here it closed the whole peer connection with ENHANCE_YOUR_CALM.
/// Every request multiplexed on that connection then failed at once, so one burst of
/// deadlines between two workers failed all their other traffic. Peers are trusted
/// workers, so the limit only needs to stop a runaway peer: 65,536 is above the
/// streams one connection carries during a burst of actor starts.
const PEER_MAX_PENDING_ACCEPT_RESET_STREAMS: usize = 65_536;

/// Stream resets the peer server may send for peer errors on one connection.
///
/// When this limit is reached, h2 also closes the connection with ENHANCE_YOUR_CALM. It
/// uses the same value as the pending accept limit, for the same reason.
const PEER_MAX_LOCAL_ERROR_RESET_STREAMS: usize = 65_536;

/// Receive window of one peer HTTP/2 connection, for both the server and the client.
///
/// h2 closes a connection with ENHANCE_YOUR_CALM (`too_many_data_frames`) once the small
/// DATA frames buffered but not yet read across all its streams exceed half this window.
/// Each frame under 256 bytes costs `256 - len` of that budget, so it is reached long
/// before flow control would pause the sender. UPS stream packets are small, and one
/// connection carries every stream between two workers, including thousands of held
/// gateway tunnels whose readers lag during a burst of actor starts. With the 1 MiB server
/// default, about 3,000 unread packets closed the connection and failed every stream on
/// it. This window allows about 100,000 unread small packets per connection, while UPS
/// stream and class caps still bound what is buffered.
const PEER_CONNECTION_WINDOW_BYTES: u32 = 32 * 1024 * 1024;

/// Receive window of one stream on the peer server.
///
/// A peer channel carries every unary request from one caller over a single stream, so with
/// hyper's 1 MiB default a large request crawls at about 1 MiB per round trip and holds the
/// requests queued behind it. This matches the default stream window callers use for replies.
/// The connection window above still bounds what one connection buffers across its streams.
const PEER_STREAM_WINDOW_BYTES: u32 = 4 * 1024 * 1024;

async fn serve(
	listener: tokio::net::TcpListener,
	router: axum::Router,
	mut shutdown: ShutdownFuture,
	keep_alive_interval: Duration,
	keep_alive_timeout: Duration,
	capacity: usize,
) -> Result<()> {
	use hyper_util::{
		rt::{TokioExecutor, TokioIo, TokioTimer},
		server::{conn::auto::Builder, graceful::GracefulShutdown},
		service::TowerToHyperService,
	};

	let mut builder = Builder::new(TokioExecutor::new());
	// Peers may keep application streams idle forever. Probe from both ends so a
	// partitioned client cannot leave an idle server response alive indefinitely.
	builder
		.http2()
		.timer(TokioTimer::new())
		.keep_alive_interval(keep_alive_interval)
		.keep_alive_timeout(keep_alive_timeout)
		// UPS bounds incoming/outgoing operations and rejects excess work at admission.
		// A second, smaller H2 stream limit silently queues dependency RPCs behind
		// long-lived waits on the same peer connection, preventing those waits from
		// completing. Let application admission see each request instead.
		.max_concurrent_streams(None)
		// Stream reset limits and the connection window keep h2's abuse guards from
		// closing a peer connection that every request between two workers shares.
		.max_pending_accept_reset_streams(PEER_MAX_PENDING_ACCEPT_RESET_STREAMS)
		.max_local_error_reset_streams(PEER_MAX_LOCAL_ERROR_RESET_STREAMS)
		.initial_connection_window_size(PEER_CONNECTION_WINDOW_BYTES)
		.initial_stream_window_size(PEER_STREAM_WINDOW_BYTES);
	let graceful = GracefulShutdown::new();
	let mut connections = tokio::task::JoinSet::new();
	let slots = std::sync::Arc::new(tokio::sync::Semaphore::new(capacity));
	let _capacity = SERVER_CONNECTIONS_CAPACITY.add_guard(capacity as i64);
	let mut warned_full = false;

	loop {
		crate::select_biased!("stop accepting before draining completed connections",
			_ = &mut shutdown => break,
			_ = crate::tokio::when(!connections.is_empty(), connections.join_next()) => {},
			accepted = listener.accept() => {
				let (stream, _) = match accepted {
					Ok(accepted) => accepted,
					Err(error) => {
						tracing::warn!(?error, "HTTP listener accept failed");
						tokio::time::sleep(Duration::from_secs(1)).await;
						continue;
					}
				};
				let slot = match slots.clone().try_acquire_owned() {
					Ok(slot) => {
						warned_full = false;
						slot
					}
					Err(_) => {
						SERVER_CONNECTIONS_REJECTED.inc();
						if !warned_full {
							tracing::warn!(capacity, "HTTP server connection capacity reached");
							warned_full = true;
						}
						continue;
					}
				};
				// Upgraded Envoy sockets carry latency-sensitive bidirectional RPC.
				// Nagle can coalesce requests separated by tens of milliseconds,
				// making independent handlers arrive as a batch at the runtime.
				stream.set_nodelay(true)?;
				let connection = builder.serve_connection_with_upgrades(
					TokioIo::new(stream), TowerToHyperService::new(router.clone())
				).into_owned();
				let connection = graceful.watch(connection);
				let live = SERVER_CONNECTIONS.inc_guard();
				connections.spawn(async move {
					let _slot = slot;
					let _live = live;
					if let Err(error) = connection.await {
						tracing::debug!(?error, "HTTP connection closed");
					}
				});
			},
		);
	}

	drop(listener);
	graceful.shutdown().await;
	while connections.join_next().await.is_some() {}
	Ok(())
}


