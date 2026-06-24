use std::{path::PathBuf, sync::Arc, time::Duration};

use futures_util::future::poll_fn;
use rivet_postgres_util::build_tls_config;
use scc::HashMap;
use tokio::{
	io::{AsyncRead, AsyncWrite},
	sync::{Mutex, broadcast},
};
use tokio_postgres::AsyncMessage;
use tokio_postgres_rustls::MakeRustlsConnect;

/// How long to wait between reconnect attempts for the dedicated LISTEN connection.
const RECONNECT_BACKOFF: Duration = Duration::from_secs(1);
/// Capacity of each channel's broadcast buffer. Notifications are wakeup signals with a polling
/// backstop, so a lagged receiver only delays a wake, never drops a durable commit.
const BROADCAST_CAPACITY: usize = 1024;

struct Subscription {
	tx: broadcast::Sender<String>,
}

/// Owns a single dedicated Postgres connection used exclusively for `LISTEN`. Demultiplexes
/// incoming `NOTIFY` payloads to per-channel broadcast senders and re-`LISTEN`s every registered
/// channel after a reconnect.
///
/// This is separate from the deadpool pool because deadpool recycles connections and drops the
/// async notification stream; LISTEN requires owning the connection's message stream directly.
pub struct PgListener {
	conn_str: String,
	ssl_disabled: bool,
	ssl_root_cert_path: Option<PathBuf>,
	ssl_client_cert_path: Option<PathBuf>,
	ssl_client_key_path: Option<PathBuf>,
	channels: Arc<HashMap<String, Subscription>>,
	client: Arc<Mutex<Option<tokio_postgres::Client>>>,
}

impl PgListener {
	pub fn new(
		conn_str: String,
		ssl_disabled: bool,
		ssl_root_cert_path: Option<PathBuf>,
		ssl_client_cert_path: Option<PathBuf>,
		ssl_client_key_path: Option<PathBuf>,
	) -> Self {
		let channels: Arc<HashMap<String, Subscription>> = Arc::new(HashMap::new());
		let client: Arc<Mutex<Option<tokio_postgres::Client>>> = Arc::new(Mutex::new(None));

		tokio::spawn(Self::connection_lifecycle(
			conn_str.clone(),
			ssl_disabled,
			ssl_root_cert_path.clone(),
			ssl_client_cert_path.clone(),
			ssl_client_key_path.clone(),
			channels.clone(),
			client.clone(),
		));

		Self {
			conn_str,
			ssl_disabled,
			ssl_root_cert_path,
			ssl_client_cert_path,
			ssl_client_key_path,
			channels,
			client,
		}
	}

	/// Subscribe to a channel, registering a `LISTEN` if this is the first subscriber. Returns a
	/// broadcast receiver of notification payloads. Idempotent per channel.
	pub async fn listen(&self, channel: &str) -> broadcast::Receiver<String> {
		match self.channels.entry_async(channel.to_string()).await {
			scc::hash_map::Entry::Occupied(entry) => entry.get().tx.subscribe(),
			scc::hash_map::Entry::Vacant(entry) => {
				let (tx, rx) = broadcast::channel(BROADCAST_CAPACITY);
				entry.insert_entry(Subscription { tx });

				// Best-effort immediate LISTEN; the lifecycle task re-LISTENs on reconnect.
				if let Some(client) = &*self.client.lock().await {
					if let Err(err) = client.execute(&format!("LISTEN \"{channel}\""), &[]).await {
						tracing::warn!(?err, %channel, "failed to LISTEN, will retry on reconnect");
					}
				}

				rx
			}
		}
	}

