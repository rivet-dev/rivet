mod common;

use std::{
	io::{BufRead, BufReader, Write},
	net::{TcpStream, UdpSocket},
};

use chirp_workflow::prelude::*;
use common::*;

#[workflow_test]
async fn server_connectivity_http_normal(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let setup = Setup::init(&ctx).await;

	let server_id = setup.create_bridge_server(&ctx).await;

	let (hostname, _) = get_server_addr(&ctx, server_id, "test-http").await;
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

#[workflow_test]
async fn server_connectivity_http_host(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let setup = Setup::init(&ctx).await;

	let server_id = setup.create_host_server(&ctx).await;

	// Echo body (bridge)
	{
		let (hostname, _) = get_server_addr(&ctx, server_id, "test-http").await;
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

	// // Echo body (host)
	// {
	// 	let (hostname, port) = get_server_addr(&ctx, server_id, "test-host-http").await;
	// 	tracing::info!("testing http to {}:{}", hostname, port);

	// 	let random_body = Uuid::new_v4().to_string();
	// 	let client = reqwest::Client::new();
	// 	let res = client
	// 		.post(format!("http://{hostname}:{port}"))
	// 		.body(random_body.clone())
	// 		.send()
	// 		.await
	// 		.unwrap()
	// 		.error_for_status()
	// 		.unwrap();
	// 	let res_text = res.text().await.unwrap();
	// 	assert_eq!(random_body, res_text, "echoed wrong response");
	// }
}

#[workflow_test]
async fn server_connectivity_tcp(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let setup = Setup::init(&ctx).await;

	let server_id = setup.create_bridge_server(&ctx).await;

	let (hostname, port) = get_server_addr(&ctx, server_id, "test-tcp").await;
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

#[workflow_test]
async fn server_connectivity_tcp_host(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let setup = Setup::init(&ctx).await;

	let server_id = setup.create_host_server(&ctx).await;

	// Echo body (bridge)
	{
		let (hostname, port) = get_server_addr(&ctx, server_id, "test-tcp").await;
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

	// // Echo body (host)
	// {
	// 	let (hostname, port) = get_server_addr(&ctx, server_id, "test-host-tcp").await;
	// 	tracing::info!("testing tcp to {}:{}", hostname, port);

	// 	let random_body = Uuid::new_v4().to_string();
	// 	let mut stream = TcpStream::connect((hostname, port)).unwrap();

	// 	stream.write_all(random_body.as_ref()).unwrap();
	// 	stream.write_all(b"\n").unwrap();
	// 	stream.flush().unwrap();

	// 	let mut reader = BufReader::new(&stream);
	// 	let mut response = String::new();
	// 	reader.read_line(&mut response).expect("read line");

	// 	assert_eq!(
	// 		&random_body,
	// 		&response[..response.len() - 1],
	// 		"echoed wrong response"
	// 	);
	// }
}

#[workflow_test]
async fn server_connectivity_udp(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let setup = Setup::init(&ctx).await;

	let server_id = setup.create_bridge_server(&ctx).await;

	let (hostname, port) = get_server_addr(&ctx, server_id, "test-udp").await;
	tracing::info!("testing udp to {}:{}", hostname, port);

	// Echo body
	let random_body = Uuid::new_v4();
	let socket = UdpSocket::bind(("0.0.0.0", 0)).unwrap();
	socket.connect((hostname, port)).unwrap();
	socket.send(random_body.as_ref()).unwrap();

	let mut response = [0; 2048];
	let recv_len = socket.recv(&mut response).unwrap();

	assert_eq!(
		random_body.as_bytes(),
		&response[..recv_len],
		"echoed wrong response"
	);
}

#[workflow_test]
async fn server_connectivity_udp_host(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let setup = Setup::init(&ctx).await;

	let server_id = setup.create_host_server(&ctx).await;

	// Echo body (host)
	{
		let (hostname, port) = get_server_addr(&ctx, server_id, "test-udp").await;
		tracing::info!("testing udp to {}:{}", hostname, port);

		let random_body = Uuid::new_v4();
		let socket = UdpSocket::bind(("0.0.0.0", 0)).unwrap();
		socket.connect((hostname, port)).unwrap();
		socket.send(random_body.as_ref()).unwrap();

		let mut response = [0; 2048];
		let recv_len = socket.recv(&mut response).unwrap();

		assert_eq!(
			random_body.as_bytes(),
			&response[..recv_len],
			"echoed wrong response"
		);
	}

	// // Echo body (host)
	// {
	// 	let (hostname, port) = get_server_addr(&ctx, server_id, "test-host-udp").await;
	// 	tracing::info!("testing udp to {}:{}", hostname, port);

	// 	let random_body = Uuid::new_v4();
	// 	let socket = UdpSocket::bind(("0.0.0.0", 0)).unwrap();
	// 	socket.connect((hostname, port)).unwrap();
	// 	socket.send(random_body.as_ref()).unwrap();

	// 	let mut response = [0; 2048];
	// 	let recv_len = socket.recv(&mut response).unwrap();

	// 	assert_eq!(
	// 		random_body.as_bytes(),
	// 		&response[..recv_len],
	// 		"echoed wrong response"
	// 	);
	// }
}

async fn get_server_addr(ctx: &TestCtx, server_id: Uuid, port: &str) -> (String, u16) {
	let server = ctx
		.op(ds::ops::server::get::Input {
			server_ids: vec![server_id],
		})
		.await
		.unwrap()
		.servers
		.into_iter()
		.next()
		.unwrap();

	let hostname = server
		.network_ports
		.get(port)
		.expect("no port")
		.public_hostname
		.as_ref()
		.expect("no public hostname");
	let port = server
		.network_ports
		.get(port)
		.expect("no port")
		.public_port
		.as_ref()
		.expect("no public port");

	(hostname.clone(), *port as u16)
}
