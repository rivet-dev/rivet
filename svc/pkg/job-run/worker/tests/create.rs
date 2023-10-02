use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use std::{
	convert::TryInto,
	io::{Read, Write},
	net::{TcpStream, UdpSocket},
	sync::Arc,
};

#[worker_test]
async fn basic_http(ctx: TestCtx) {
	let domain_job = util::env::domain_job().unwrap();

	let region_res = op!([ctx] faker_region {}).await.unwrap();
	let region_id = region_res.region_id.as_ref().unwrap().as_uuid();
	let region_name_id = &region_res.region.as_ref().unwrap().name_id;

	let template_res = op!([ctx] faker_job_template {
		kind: Some(faker::job_template::request::Kind::EchoServer(Default::default())),
		..Default::default()
	})
	.await
	.unwrap();

	// Run the job
	let run_id = Uuid::new_v4();
	let test_id = Uuid::new_v4().to_string();
	let mut planned_sub = subscribe!([ctx] job_run::msg::alloc_planned(run_id))
		.await
		.unwrap();
	let mut fail_sub = subscribe!([ctx] job_run::msg::fail(run_id)).await.unwrap();
	let mut eval_sub = subscribe!([ctx] job_run::msg::eval_complete(run_id))
		.await
		.unwrap();
	let ingress_hostname_http = format!("test-{run_id}-http.lobby.{region_name_id}.{domain_job}");
	let ingress_hostname_https = format!("test-{run_id}-https.lobby.{region_name_id}.{domain_job}");
	msg!([ctx] job_run::msg::create(run_id) {
		run_id: Some(run_id.into()),
		region_id: Some(region_id.into()),
		parameters: vec![
			job_run::msg::create::Parameter {
				key: "test_id".into(),
				value: test_id.clone(),
			},
		],
		job_spec_json: template_res.job_spec_json.clone(),
		proxied_ports: vec![
			job_run::msg::create::ProxiedPort {
				#[allow(deprecated)]
				target_nomad_port_label: Some("http".into()),
				ingress_port: None,
				ingress_hostnames: vec![ingress_hostname_http.clone()],
				proxy_protocol: backend::job::ProxyProtocol::Http as i32,
				ssl_domain_mode: backend::job::SslDomainMode::Exact as i32,
			},
			job_run::msg::create::ProxiedPort {
				#[allow(deprecated)]
				target_nomad_port_label: Some("http".into()),
				ingress_port: None,
				ingress_hostnames: vec![ingress_hostname_https.clone()],
				proxy_protocol: backend::job::ProxyProtocol::Https as i32,
				ssl_domain_mode: backend::job::SslDomainMode::Exact as i32,
			},
		],
		..Default::default()
	})
	.await
	.unwrap();
	tokio::select! {
		_ = async {
			planned_sub.next().await.unwrap();
			eval_sub.next().await.unwrap();
		} => {
			tracing::info!("started");
		}
		res = fail_sub.next() => {
			let msg = res.unwrap();
			tracing::error!(?msg, "run failed");
			panic!("run failed");
		}
	}

	let ValidateJobOutput { ip, port, .. } =
		validate_job(ctx.crdb().await.unwrap(), run_id, region_id, "http").await;

	// Test against origin
	compare_test_id_http(&format!("http://{}:{}", ip, port), &test_id).await;

	// Test via proxy with HTTP
	compare_test_id_http(&format!("http://{ingress_hostname_http}"), &test_id).await;

	// Test via proxy with HTTPS
	compare_test_id_http(&format!("https://{ingress_hostname_https}"), &test_id).await;
}