	async fn connection_lifecycle(
		conn_str: String,
		ssl_disabled: bool,
		ssl_root_cert_path: Option<PathBuf>,
		ssl_client_cert_path: Option<PathBuf>,
		ssl_client_key_path: Option<PathBuf>,
		channels: Arc<HashMap<String, Subscription>>,
		client: Arc<Mutex<Option<tokio_postgres::Client>>>,
	) {
		loop {
			let connected = if ssl_disabled {
				Self::connect_and_run(&conn_str, tokio_postgres::NoTls, &channels, &client).await
			} else {
				match build_tls_config(
					ssl_root_cert_path.as_ref(),
					ssl_client_cert_path.as_ref(),
					ssl_client_key_path.as_ref(),
				) {
					Ok(tls_config) => {
						Self::connect_and_run(
							&conn_str,
							MakeRustlsConnect::new(tls_config),
							&channels,
							&client,
						)
						.await
					}
					Err(err) => {
						tracing::error!(?err, "failed to build listener TLS config");
						false
					}
				}
			};

			if !connected {
				tokio::time::sleep(RECONNECT_BACKOFF).await;
			}
		}
	}

	/// Connects, re-LISTENs all channels, then drives the notification poll loop until the
	/// connection closes. Returns `true` if a connection was successfully established (so the caller
	/// can skip the reconnect backoff).
	async fn connect_and_run<T>(
		conn_str: &str,
		tls: T,
		channels: &Arc<HashMap<String, Subscription>>,
		client: &Arc<Mutex<Option<tokio_postgres::Client>>>,
	) -> bool
	where
		T: tokio_postgres::tls::MakeTlsConnect<tokio_postgres::Socket>,
		T::Stream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
		T::TlsConnect: Send,
		<T::TlsConnect as tokio_postgres::tls::TlsConnect<tokio_postgres::Socket>>::Future: Send,
	{
		let (new_client, connection) = match tokio_postgres::connect(conn_str, tls).await {
			Ok(pair) => pair,
			Err(err) => {
				tracing::error!(?err, "failed to connect postgres listener");
				return false;
			}
		};

		let channels_poll = channels.clone();
		let poll_handle =
			tokio::spawn(async move { Self::poll_connection(connection, channels_poll).await });

		// Re-LISTEN all registered channels on the fresh connection.
		let mut registered = Vec::new();
		channels
			.iter_async(|k, _| {
				registered.push(k.clone());
				true
			})
			.await;
		for channel in &registered {
			if let Err(err) = new_client
				.execute(&format!("LISTEN \"{channel}\""), &[])
				.await
			{
				tracing::error!(?err, %channel, "failed to re-LISTEN channel after reconnect");
			}
		}

		*client.lock().await = Some(new_client);

		// Block until the poll loop ends (connection closed or errored).
		let _ = poll_handle.await;

		*client.lock().await = None;

		true
	}

	async fn poll_connection<S, T>(
		mut connection: tokio_postgres::Connection<S, T>,
		channels: Arc<HashMap<String, Subscription>>,
	) where
		S: AsyncRead + AsyncWrite + Unpin,
		T: AsyncRead + AsyncWrite + Unpin,
	{
		loop {
			match poll_fn(|cx| connection.poll_message(cx)).await {
				Some(Ok(AsyncMessage::Notification(note))) => {
					if let Some(sub) = channels.get_async(note.channel()).await {
						// Ignore send errors: no active receiver just means no one is waiting
						// right now; the polling backstop covers them.
						let _ = sub.tx.send(note.payload().to_string());
					}
				}
				Some(Ok(_)) => {}
				Some(Err(err)) => {
					tracing::warn!(?err, "postgres listener connection error");
					break;
				}
				None => {
					tracing::warn!("postgres listener connection closed");
					break;
				}
			}
		}
	}
}

impl Clone for PgListener {
	fn clone(&self) -> Self {
		Self {
			conn_str: self.conn_str.clone(),
			ssl_disabled: self.ssl_disabled,
			ssl_root_cert_path: self.ssl_root_cert_path.clone(),
			ssl_client_cert_path: self.ssl_client_cert_path.clone(),
			ssl_client_key_path: self.ssl_client_key_path.clone(),
			channels: self.channels.clone(),
			client: self.client.clone(),
		}
	}
}
