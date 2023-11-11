mod common;

use chirp_worker::prelude::*;
use common::*;
use proto::backend::{self, pkg::*};
use std::{
	io::{BufRead, BufReader, Read, Write},
	net::{TcpStream, UdpSocket},
};

#[worker_test]
async fn lobby_connectivity_http(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let setup = Setup::init(&ctx).await;

	let lobby_id = setup.create_lobby(&ctx).await;

	let (hostname, _) = get_lobby_addr(&ctx, lobby_id, "test-http").await;
	tracing::info!("testing http to {}", hostname);

	// Echo body
	let random_body = Uuid::new_v4().to_string();
	let client = reqwest::Client::new();
	let res = client
		.post(format!("http://{hostname}"))
		.body(random_body.clone())
		.send()
		.await
		.unwrap()
		.error_for_status()
		.unwrap();
	let res_text = res.text().await.unwrap();
	assert_eq!(random_body, res_text, "echoed wrong response");
}

#[worker_test]
async fn lobby_connectivity_http_host(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let setup = Setup::init(&ctx).await;

	let lobby_id = setup
		.create_lobby_with_lgi(&ctx, setup.lobby_group_id_host)
		.await;

	// Echo body (bridge)
	{
		let (hostname, _) = get_lobby_addr(&ctx, lobby_id, "test-http").await;
		tracing::info!("testing http to {}", hostname);

		let random_body = Uuid::new_v4().to_string();
		let client = reqwest::Client::new();
		let res = client
			.post(format!("http://{hostname}"))
			.body(random_body.clone())
			.send()
			.await
			.unwrap()
			.error_for_status()
			.unwrap();
		let res_text = res.text().await.unwrap();
		assert_eq!(random_body, res_text, "echoed wrong response");
	}

	// Echo body (host)
	{
		let host_ip = get_lobby_host_ip(&ctx, lobby_id).await;
		tracing::info!("testing http to {}:{}", host_ip, setup.host_port_http);

		let random_body = Uuid::new_v4().to_string();
		let client = reqwest::Client::new();
		let res = client
			.post(format!("http://{host_ip}:{}", setup.host_port_http))
			.body(random_body.clone())
			.send()
			.await
			.unwrap()
			.error_for_status()
			.unwrap();
		let res_text = res.text().await.unwrap();
		assert_eq!(random_body, res_text, "echoed wrong response");
	}
}

#[worker_test]
async fn lobby_connectivity_tcp(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let setup = Setup::init(&ctx).await;

	let lobby_id = setup.create_lobby(&ctx).await;

	let (hostname, port) = get_lobby_addr(&ctx, lobby_id, "test-tcp").await;
	tracing::info!("testing tcp to {}:{}", hostname, port);

	// Echo body
	let random_body = Uuid::new_v4().to_string();
	let mut stream = TcpStream::connect((hostname, port)).unwrap();

	stream.write_all(random_body.as_ref()).unwrap();
	stream.write_all(b"\n").unwrap();
	stream.flush().unwrap();

	let mut reader = BufReader::new(&stream);
	let mut response = String::new();
	reader.read_line(&mut response).expect("read line");

	assert_eq!(
		&random_body,
		&response[..response.len() - 1],
		"echoed wrong response"
	);
}

#[worker_test]
async fn lobby_connectivity_tcp_host(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let setup = Setup::init(&ctx).await;

	let lobby_id = setup
		.create_lobby_with_lgi(&ctx, setup.lobby_group_id_host)
		.await;

	// Echo body (bridge)
	{
		let (hostname, port) = get_lobby_addr(&ctx, lobby_id, "test-tcp").await;
		tracing::info!("testing tcp to {}:{}", hostname, port);

		// Echo body
		let random_body = Uuid::new_v4().to_string();
		let mut stream = TcpStream::connect((hostname, port)).unwrap();

		stream.write_all(random_body.as_ref()).unwrap();
		stream.write_all(b"\n").unwrap();
		stream.flush().unwrap();

		let mut reader = BufReader::new(&stream);
		let mut response = String::new();
		reader.read_line(&mut response).expect("read line");

		assert_eq!(
			&random_body,
			&response[..response.len() - 1],
			"echoed wrong response"
		);
	}

	// Echo body (host)
	{
		let host_ip = get_lobby_host_ip(&ctx, lobby_id).await;
		tracing::info!("testing tcp to {}:{}", host_ip, setup.host_port_tcp);

		let random_body = Uuid::new_v4().to_string();
		let mut stream = TcpStream::connect((host_ip, setup.host_port_tcp)).unwrap();

		stream.write_all(random_body.as_ref()).unwrap();
		stream.write_all(b"\n").unwrap();
		stream.flush().unwrap();

		let mut reader = BufReader::new(&stream);
		let mut response = String::new();
		reader.read_line(&mut response).expect("read line");

		assert_eq!(
			&random_body,
			&response[..response.len() - 1],
			"echoed wrong response"
		);
	}
}