#[worker_test]
async fn basic_tcp(ctx: TestCtx) {
	let domain_job = util::env::domain_job().unwrap();

	let region_res = op!([ctx] faker_region {}).await.unwrap();
	let region_id = region_res.region_id.as_ref().unwrap().as_uuid();
	let region_name_id = &region_res.region.as_ref().unwrap().name_id;

	let template_res = op!([ctx] faker_job_template {
		kind: Some(faker::job_template::request::Kind::EchoServerTcp(Default::default())),
		..Default::default()
	})
	.await
	.unwrap();

	// Run the job
	let run_id = Uuid::new_v4();
	let test_id = Uuid::new_v4().to_string();
	let mut planned_sub = subscribe!([ctx] job_run::msg::alloc_planned(run_id))
		.await
		.unwrap();
	let mut fail_sub = subscribe!([ctx] job_run::msg::fail(run_id)).await.unwrap();
	let mut eval_sub = subscribe!([ctx] job_run::msg::eval_complete(run_id))
		.await
		.unwrap();
	let ingress_hostname_tcp = format!("test-{run_id}-tcp.lobby.{region_name_id}.{domain_job}");
	let ingress_hostname_tcp_tls =
		format!("test-{run_id}-tcp-tls.lobby.{region_name_id}.{domain_job}");
	msg!([ctx] job_run::msg::create(run_id) {
		run_id: Some(run_id.into()),
		region_id: Some(region_id.into()),
		parameters: vec![
			job_run::msg::create::Parameter {
				key: "test_id".into(),
				value: test_id.clone(),
			},
		],
		job_spec_json: template_res.job_spec_json.clone(),
		proxied_ports: vec![
			job_run::msg::create::ProxiedPort {
				#[allow(deprecated)]
				target_nomad_port_label: Some("tcp".into()),
				ingress_port: None,
				ingress_hostnames: vec![ingress_hostname_tcp.clone()],
				proxy_protocol: backend::job::ProxyProtocol::Tcp as i32,
				ssl_domain_mode: backend::job::SslDomainMode::Exact as i32,
			},
			job_run::msg::create::ProxiedPort {
				#[allow(deprecated)]
				target_nomad_port_label: Some("tcp".into()),
				ingress_port: None,
				ingress_hostnames: vec![ingress_hostname_tcp_tls.clone()],
				proxy_protocol: backend::job::ProxyProtocol::TcpTls as i32,
				ssl_domain_mode: backend::job::SslDomainMode::Exact as i32,
			},
		],
		..Default::default()
	})
	.await
	.unwrap();
	tokio::select! {
		_ = async {
			planned_sub.next().await.unwrap();
			eval_sub.next().await.unwrap();
		} => {
			tracing::info!("started");
		}
		res = fail_sub.next() => {
			let msg = res.unwrap();
			tracing::error!(?msg, "run failed");
			panic!("run failed");
		}
	}

	let ValidateJobOutput {
		ip,
		port,
		proxied_ports,
	} = validate_job(ctx.crdb().await.unwrap(), run_id, region_id, "tcp").await;
	let ingress_port_tcp = proxied_ports
		.iter()
		.find(|x| x.proxy_protocol == backend::job::ProxyProtocol::Tcp as i64)
		.unwrap()
		.ingress_port;
	let ingress_port_tcp_tls = proxied_ports
		.iter()
		.find(|x| x.proxy_protocol == backend::job::ProxyProtocol::TcpTls as i64)
		.unwrap()
		.ingress_port;

	// Test against origin
	compare_test_id_tcp(&ip, port as u16, &test_id).await;

	// Test via proxy
	compare_test_id_tcp(&ingress_hostname_tcp, ingress_port_tcp as u16, &test_id).await;

	// Test via proxy with TLS
	compare_test_id_tcp_tls(
		&ingress_hostname_tcp_tls,
		ingress_port_tcp_tls as u16,
		&test_id,
	)
	.await;
}

