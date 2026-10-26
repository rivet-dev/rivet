//! Injectable upstream `http` requests/responses and Axum listeners. Routing, authentication and
//! destination policy remain with callers; adapters own sockets and DNS.

pub use ::http::{Method, Request, Response, StatusCode};
use anyhow::Result;
use bytes::Bytes;
#[cfg(not(target_arch = "wasm32"))]
use std::net::SocketAddr;
use std::{future::Future, pin::Pin};

pub type HttpFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

pub type HttpBody = futures_util::stream::BoxStream<'static, Result<Bytes>>;

pub type ShutdownFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// The adapter failed to establish a connection, before sending any request bytes.
/// Other HTTP errors have uncertain outcomes and must not carry this marker.
#[derive(Debug)]
pub struct RequestNotSent;

impl std::fmt::Display for RequestNotSent {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("HTTP connection failed before the request was sent")
	}
}
impl std::error::Error for RequestNotSent {}

#[derive(Debug)]
pub struct ResponseTooLarge;

/// Which set of peer connections a streaming request uses. Unary requests and long-lived
/// streams keep separate connections, so a stalled stream cannot hold up requests, and each
/// set spreads its traffic over several connections by `key`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerRoute {
	pub lane: PeerLane,
	/// Chooses the connection within the lane. Requests with the same key share one.
	pub key: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeerLane {
	/// A request with one input and one reply.
	Request,
	/// A stream that may stay open and carry many messages either way.
	Stream,
}

impl PeerLane {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Request => "request",
			Self::Stream => "stream",
		}
	}
}

impl std::fmt::Display for ResponseTooLarge {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("HTTP response exceeds configured limit")
	}
}

impl std::error::Error for ResponseTooLarge {}

pub trait HttpClient: Send + Sync {
	/// Full-duplex HTTP/2. Returns after response headers without collecting either
	/// body. Callers own frame limits, deadlines, and application retry policy.
	fn execute_stream(&self, request: Request<HttpBody>) -> HttpFuture<'_, Response<HttpBody>>;

	/// Cancellation stops local I/O, but may leave remote work running.
	fn execute(
		&self,
		request: Request<Bytes>,
		max_response_bytes: usize,
	) -> HttpFuture<'_, Response<Bytes>>;
}

#[cfg(not(target_arch = "wasm32"))]
pub trait HttpServer: Send + Sync {
	fn bind(&self, address: SocketAddr) -> HttpFuture<'_, Box<dyn HttpListener>>;
}

#[cfg(not(target_arch = "wasm32"))]
pub trait HttpListener: Send {
	fn local_addr(&self) -> Result<SocketAddr>;

	fn serve(
		self: Box<Self>,
		router: axum::Router,
		shutdown: ShutdownFuture,
	) -> HttpFuture<'static, ()>;

	/// Hands each accepted connection and its remote address to `accept`, for a server that
	/// runs its own connection loop, such as guard. The listener owns accepting and stops at
	/// `shutdown`; `accept` owns serving and draining what it was handed. `accept` is called
	/// once per connection, so a request never crosses a dynamic call here.
	fn serve_connections(
		self: Box<Self>,
		accept: AcceptConnection,
		shutdown: ShutdownFuture,
	) -> HttpFuture<'static, ()>;
}

/// Receives each connection a listener accepts. It must not block: spawn the serving.
#[cfg(not(target_arch = "wasm32"))]
pub type AcceptConnection = Box<dyn FnMut(Connection, SocketAddr) + Send>;

/// One accepted connection. Each read and write matches on the transport an adapter produced,
/// so the production path keeps the concrete socket with no dynamic dispatch.
#[cfg(not(target_arch = "wasm32"))]
pub struct Connection(ConnectionIo);

#[cfg(not(target_arch = "wasm32"))]
#[allow(
	clippy::disallowed_types,
	reason = "the production adapter owns the accepted socket"
)]
enum ConnectionIo {
	Tcp(tokio::net::TcpStream),
	Memory(tokio::io::DuplexStream),
}

#[cfg(not(target_arch = "wasm32"))]
impl Connection {
	#[allow(
		clippy::disallowed_types,
		reason = "the production adapter owns the accepted socket"
	)]
	pub(crate) fn tcp(stream: tokio::net::TcpStream) -> Self {
		Self(ConnectionIo::Tcp(stream))
	}

	/// An in-memory connection for adapters that serve without a socket.
	#[allow(
		dead_code,
		reason = "only the simulation backend builds memory connections"
	)]
	pub fn memory(stream: tokio::io::DuplexStream) -> Self {
		Self(ConnectionIo::Memory(stream))
	}

	/// Disables Nagle on a socket. In-memory connections have none to disable.
	pub fn set_nodelay(&self, nodelay: bool) -> std::io::Result<()> {
		match &self.0 {
			ConnectionIo::Tcp(stream) => stream.set_nodelay(nodelay),
			ConnectionIo::Memory(_) => Ok(()),
		}
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl tokio::io::AsyncRead for Connection {
	fn poll_read(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		buf: &mut tokio::io::ReadBuf<'_>,
	) -> std::task::Poll<std::io::Result<()>> {
		match &mut self.get_mut().0 {
			ConnectionIo::Tcp(stream) => Pin::new(stream).poll_read(cx, buf),
			ConnectionIo::Memory(stream) => Pin::new(stream).poll_read(cx, buf),
		}
	}
}

#[cfg(not(target_arch = "wasm32"))]
impl tokio::io::AsyncWrite for Connection {
	fn poll_write(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		buf: &[u8],
	) -> std::task::Poll<std::io::Result<usize>> {
		match &mut self.get_mut().0 {
			ConnectionIo::Tcp(stream) => Pin::new(stream).poll_write(cx, buf),
			ConnectionIo::Memory(stream) => Pin::new(stream).poll_write(cx, buf),
		}
	}

	fn poll_write_vectored(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		bufs: &[std::io::IoSlice<'_>],
	) -> std::task::Poll<std::io::Result<usize>> {
		match &mut self.get_mut().0 {
			ConnectionIo::Tcp(stream) => Pin::new(stream).poll_write_vectored(cx, bufs),
			ConnectionIo::Memory(stream) => Pin::new(stream).poll_write_vectored(cx, bufs),
		}
	}

	fn is_write_vectored(&self) -> bool {
		match &self.0 {
			ConnectionIo::Tcp(stream) => stream.is_write_vectored(),
			ConnectionIo::Memory(stream) => stream.is_write_vectored(),
		}
	}

	fn poll_flush(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<std::io::Result<()>> {
		match &mut self.get_mut().0 {
			ConnectionIo::Tcp(stream) => Pin::new(stream).poll_flush(cx),
			ConnectionIo::Memory(stream) => Pin::new(stream).poll_flush(cx),
		}
	}

	fn poll_shutdown(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<std::io::Result<()>> {
		match &mut self.get_mut().0 {
			ConnectionIo::Tcp(stream) => Pin::new(stream).poll_shutdown(cx),
			ConnectionIo::Memory(stream) => Pin::new(stream).poll_shutdown(cx),
		}
	}
}
