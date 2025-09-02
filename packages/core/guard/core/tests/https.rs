// TODO: We need to auto-generate cerificates for tests, since the provided ones have expired
// mod common;
//
// use anyhow::*;
// use futures::future::BoxFuture;
// use http_body_util::Full;
// use hyper::body::Bytes;
// use hyper::{Response, StatusCode};
// use rivet_guard_core::proxy_service::{RouteConfig, RoutingOutput};
// use rivet_util::Id;
// use rustls::crypto::ring::sign::any_supported_type;
// use rustls::sign::CertifiedKey;
// use rustls_pemfile::{certs, private_key};
// use std::{fs::File, io::BufReader, net::SocketAddr, sync::Arc, time::Duration};
// use uuid::Uuid;
//
// use common::{create_test_config, create_test_middleware_fn, init_tracing, TestServer};
// use rivet_guard_core::{
// 	proxy_service::{MiddlewareResponse, PortType, RouteTarget, RoutingTimeout},
// 	CertResolverFn,
// };
//
// // Helper function to load certificates from test fixtures
// fn load_certified_key(
// 	cert_path: &std::path::Path,
// 	key_path: &std::path::Path,
// ) -> Result<CertifiedKey> {
// 	let cert_file = File::open(cert_path)?;
// 	let cert_reader = &mut BufReader::new(cert_file);
//
// 	let cert_chain = certs(cert_reader).collect::<Result<Vec<_>, _>>()?;
//
// 	if cert_chain.is_empty() {
// 		bail!("No certificate found");
// 	}
//
// 	let key_file = File::open(key_path)?;
// 	let key_reader = &mut BufReader::new(key_file);
//
// 	let key_der = private_key(key_reader)?.context("No private key found")?;
//
// 	let signing_key =
// 		any_supported_type(&key_der).map_err(|e| anyhow!("Failed to load private key: {}", e))?;
//
// 	Ok(CertifiedKey::new(cert_chain, signing_key))
// }
//
// // Find an available port by binding to port 0
// async fn find_available_port() -> u16 {
// 	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
// 	listener.local_addr().unwrap().port()
// }
//
// // Modified start_guard function that supports HTTPS and cert resolver
// async fn start_guard_with_https(
// 	mut config: rivet_config::Config,
// 	routing_fn: Arc<
// 		dyn for<'a> Fn(&'a str, &'a str, PortType) -> BoxFuture<'a, Result<RoutingOutput>>
// 			+ Send
// 			+ Sync,
// 	>,
// 	middleware_fn: Arc<
// 		dyn for<'a> Fn(&'a Id) -> BoxFuture<'a, Result<MiddlewareResponse>> + Send + Sync,
// 	>,
// 	cert_resolver_fn: Option<CertResolverFn>,
// ) -> (String, tokio::sync::oneshot::Sender<()>) {
// 	// Find available ports for HTTP and HTTPS
// 	let http_port = find_available_port().await;
// 	let https_port = find_available_port().await;
//
// 	// Recreate config with dynamic ports
// 	let new_config = create_test_config(|guard_config| {
// 		guard_config.http_port = http_port;
//
// 		// Copy the TLS config from the original config
// 		if let Result::Ok(original_guard) = config.guard() {
// 			if let Some(original_https) = &original_guard.https {
// 				let mut https = original_https.clone();
// 				https.port = https_port;
// 				guard_config.https = Some(https);
// 			}
// 		}
// 	});
//
// 	config = new_config;
//
// 	let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
//
// 	// Construct the addresses now since we already know the ports
// 	let http_addr = format!("127.0.0.1:{}", http_port);
// 	let https_addr = format!("127.0.0.1:{}", https_port);
// 	let addrs = format!("{},{}", http_addr, https_addr);
//
// 	// Start the server
// 	tokio::spawn(async move {
// 		let server_task = tokio::spawn(async move {
// 			let result = rivet_guard_core::run_server(
// 				config,
// 				routing_fn,
// 				middleware_fn,
// 				cert_resolver_fn,
// 				None,
// 			)
// 			.await;
//
// 			if let Err(e) = result {
// 				tracing::error!("Guard server error: {}", e);
// 			}
// 		});
//
// 		// Give the server a moment to start
// 		tokio::time::sleep(Duration::from_millis(100)).await;
//
// 		// Wait for shutdown signal
// 		let _ = shutdown_rx.await;
// 		server_task.abort();
// 	});
//
// 	(addrs, shutdown_tx)
// }
//
// // Test that verifies the guard server can start with HTTPS and TLS support
// #[tokio::test]
// async fn test_https_with_tls() {
// 	init_tracing();
//
// 	// Initialize with a default CryptoProvider
// 	let provider = rustls::crypto::ring::default_provider();
// 	provider
// 		.install_default()
// 		.expect("Failed to install crypto provider");
//
// 	// Set up a test server for handling backend requests
// 	let test_server = TestServer::with_handler(|req, _request_log| {
// 		// Return a simple 200 OK response for any request
// 		let response = Response::builder()
// 			.status(StatusCode::OK)
// 			.body(Full::new(Bytes::from("OK")))
// 			.unwrap();
//
// 		Box::pin(async move { Result::<_, std::convert::Infallible>::Ok(response) })
// 	})
// 	.await;
// 	let test_server_addr = test_server.addr;
//
// 	// Set up paths to TLS certificates
// 	let fixtures_abs_path =
// 		std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tls");
// 	assert!(
// 		fixtures_abs_path.exists(),
// 		"Fixtures directory doesn't exist: {}",
// 		fixtures_abs_path.display()
// 	);
//
// 	let api_cert_path = fixtures_abs_path.join("api_cert.pem");
// 	let api_key_path = fixtures_abs_path.join("api_key.pem");
//
// 	// Load API certificate
// 	let api_cert = match load_certified_key(&api_cert_path, &api_key_path) {
// 		Result::Ok(cert) => Arc::new(cert),
// 		Err(e) => {
// 			panic!("Failed to load API certificate: {}", e);
// 		}
// 	};
//
// 	// Load job certificate
// 	let job_cert_path = fixtures_abs_path.join("job_cert.pem");
// 	let job_key_path = fixtures_abs_path.join("job_key.pem");
//
// 	let job_cert = match load_certified_key(&job_cert_path, &job_key_path) {
// 		Result::Ok(cert) => Arc::new(cert),
// 		Err(e) => {
// 			panic!("Failed to load job certificate: {}", e);
// 		}
// 	};
//
// 	// Create a cert resolver function that chooses certificate based on hostname
// 	let api_cert_for_resolver = api_cert.clone();
// 	let job_cert_for_resolver = job_cert.clone();
// 	let cert_resolver_fn: rivet_guard_core::CertResolverFn = Arc::new(
// 		move |hostname: &str| -> std::result::Result<
// 			Arc<CertifiedKey>,
// 			Box<dyn std::error::Error + Send + Sync>,
// 		> {
// 			let api_cert = api_cert_for_resolver.clone();
// 			let job_cert = job_cert_for_resolver.clone();
//
// 			// Select certificate based on domain
// 			if hostname.ends_with("microgravity.io") {
// 				Result::Ok(job_cert)
// 			} else if hostname.ends_with("gameinc.io") {
// 				Result::Ok(api_cert)
// 			} else {
// 				Result::Err(anyhow!("No certificate found for hostname: {}", hostname).into())
// 			}
// 		},
// 	);
//
// 	// Create routing function that routes to our test server
// 	let routing_fn: rivet_guard_core::proxy_service::RoutingFn =
// 		Arc::new(move |hostname: &str, _path: &str, _port_type: PortType| {
// 			let test_server_addr = test_server_addr.clone();
//
// 			Box::pin(async move {
// 				// Extract a UUID from the hostname if it contains one
// 				let actor_id = if hostname.contains("actor") {
// 					if hostname.contains("54c60ee9-ae31-4a3f-b5ff-06aa5639310f") {
// 						Uuid::parse_str("54c60ee9-ae31-4a3f-b5ff-06aa5639310f").unwrap()
// 					} else {
// 						Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap()
// 					}
// 				} else {
// 					Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap()
// 				};
//
// 				// Create route target pointing to our test server
// 				let route_target = RouteTarget {
// 					actor_id: Some(Id::v1(actor_id, 0)),
// 					server_id: Some(
// 						Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap(),
// 					),
// 					host: test_server_addr.ip().to_string(),
// 					port: test_server_addr.port(),
// 					path: "/test-http".to_string(),
// 				};
//
// 				Ok(RoutingOutput::Route(RouteConfig {
// 					targets: vec![route_target],
// 					timeout: RoutingTimeout { routing_timeout: 5 },
// 				}))
// 			})
// 		});
//
// 	// Create a middleware function
// 	let middleware_fn = create_test_middleware_fn(|_config| {});
//
// 	// Configure Guard with HTTPS enabled
// 	let config = create_test_config(|guard_config| {
// 		// Use port 0 to let the OS choose an available port
// 		guard_config.http_port = 0;
//
// 		// Enable HTTPS with port 0 (OS-assigned)
// 		guard_config.https = Some(rivet_config::config::guard::Https {
// 			port: 0,
// 			tls: rivet_config::config::guard::Tls {
// 				api_cert_path: api_cert_path.clone(),
// 				api_key_path: api_key_path.clone(),
// 				actor_cert_path: job_cert_path.clone(),
// 				actor_key_path: job_key_path.clone(),
// 			},
// 		});
// 	});
//
// 	// Start the guard server with HTTPS and TLS support
// 	let (addrs, _shutdown) =
// 		start_guard_with_https(config, routing_fn, middleware_fn, Some(cert_resolver_fn)).await;
//
// 	// Parse HTTP and HTTPS ports
// 	let mut parts = addrs.split(',');
// 	let http_addr = parts.next().unwrap();
// 	let https_addr = parts.next().unwrap();
//
// 	// Check that we got valid addresses
// 	assert!(
// 		http_addr.parse::<SocketAddr>().is_ok(),
// 		"Invalid HTTP address: {}",
// 		http_addr
// 	);
// 	assert!(
// 		https_addr.parse::<SocketAddr>().is_ok(),
// 		"Invalid HTTPS address: {}",
// 		https_addr
// 	);
//
// 	// Give the server a moment to fully initialize
// 	tokio::time::sleep(Duration::from_millis(50)).await;
//
// 	// Test 1: HTTPS with microgravity.io domain (should use job cert)
// 	let test_host = "foobar.actor.54c60ee9-ae31-4a3f-b5ff-06aa5639310f.staging2.microgravity.io";
// 	let https_port = https_addr.split(':').nth(1).unwrap();
//
// 	// First, test the TLS connection with openssl s_client
// 	let tls_status = tokio::process::Command::new("sh")
// 		.arg("-c")
// 		.arg(format!(
// 			"echo Q | openssl s_client -connect 127.0.0.1:{} -servername {} -brief",
// 			https_port, test_host
// 		))
// 		.status()
// 		.await
// 		.expect("Failed to execute openssl command");
//
// 	assert!(
// 		tls_status.success(),
// 		"Expected openssl command to exit with code 0, got: {:?}",
// 		tls_status
// 	);
//
// 	// Use curl to make the HTTPS request
// 	let status = tokio::process::Command::new("curl")
// 		.args([
// 			"-v",
// 			"-L",
// 			"--fail",
// 			"--max-time",
// 			"5",
// 			"--connect-timeout",
// 			"5",
// 			"--resolve",
// 			&format!("{}:{}:127.0.0.1", test_host, https_port),
// 			&format!("https://{}:{}/test-http", test_host, https_port),
// 		])
// 		.status()
// 		.await
// 		.expect("Failed to execute curl command");
//
// 	assert!(
// 		status.success(),
// 		"Expected curl command to exit with code 0, got: {:?}",
// 		status
// 	);
//
// 	// Verify the request reached the test server
// 	assert!(
// 		test_server.request_count() >= 1,
// 		"Expected at least the HTTP request to reach the server"
// 	);
//
// 	let last_request = test_server.last_request().unwrap();
// 	assert_eq!(
// 		last_request.method, "GET",
// 		"Expected GET method for HTTPS request"
// 	);
// 	assert_eq!(
// 		last_request.uri, "/test-http",
// 		"Unexpected URI path for HTTPS request"
// 	);
//
// 	// Test 2: HTTPS with gameinc.io domain (should use API certificate)
// 	let gameinc_host = "api.lnd-atl.staging2.gameinc.io";
//
// 	let gameinc_status = tokio::process::Command::new("curl")
// 		.args([
// 			"--fail",
// 			"-v",
// 			"-L",
// 			"--max-time",
// 			"5",
// 			"--connect-timeout",
// 			"5",
// 			"--resolve",
// 			&format!("{}:{}:127.0.0.1", gameinc_host, https_port),
// 			&format!("https://{}:{}/test-http", gameinc_host, https_port),
// 		])
// 		.status()
// 		.await
// 		.expect("Failed to execute curl command");
//
// 	assert!(
// 		gameinc_status.success(),
// 		"curl command failed for gameinc.io: {:?}",
// 		gameinc_status
// 	);
// 	assert_eq!(
// 		gameinc_status.code(),
// 		Some(0),
// 		"Expected exit code 0 for gameinc.io curl command"
// 	);
//
// 	// Test 3: HTTPS with invalid domain (should fail with cert error)
// 	let invalid_host = "this-host-does-not-exist.nowhere";
//
// 	let invalid_status = tokio::process::Command::new("curl")
// 		.args([
// 			"--fail",
// 			"-v",
// 			"-L",
// 			"--max-time",
// 			"5",
// 			"--connect-timeout",
// 			"5",
// 			"--resolve",
// 			&format!("{}:{}:127.0.0.1", invalid_host, https_port),
// 			&format!("https://{}:{}/test-http", invalid_host, https_port),
// 		])
// 		.status()
// 		.await
// 		.expect("Failed to execute curl command");
//
// 	// This should fail because the hostname doesn't exist (cert resolver won't find a matching certificate)
// 	assert!(
// 		!invalid_status.success(),
// 		"Expected curl to fail with invalid domain"
// 	);
// 	assert_ne!(
// 		invalid_status.code(),
// 		Some(0),
// 		"Expected non-zero exit code for invalid domain curl command"
// 	);
// }