#[worker_test]
async fn basic_udp(ctx: TestCtx) {
	let domain_job = util::env::domain_job().unwrap();

	let region_res = op!([ctx] faker_region {}).await.unwrap();
	let region_id = region_res.region_id.as_ref().unwrap().as_uuid();
	let region_name_id = &region_res.region.as_ref().unwrap().name_id;

	let template_res = op!([ctx] faker_job_template {
		kind: Some(faker::job_template::request::Kind::EchoServerUdp(Default::default())),
		..Default::default()
	})
	.await
	.unwrap();

	// Run the job
	let run_id = Uuid::new_v4();
	let test_id = Uuid::new_v4().to_string();
	let mut planned_sub = subscribe!([ctx] job_run::msg::alloc_planned(run_id))
		.await
		.unwrap();
	let mut fail_sub = subscribe!([ctx] job_run::msg::fail(run_id)).await.unwrap();
	let mut eval_sub = subscribe!([ctx] job_run::msg::eval_complete(run_id))
		.await
		.unwrap();
	let ingress_hostname_udp = format!("test-{run_id}-udp.lobby.{region_name_id}.{domain_job}");
	msg!([ctx] job_run::msg::create(run_id) {
		run_id: Some(run_id.into()),
		region_id: Some(region_id.into()),
		parameters: vec![
			job_run::msg::create::Parameter {
				key: "test_id".into(),
				value: test_id.clone(),
			},
		],
		job_spec_json: template_res.job_spec_json.clone(),
		proxied_ports: vec![
			job_run::msg::create::ProxiedPort {
				#[allow(deprecated)]
				target_nomad_port_label: Some("udp".into()),
				ingress_port: None,
				ingress_hostnames: vec![ingress_hostname_udp.clone()],
				proxy_protocol: backend::job::ProxyProtocol::Udp as i32,
				ssl_domain_mode: backend::job::SslDomainMode::Exact as i32,
			},
		],
		..Default::default()
	})
	.await
	.unwrap();
	tokio::select! {
		_ = async {
			planned_sub.next().await.unwrap();
			eval_sub.next().await.unwrap();
		} => {
			tracing::info!("started");
		}
		res = fail_sub.next() => {
			let msg = res.unwrap();
			tracing::error!(?msg, "run failed");
			panic!("run failed");
		}
	}

	let ValidateJobOutput {
		ip,
		port,
		proxied_ports,
	} = validate_job(ctx.crdb().await.unwrap(), run_id, region_id, "udp").await;
	let ingress_port_udp = proxied_ports.first().unwrap().ingress_port;

	// Test against origin
	compare_test_id_udp(&ip, port as u16, &test_id).await;

	// Test via proxy
	compare_test_id_udp(&ingress_hostname_udp, ingress_port_udp as u16, &test_id).await;
}

#[worker_test]
async fn plan_fail(ctx: TestCtx) {
	let region_res = op!([ctx] faker_region {}).await.unwrap();
	let region_id = region_res.region_id.as_ref().unwrap().as_uuid();

	// Attempt to schedule a job with an impossible amount of memory and check that the job
	// schedules a failure
	let template_res = op!([ctx] faker_job_template {
		kind: Some(faker::job_template::request::Kind::EchoServer(Default::default())),
		memory_mb: Some(99_999_999),
		..Default::default()
	})
	.await
	.unwrap();

	// Run the job
	let run_id = Uuid::new_v4();
	let test_id = Uuid::new_v4().to_string();
	let mut fail_sub = subscribe!([ctx] job_run::msg::fail(run_id)).await.unwrap();
	let mut stop_sub = subscribe!([ctx] job_run::msg::stop(run_id)).await.unwrap();
	msg!([ctx] job_run::msg::create(run_id) {
		run_id: Some(run_id.into()),
		region_id: Some(region_id.into()),
		parameters: vec![
			job_run::msg::create::Parameter {
				key: "test_id".into(),
				value: test_id.clone(),
			},
		],
		job_spec_json: template_res.job_spec_json.clone(),
		proxied_ports: vec![],
		..Default::default()
	})
	.await
	.unwrap();
	let fail_msg = fail_sub.next().await.unwrap();
	stop_sub.next().await.unwrap();

	assert_eq!(
		job_run::msg::fail::ErrorCode::NomadEvalPlanFailed as i32,
		fail_msg.error_code
	);
}

