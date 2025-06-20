use crate::cert_resolver::{create_tls_config, CertResolverFn};
use crate::metrics;
use crate::proxy_service::{MiddlewareFn, ProxyServiceFactory, RoutingFn};
use global_error::*;
use hyper::service::service_fn;
use std::fmt;
use std::net::SocketAddr;
use std::pin::pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::signal::unix::signal;
use tokio::signal::unix::SignalKind;
use tokio_rustls::TlsAcceptor;
use tracing::Instrument;

// HACK: GlobalError does not conform to StdError required by hyper
#[derive(Debug)]
pub struct GlobalErrorWrapper {
	pub err: GlobalError,
}

impl fmt::Display for GlobalErrorWrapper {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.err)
	}
}

impl std::error::Error for GlobalErrorWrapper {}

// Start the server
#[tracing::instrument(skip_all)]
pub async fn run_server(
	config: rivet_config::Config,
	routing_fn: RoutingFn,
	middleware_fn: MiddlewareFn,
	cert_resolver_fn: Option<CertResolverFn>,
	clickhouse_inserter: Option<clickhouse_inserter::ClickHouseInserterHandle>,
) -> GlobalResult<()> {
	// Configure servers for different ports
	let guard_config = config.guard()?;

	// Set up HTTP server
	let http_addr: std::net::SocketAddr = ([0, 0, 0, 0], guard_config.http_port).into();
	let http_factory = Arc::new(ProxyServiceFactory::new(
		config.clone(),
		routing_fn.clone(),
		middleware_fn.clone(),
		crate::proxy_service::PortType::Http,
		clickhouse_inserter.clone(),
	));
	let http_listener = tokio::net::TcpListener::bind(http_addr).await?;

	// Set up HTTPS server (if configured)
	let (https_addr, https_factory, https_listener, https_acceptor) = if let Some(https) =
		&guard_config.https
	{
		let https_addr: std::net::SocketAddr = ([0, 0, 0, 0], https.port).into();
		let https_factory = Arc::new(ProxyServiceFactory::new(
			config.clone(),
			routing_fn.clone(),
			middleware_fn.clone(),
			crate::proxy_service::PortType::Https,
			clickhouse_inserter.clone(),
		));
		let listener = tokio::net::TcpListener::bind(https_addr).await?;

		// Configure TLS if resolver function is provided
		let acceptor = if let Some(resolver_fn) = cert_resolver_fn {
			// Create a TLS server config using our certificate resolver
			let server_config = create_tls_config(resolver_fn);

			Some(TlsAcceptor::from(Arc::new(server_config)))
		} else {
			tracing::warn!("No TLS certificate resolver provided, HTTPS will not work properly");
			None
		};

		(
			Some(https_addr),
			Some(https_factory),
			Some(listener),
			acceptor,
		)
	} else {
		(None, None, None, None)
	};

	// Set up server builder and graceful shutdown
	let server = hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
	let graceful = hyper_util::server::graceful::GracefulShutdown::new();

	// Set up signal handling for graceful shutdown
	let mut sigterm = signal(SignalKind::terminate())?;
	let mut sigterm = pin!(sigterm.recv());
	let mut sigint = signal(SignalKind::interrupt())?;
	let mut sigint = pin!(sigint.recv());

	tracing::info!("HTTP server listening on {}", http_addr);
	if let Some(addr) = &https_addr {
		tracing::info!("HTTPS server listening on {}", addr);
	}

	// Helper function to process regular connections
	#[tracing::instrument(skip_all, fields(?remote_addr))]
	fn process_connection(
		tcp_stream: tokio::net::TcpStream,
		remote_addr: SocketAddr,
		factory_clone: Arc<ProxyServiceFactory>,
		server: &hyper_util::server::conn::auto::Builder<hyper_util::rt::TokioExecutor>,
		graceful: &hyper_util::server::graceful::GracefulShutdown,
		port_type_str: String,
	) {
		let connection_start = Instant::now();
		metrics::TCP_CONNECTION_PENDING.inc();
		metrics::TCP_CONNECTION_TOTAL.inc();

		let io = hyper_util::rt::TokioIo::new(tcp_stream);

		// Create a proxy service instance for this connection
		let proxy_service = factory_clone.create_service(remote_addr);

		// Using service_fn to convert our function into a hyper service
		let service = service_fn(move |req| {
			let service_clone = proxy_service.clone();
			async move {
				service_clone
					.process(req)
					.await
					.map_err(|err| GlobalErrorWrapper { err })
			}
		});

		// Serve the connection with graceful shutdown support
		let conn = server.serve_connection_with_upgrades(io, service);
		let conn = graceful.watch(conn.into_owned());

		tokio::spawn(
			async move {
				if let Err(err) = conn.await {
					tracing::error!("{} connection error: {}", port_type_str, err);
				}
				tracing::debug!("{} connection dropped: {}", port_type_str, remote_addr);

				let connection_duration = connection_start.elapsed().as_secs_f64();
				metrics::TCP_CONNECTION_DURATION.observe(connection_duration);
				metrics::TCP_CONNECTION_PENDING.dec();
			}
			.instrument(tracing::info_span!(parent: None, "process_connection_task")),
		);
	}

	// Accept connections until we receive a shutdown signal
	loop {
		let result: Result<(), GlobalError> = tokio::select! {
			conn = http_listener.accept() => {
				match conn {
					Ok((tcp_stream, remote_addr)) => {
						process_connection(
							tcp_stream,
							remote_addr,
							http_factory.clone(),
							&server,
							&graceful,
							"HTTP".to_string()
						);
					},
					Err(err) => {
						tracing::debug!(?err, "Accept error on HTTP port");
						tokio::time::sleep(Duration::from_secs(1)).await;
					}
				}
				Ok(())
			},
			conn = async {
				match &https_listener {
					Some(listener) => Some(listener.accept().await),
					None => {
						// If HTTPS is not configured, this future never returns
						std::future::pending::<Option<_>>().await
					}
				}
			} => {
				if let Some(conn) = conn {
					match conn {
						Ok((tcp_stream, remote_addr)) => {
							if let Some(factory) = &https_factory {
								// Check if we have a TLS acceptor
								if let Some(acceptor) = &https_acceptor {
									// Handle TLS connection
									let https_factory_clone = factory.clone();
									let acceptor_clone = acceptor.clone();

									// Accept TLS connection in a separate task to avoid ownership issues
									tokio::spawn(async move {
										let connection_start = Instant::now();
										metrics::TCP_CONNECTION_PENDING.inc();
										metrics::TCP_CONNECTION_TOTAL.inc();

										match acceptor_clone
											.accept(tcp_stream)
											.instrument(tracing::info_span!("accept"))
											.await
										{
											Ok(tls_stream) => {
												tracing::debug!("TLS handshake successful for {}", remote_addr);

												// Create service for this connection
												let io = hyper_util::rt::TokioIo::new(tls_stream);
												let proxy_service = https_factory_clone.create_service(remote_addr);

												// Using service_fn to convert our function into a hyper service
												let service = service_fn(move |req| {
													let service_clone = proxy_service.clone();

													async move {
														service_clone.process(req).await.map_err(|err| GlobalErrorWrapper{err})
													}
												});

												// Create a new server for each connection
												let conn_server = hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());

												// Serve the connection (no graceful shutdown in spawned task)
												if let Err(err) = conn_server.serve_connection_with_upgrades(io, service).await {
													tracing::debug!(?err, "HTTPS connection error");
												}

												tracing::debug!("HTTPS connection dropped: {}", remote_addr);
											},
											Err(err) => {
												tracing::debug!(?err, "TLS handshake failed for {}", remote_addr);
											}
										}

										let connection_duration = connection_start.elapsed().as_secs_f64();
										metrics::TCP_CONNECTION_DURATION.observe(connection_duration);
										metrics::TCP_CONNECTION_PENDING.dec();
									}.instrument(tracing::info_span!(parent: None, "process_tls_connection_task")));
								} else {
									// Fallback to non-TLS handling (useful for testing)
									// In production, this would not secure the connection
									tracing::warn!("HTTPS port configured but no TLS acceptor available");
									process_connection(
										tcp_stream,
										remote_addr,
										factory.clone(),
										&server,
										&graceful,
										"HTTPS (unsecured)".to_string()
									);
								}
							}
						},
						Err(err) => {
							tracing::debug!(?err, "Accept error on HTTPS port");
							tokio::time::sleep(Duration::from_secs(1)).await;
						}
					}
				}
				Ok(())
			},

			_ = sigterm.as_mut() => {
				tracing::info!("SIGTERM received, starting shutdown");
				break;
			},

			_ = sigint.as_mut() => {
				tracing::info!("SIGINT (Ctrl-C) received, starting shutdown");
				break;
			}
		};

		if let Err(err) = result {
			tracing::error!(?err, "Error in server loop");
		}
	}

	// Start graceful shutdown with timeout
	tokio::select! {
		_ = graceful.shutdown() => {
			tracing::info!("Gracefully shutdown completed");
		},
		_ = tokio::time::sleep(Duration::from_secs(30)) => {
			tracing::error!("Waited 30 seconds for graceful shutdown, aborting...");
		}
	}

	Ok(())
}
