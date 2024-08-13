use std::{convert::Infallible, env, net::SocketAddr, process::Command};

use anyhow::{Context, Result};
use futures_util::{sink::SinkExt, stream::StreamExt};
use tokio::net::{TcpListener, UdpSocket};
use tokio_util::codec::{BytesCodec, Framed};

#[tokio::main]
async fn main() -> Result<()> {
	// Env
	let envs: Vec<(String, String)> = env::vars().collect();
	println!("Env:\n{:#?}\n", envs);

	// resolv.conf
	let output = Command::new("cat")
		.arg("/etc/resolv.conf")
		.output()
		.expect("Failed to execute command");
	println!(
		"resolv.conf:\n{}\n",
		String::from_utf8_lossy(&output.stdout)
	);

	// Echo servers
	if let Ok(http_port) = env::var("HTTP_PORT") {
		let http_port: u16 = http_port.parse()?;
		tokio::spawn(with_select_term(echo_http_server(http_port)));
	}

	if let Ok(tcp_port) = env::var("TCP_PORT") {
		let tcp_port: u16 = tcp_port.parse()?;
		tokio::spawn(with_select_term(echo_tcp_server(tcp_port)));
	}

	if let Ok(udp_port) = env::var("UDP_PORT") {
		let udp_port: u16 = udp_port.parse()?;
		tokio::spawn(with_select_term(echo_udp_server(udp_port)));
	}

	// Wait indefinitely
	println!("Waiting indefinitely...");
	wait_term().await?;
	println!("Ctrl+C pressed. Exiting main...");

	Ok(())
}

/// Waits for the SIGTERM signal.
async fn wait_term() -> Result<()> {
	use tokio::signal::unix::{signal, SignalKind};

	signal(SignalKind::terminate())
		.expect("Failed to set up SIGTERM handler")
		.recv()
		.await;

	Ok(())
}

/// Waits until future exits or term.
async fn with_select_term(future: impl std::future::Future<Output = Result<()>>) -> Result<()> {
	tokio::select! {
		result = future => result,
		_ = wait_term() => {
			println!("Ctrl+C pressed. Exiting...");
			Ok(())
		},
	}
}

async fn echo_http_server(port: u16) -> Result<()> {
	use hyper::{
		service::{make_service_fn, service_fn},
		Body, Request, Response, Server,
	};

	let addr = SocketAddr::from(([0, 0, 0, 0], port));
	println!("HTTP: {}", port);

	async fn echo(req: Request<Body>) -> Result<Response<Body>, Infallible> {
		Ok(Response::new(req.into_body()))
	}

	let make_service = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(echo)) });
	Server::bind(&addr)
		.serve(make_service)
		.await
		.expect("hyper server");

	Ok(())
}

async fn echo_tcp_server(port: u16) -> Result<()> {
	let addr = SocketAddr::from(([0, 0, 0, 0], port));
	println!("TCP: {}", port);

	let listener = TcpListener::bind(&addr).await.context("bind failed")?;
	loop {
		let (socket, addr) = listener.accept().await.context("accept failed")?;
		println!("connection: {addr}");

		tokio::spawn(async move {
			let mut framed = Framed::new(socket, BytesCodec::new());

			while let Some(res) = framed.next().await {
				framed
					.send(res.expect("read failed"))
					.await
					.expect("write failed");
			}

			println!("connection closed: {addr}");
		});
	}
}

async fn echo_udp_server(port: u16) -> Result<()> {
	let addr = SocketAddr::from(([0, 0, 0, 0], port));
	println!("UDP: {}", port);

	let socket = UdpSocket::bind(&addr).await?;
	let mut buf = vec![0u8; 1024];
	loop {
		let (size, src) = socket.recv_from(&mut buf).await?;
		let data = String::from_utf8_lossy(&buf[..size]);
		println!("Received data: {}", data);

		socket.send_to(&buf[..size], &src).await?;
	}
}