#[tracing::instrument]
async fn compare_test_id_http(test_server_addr: &str, test_id: &str) {
	// Access the server to validate the test ID from the meta. Run this in
	// a loop since the job may not be running yet.
	tracing::info!(%test_server_addr, "fetching test id via http");
	loop {
		match reqwest::get(test_server_addr).await {
			Ok(res) => {
				if res.status().is_success() {
					let fetched_test_id = res.text().await.unwrap();
					assert_eq!(test_id, fetched_test_id.trim(), "returned wrong test id");
					break;
				} else {
					tracing::info!(
						status = ?res.status(),
						"returned non-200 response, probably not booted yet, retrying"
					);
					tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
				}
			}
			Err(err) => {
				tracing::info!(?err, "cannot reach job, probably not booted yet, retrying");
				tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
			}
		}
	}
}

#[tracing::instrument]
async fn compare_test_id_tcp(hostname: &str, port: u16, test_id: &str) {
	// Access the server to validate the test ID from the meta. Run this in
	// a loop since the job may not be running yet.
	tracing::info!(%hostname, %port, "fetching test id via tcp+tls");
	loop {
		let mut stream = match TcpStream::connect((hostname, port)) {
			Ok(x) => x,
			Err(err) => {
				tracing::info!(?err, "cannot reach job, probably not booted yet, retrying");
				tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
				continue;
			}
		};
		tracing::info!("connected");

		// Write an empty package to get the response
		stream.write_all(b"Hello, world!").unwrap();
		stream.flush().unwrap();
		tracing::info!("written");

		// Read the response test ID
		let mut fetched_test_id = String::new();
		stream.read_to_string(&mut fetched_test_id).unwrap();
		if fetched_test_id.trim().starts_with("HTTP") {
			tracing::info!(
				?fetched_test_id,
				"received http response, probably not booted yet, retrying"
			);
			tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
			continue;
		}
		assert_eq!(test_id, fetched_test_id.trim(), "returned wrong test id");
		tracing::info!("test id matches");

		break;
	}
}

#[tracing::instrument]
async fn compare_test_id_tcp_tls(hostname: &str, port: u16, test_id: &str) {
	// Create TLS root store
	let mut root_store = rustls::RootCertStore::empty();
	root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
		rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
			ta.subject,
			ta.spki,
			ta.name_constraints,
		)
	}));

	// Create TLS client
	let config = rustls::ClientConfig::builder()
		.with_safe_defaults()
		.with_root_certificates(root_store)
		.with_no_client_auth();
	let mut client =
		rustls::ClientConnection::new(Arc::new(config), hostname.try_into().unwrap()).unwrap();

	// Access the server to validate the test ID from the meta. Run this in
	// a loop since the job may not be running yet.
	tracing::info!(%hostname, %port, "fetching test id via tcp");
	loop {
		// Connect to server
		let mut stream = match TcpStream::connect((hostname, port)) {
			Ok(x) => x,
			Err(err) => {
				tracing::info!(?err, "cannot reach job, probably not booted yet, retrying");
				tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
				continue;
			}
		};
		tracing::info!("connected");

		let mut tls = rustls::Stream::new(&mut client, &mut stream);

		tls.write_all(b"Hello, world!").unwrap();
		tls.flush().unwrap();
		tracing::info!("written");

		// Read the response test ID
		let mut fetched_test_id = String::new();
		tls.read_to_string(&mut fetched_test_id).unwrap();
		if fetched_test_id.trim().starts_with("HTTP") {
			tracing::info!(
				?fetched_test_id,
				"received http response, probably not booted yet, retrying"
			);
			tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
			continue;
		}
		assert_eq!(test_id, fetched_test_id.trim(), "returned wrong test id");
		tracing::info!("test id matches");

		break;
	}
}