#[worker_test]
async fn lobby_connectivity_udp(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let setup = Setup::init(&ctx).await;

	let lobby_id = setup.create_lobby(&ctx).await;

	let (hostname, port) = get_lobby_addr(&ctx, lobby_id, "test-udp").await;
	tracing::info!("testing udp to {}:{}", hostname, port);

	// Echo body
	let random_body = Uuid::new_v4();
	let socket = UdpSocket::bind(("0.0.0.0", 0)).unwrap();
	socket.connect((hostname, port)).unwrap();
	socket.send(random_body.as_ref()).unwrap();

	let mut response = [0; 2048];
	let recv_len = socket.recv(&mut response).unwrap();

	assert_eq!(
		random_body.as_ref(),
		&response[..recv_len],
		"echoed wrong response"
	);
}

#[worker_test]
async fn lobby_connectivity_udp_host(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let setup = Setup::init(&ctx).await;

	let lobby_id = setup
		.create_lobby_with_lgi(&ctx, setup.lobby_group_id_host)
		.await;

	let host_ip = get_lobby_host_ip(&ctx, lobby_id).await;

	// Echo body (bridge)
	{
		let (hostname, port) = get_lobby_addr(&ctx, lobby_id, "test-udp").await;
		tracing::info!("testing udp to {}:{}", hostname, port);

		let random_body = Uuid::new_v4();
		let socket = UdpSocket::bind(("0.0.0.0", 0)).unwrap();
		socket.connect((hostname, port)).unwrap();
		socket.send(random_body.as_ref()).unwrap();

		let mut response = [0; 2048];
		let recv_len = socket.recv(&mut response).unwrap();

		assert_eq!(
			random_body.as_ref(),
			&response[..recv_len],
			"echoed wrong response"
		);
	}

	// Echo body (host)
	{
		tracing::info!("testing udp to {}:{}", host_ip, setup.host_port_udp);

		let random_body = Uuid::new_v4();
		let socket = UdpSocket::bind(("0.0.0.0", 0)).unwrap();
		socket.connect((host_ip, setup.host_port_udp)).unwrap();
		socket.send(random_body.as_ref()).unwrap();

		let mut response = [0; 2048];
		let recv_len = socket.recv(&mut response).unwrap();

		assert_eq!(
			random_body.as_ref(),
			&response[..recv_len],
			"echoed wrong response"
		);
	}
}

/// Fetches the address to get the lobby from.
async fn get_lobby_addr(ctx: &TestCtx, lobby_id: Uuid, port: &str) -> (String, u16) {
	let lobby_res = op!([ctx] mm_lobby_get { lobby_ids: vec![lobby_id.into()] })
		.await
		.unwrap();
	let lobby = lobby_res.lobbies.first().unwrap();
	let run_id = lobby.run_id.unwrap();

	let run_res = op!([ctx] job_run_get { run_ids: vec![run_id] })
		.await
		.unwrap();
	let run = run_res.runs.first().unwrap();

	let port = run
		.proxied_ports
		.iter()
		.find(|x| x.target_nomad_port_label == Some(util_mm::format_nomad_port_label(port)))
		.unwrap();

	(
		port.ingress_hostnames.first().unwrap().clone(),
		port.ingress_port as u16,
	)
}

/// Fetches the address to get the lobby from for host networking.
async fn get_lobby_host_ip(ctx: &TestCtx, lobby_id: Uuid) -> String {
	let lobby_res = op!([ctx] mm_lobby_get { lobby_ids: vec![lobby_id.into()] })
		.await
		.unwrap();
	let lobby = lobby_res.lobbies.first().unwrap();
	let run_id = lobby.run_id.unwrap();

	let run_res = op!([ctx] job_run_get { run_ids: vec![run_id] })
		.await
		.unwrap();
	let run = run_res.runs.first().unwrap();

	let run_meta = run.run_meta.as_ref().unwrap();
	let Some(backend::job::run_meta::Kind::Nomad(run_meta_nomad)) = &run_meta.kind else {
		panic!()
	};
	let node_public_ipv4 = run_meta_nomad.node_public_ipv4.clone().unwrap();

	node_public_ipv4
}
