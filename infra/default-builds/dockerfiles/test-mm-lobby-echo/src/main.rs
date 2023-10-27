use anyhow::{Context, Result};
use std::{convert::Infallible, env, net::SocketAddr, process::Command};
use tokio::{
	io::{AsyncBufReadExt, AsyncWriteExt},
	net::{TcpListener, UdpSocket},
};

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

	// Echo servers (bridge networking)
	if let Ok(http_port) = env::var("PORT_test_http") {
		let http_port: u16 = http_port.parse()?;
		tokio::spawn(echo_http_server(http_port));
	}

	if let Ok(tcp_port) = env::var("PORT_test_tcp") {
		let tcp_port: u16 = tcp_port.parse()?;
		tokio::spawn(echo_tcp_server(tcp_port));
	}

	if let Ok(udp_port) = env::var("PORT_test_udp") {
		let udp_port: u16 = udp_port.parse()?;
		tokio::spawn(echo_udp_server(udp_port));
	}

	// Echo servers (host networking)
	if let Ok(http_port) = env::var("HOST_PORT_HTTP") {
		let http_port: u16 = http_port.parse()?;
		tokio::spawn(echo_http_server(http_port));
	}

	if let Ok(tcp_port) = env::var("HOST_PORT_TCP") {
		let tcp_port: u16 = tcp_port.parse()?;
		tokio::spawn(echo_tcp_server(tcp_port));
	}

	if let Ok(udp_port) = env::var("HOST_PORT_UDP") {
		let udp_port: u16 = udp_port.parse()?;
		tokio::spawn(echo_udp_server(udp_port));
	}

	// Lobby ready
	lobby_ready().await?;

	// Wait indefinitely
	println!("Waiting indefinitely...");
	std::future::pending::<()>().await;

	Ok(())
}

async fn lobby_ready() -> Result<()> {
	let url = format!(
		"{}/matchmaker/lobbies/ready",
		env::var("RIVET_API_ENDPOINT").context("RIVET_API_ENDPOINT")?
	);
	let token = env::var("RIVET_TOKEN").context("RIVET_TOKEN")?;

	let client = reqwest::Client::new();
	client
		.post(&url)
		.header("Content-Type", "application/json")
		.header("Authorization", format!("Bearer {}", token))
		.send()
		.await?;

	println!("Success, waiting indefinitely");
	Ok(())
}

async fn echo_http_server(port: u16) {
	use hyper::service::{make_service_fn, service_fn};
	use hyper::{Body, Request, Response, Server};

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
}

async fn echo_tcp_server(port: u16) {
	let addr = SocketAddr::from(([0, 0, 0, 0], port));
	println!("TCP: {}", port);

	let listener = TcpListener::bind(&addr).await.expect("bind failed");
	loop {
		let (socket, _) = listener.accept().await.expect("accept failed");
		tokio::spawn(async move {
			let mut reader = tokio::io::BufReader::new(socket);
			let mut line = String::new();
			loop {
				let bytes_read = reader.read_line(&mut line).await.expect("read line failed");
				if bytes_read == 0 {
					break;
				}

				// Echo the line
				reader
					.get_mut()
					.write_all(format!("{line}\n").as_bytes())
					.await
					.expect("write failed");
				reader.get_mut().flush().await.expect("flush failed");
				line.clear();
			}
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