#[tracing::instrument]
async fn compare_test_id_udp(hostname: &str, port: u16, test_id: &str) {
	// Access the server to validate the test ID from the meta. Run this in
	// a loop since the job may not be running yet.
	tracing::info!(%hostname, %port, "fetching test id via udp");

	let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
	tracing::info!("bound");

	loop {
		match socket.connect(format!("{hostname}:{port}")) {
			Ok(x) => x,
			Err(err) => {
				tracing::info!(?err, "cannot reach job, probably not booted yet, retrying");
				tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
				continue;
			}
		}

		// Write an empty package to get the response
		match socket.send(b"Hello, world!") {
			Ok(_) => {}
			Err(err) => {
				tracing::info!(
					?err,
					"cannot write to udp socket, probably not booted yet, retrying"
				);
				tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
				continue;
			}
		}
		tracing::info!("written");

		// Read the response test ID
		let mut fetched_test_id_bytes = [0; 2048];
		let recv_len = match socket.recv(&mut fetched_test_id_bytes) {
			Ok(x) => x,
			Err(err) => {
				tracing::info!(
					?err,
					"cannot read from udp socket, probably not booted yet, retrying"
				);
				tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
				continue;
			}
		};
		let fetched_test_id =
			String::from_utf8(fetched_test_id_bytes[..recv_len].to_vec()).unwrap();
		assert_eq!(test_id, fetched_test_id.trim(), "returned wrong test id");
		tracing::info!("test id matches");

		break;
	}
}

struct ValidateJobOutput {
	ip: String,
	port: i32,
	proxied_ports: Vec<RunProxiedPort>,
}

#[derive(sqlx::FromRow)]
struct RunProxiedPort {
	ingress_port: i64,
	proxy_protocol: i64,
}

async fn validate_job(
	crdb: CrdbPool,
	run_id: Uuid,
	region_id: Uuid,
	port_label: &str,
) -> ValidateJobOutput {
	let nomad_config = nomad_util::config_from_env().unwrap();

	let (cql_region_id,) =
		sqlx::query_as::<_, (Uuid,)>("SELECT region_id FROM db_job_state.runs WHERE run_id = $1")
			.bind(run_id)
			.fetch_one(&crdb)
			.await
			.unwrap();
	let (dispatched_job_id,) = sqlx::query_as::<_, (String,)>(
		"SELECT dispatched_job_id FROM db_job_state.run_meta_nomad WHERE run_id = $1",
	)
	.bind(run_id)
	.fetch_one(&crdb)
	.await
	.unwrap();
	let proxied_ports = sqlx::query_as::<_, RunProxiedPort>(
		"SELECT ingress_port, proxy_protocol FROM db_job_state.run_proxied_ports WHERE run_id = $1",
	)
	.bind(run_id)
	.fetch_all(&crdb)
	.await
	.unwrap();

	assert_eq!(cql_region_id, region_id, "mismatching regions");

	// Validate the allocation exists
	let nomad_alloc_stubs = nomad_client::apis::jobs_api::get_job_allocations(
		&nomad_config,
		&dispatched_job_id,
		None,
		None,
		None,
		None,
		Some(true),
	)
	.await
	.unwrap();
	let nomad_alloc_stub = nomad_alloc_stubs.first().expect("nomad alloc not created");
	let nomad_alloc = nomad_client::apis::allocations_api::get_allocation(
		&nomad_config,
		&nomad_alloc_stub.ID.clone().unwrap(),
		None,
		None,
		None,
		None,
	)
	.await
	.unwrap();

	// Validate the network
	let network = nomad_alloc
		.resources
		.as_ref()
		.unwrap()
		.networks
		.as_ref()
		.unwrap()
		.first()
		.expect("missing network");
	let ip = network.IP.as_ref().unwrap();
	let ports = network.dynamic_ports.as_ref().unwrap();

	assert_eq!(1, ports.len(), "wrong port count");
	let port = ports.first().unwrap();
	assert_eq!(port_label, port.label.as_ref().unwrap());
	tracing::info!(?port, "job port");
	let port_value = port.value.unwrap();

	ValidateJobOutput {
		ip: ip.clone(),
		port: port_value,
		proxied_ports,
	}
}
