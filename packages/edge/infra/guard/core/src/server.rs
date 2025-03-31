use crate::proxy_service::{ProxyServiceFactory, RoutingFn, MiddlewareFn};
use global_error::*;
use hyper::service::service_fn;
use std::fmt;
use std::net::SocketAddr;
use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal::unix::signal;
use tokio::signal::unix::SignalKind;
use tracing::{error, info};

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
pub async fn run_server(config: rivet_config::Config, routing_fn: RoutingFn, middleware_fn: MiddlewareFn) -> GlobalResult<()> {
	// Configure servers for different ports
	let guard_config = config.guard()?;
	
		
	// Set up HTTP server
	let http_addr: std::net::SocketAddr = ([0, 0, 0, 0], guard_config.http_port).into();
	let http_factory = Arc::new(ProxyServiceFactory::new(
		config.clone(), 
		routing_fn.clone(), 
		middleware_fn.clone(),
		crate::proxy_service::PortType::Http
	));
	let http_listener = tokio::net::TcpListener::bind(http_addr).await?;
	
	// Set up HTTPS server (if configured)
	let (https_addr, https_factory, https_listener) = if let Some(https_config) = &guard_config.https {
		let https_addr: std::net::SocketAddr = ([0, 0, 0, 0], https_config.port).into();
		let https_factory = Arc::new(ProxyServiceFactory::new(
			config.clone(), 
			routing_fn.clone(), 
			middleware_fn.clone(),
			crate::proxy_service::PortType::Https
		));
		let listener = tokio::net::TcpListener::bind(https_addr).await?;
		
		// TLS configuration would be handled here
		// For now, we're just binding to the port
		
		(Some(https_addr), Some(https_factory), Some(listener))
	} else {
		(None, None, None)
	};

	// Set up server builder and graceful shutdown
	let server = hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
	let graceful = hyper_util::server::graceful::GracefulShutdown::new();

	// Set up signal handling for graceful shutdown
	let mut sigterm = signal(SignalKind::terminate())?;
	let mut sigterm = pin!(sigterm.recv());
	let mut sigint = signal(SignalKind::interrupt())?;
	let mut sigint = pin!(sigint.recv());

		info!("HTTP server listening on {}", http_addr);
	if let Some(addr) = &https_addr {
		info!("HTTPS server listening on {}", addr);
	}

	// Helper function to process connections
	async fn process_connection(
		tcp_stream: tokio::net::TcpStream,
		remote_addr: SocketAddr,
		factory_clone: Arc<ProxyServiceFactory>,
		server: &hyper_util::server::conn::auto::Builder<hyper_util::rt::TokioExecutor>,
		graceful: &hyper_util::server::graceful::GracefulShutdown,
		port_type_str: String,
	) {
		let io = hyper_util::rt::TokioIo::new(tcp_stream);
		
		// Create a proxy service instance for this connection
		let proxy_service = factory_clone.create_service(remote_addr);
		
		// Using service_fn to convert our function into a hyper service
		let service = service_fn(move |req| {
			let service_clone = proxy_service.clone();
			async move {
				service_clone.process(req).await.map_err(|err| GlobalErrorWrapper{err})
			}
		});

		// Serve the connection with graceful shutdown support
		let conn = server.serve_connection_with_upgrades(io, service);
		let conn = graceful.watch(conn.into_owned());

		tokio::spawn(async move {
			if let Err(err) = conn.await {
				error!("{} connection error: {}", port_type_str, err);
			}
			info!("{} connection dropped: {}", port_type_str, remote_addr);
		});
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
						).await;
					},
					Err(e) => {
						error!("Accept error on HTTP port: {}", e);
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
								process_connection(
									tcp_stream, 
									remote_addr, 
									factory.clone(), 
									&server, 
									&graceful,
									"HTTPS".to_string()
								).await;
							}
						},
						Err(e) => {
							error!("Accept error on HTTPS port: {}", e);
							tokio::time::sleep(Duration::from_secs(1)).await;
						}
					}
				}
				Ok(())
			},

			_ = sigterm.as_mut() => {
				info!("SIGTERM received, starting shutdown");
				break;
			},

			_ = sigint.as_mut() => {
				info!("SIGINT (Ctrl-C) received, starting shutdown");
				break;
			}
		};

		if let Err(e) = result {
			error!("Error in server loop: {}", e);
		}
	}

	// Start graceful shutdown with timeout
	tokio::select! {
		_ = graceful.shutdown() => {
			info!("Gracefully shutdown completed");
		},
		_ = tokio::time::sleep(Duration::from_secs(30)) => {
			error!("Waited 30 seconds for graceful shutdown, aborting...");
		}
	}

	Ok(())
}
