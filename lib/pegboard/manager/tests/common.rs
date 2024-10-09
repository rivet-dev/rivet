use std::{
	convert::Infallible,
	net::SocketAddr,
	path::{Path, PathBuf},
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc, Once,
	},
};

use anyhow::Context;
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use hyper::{
	service::{make_service_fn, service_fn},
	Body, Response, Server,
};
use pegboard::protocol;
use pegboard_manager::{utils, Ctx};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tokio::{
	fs::File,
	io::BufReader,
	net::{TcpListener, TcpStream},
	process::Command,
	sync::Mutex,
};
use tokio_tungstenite::{tungstenite::protocol::Message, WebSocketStream};
use tokio_util::io::ReaderStream;
use tracing_subscriber::prelude::*;
use url::Url;
use uuid::Uuid;

pub const PROTOCOL_VERSION: u16 = 1;
pub const ARTIFACTS_PORT: u16 = 1234;

pub async fn send_packet(
	tx: &mut SplitSink<WebSocketStream<tokio::net::TcpStream>, Message>,
	packet: protocol::ToClient,
) {
	let buf = packet.serialize(PROTOCOL_VERSION).unwrap();
	tx.send(Message::Binary(buf)).await.unwrap();
}

pub async fn send_command(
	tx: &mut SplitSink<WebSocketStream<tokio::net::TcpStream>, Message>,
	cmd: protocol::Command,
) {
	let packet = protocol::ToClient::Commands(vec![protocol::CommandWrapper {
		index: utils::now(),
		inner: protocol::Raw::new(&cmd).unwrap(),
	}]);

	send_packet(tx, packet).await
}

pub async fn send_init_packet(tx: &mut SplitSink<WebSocketStream<tokio::net::TcpStream>, Message>) {
	send_packet(
		tx,
		protocol::ToClient::Init {
			last_event_idx: utils::now(),
			// Not necessary for the test
			api_endpoint: "".to_string(),
		},
	)
	.await
}

pub async fn start_echo_container(
	tx: &mut SplitSink<WebSocketStream<tokio::net::TcpStream>, Message>,
	container_id: Uuid,
	container_port: u16,
) {
	let cmd = protocol::Command::StartContainer {
		container_id,
		config: Box::new(protocol::ContainerConfig {
			image: protocol::Image {
				// Should match the URL in `serve_binaries`
				artifact_url: format!("http://127.0.0.1:{ARTIFACTS_PORT}/image"),
				kind: protocol::ImageKind::DockerImage,
				compression: protocol::ImageCompression::None,
			},
			container_runner_binary_url: format!("http://127.0.0.1:{ARTIFACTS_PORT}/0/runner"),
			root_user_enabled: false,
			env: [("PORT".to_string(), container_port.to_string())]
				.into_iter()
				.collect(),
			ports: [(
				"main".to_string(),
				protocol::Port::Host {
					protocol: protocol::TransportProtocol::Tcp,
				},
			)]
			.into_iter()
			.collect(),
			network_mode: protocol::NetworkMode::Host,
			resources: protocol::Resources {
				cpu: 100,
				memory: 10 * 1024 * 1024,
				memory_max: 15 * 1024 * 1024,
			},
			stakeholder: protocol::Stakeholder::DynamicServer {
				server_id: container_id,
			},
		}),
	};

	send_command(tx, cmd).await;
}

pub fn start_server<F, Fut>(
	ctx_wrapper: Arc<Mutex<Option<Arc<Ctx>>>>,
	close_tx: tokio::sync::watch::Sender<()>,
	port: u16,
	handle_connection: F,
) where
	F: Fn(Arc<Mutex<Option<Arc<Ctx>>>>, tokio::sync::watch::Sender<()>, TcpStream) -> Fut
		+ Send
		+ 'static,
	Fut: std::future::Future<Output = ()> + Send,
{
	tokio::spawn(async move {
		let addr = SocketAddr::from(([0, 0, 0, 0], port));

		let listener = TcpListener::bind(addr).await.unwrap();
		tracing::info!(?port, "server listening");

		loop {
			let (stream, _) = listener.accept().await.unwrap();

			handle_connection(ctx_wrapper.clone(), close_tx.clone(), stream).await;
		}
	});
}

pub async fn start_client(
	working_path: &Path,
	ctx_wrapper: Arc<Mutex<Option<Arc<Ctx>>>>,
	mut close_rx: tokio::sync::watch::Receiver<()>,
	port: u16,
) {
	let client_id = Uuid::new_v4();
	let datacenter_id = Uuid::new_v4();
	let network_ip = "127.0.0.1".parse().unwrap();

	// Read system metrics
	let system = System::new_with_specifics(
		RefreshKind::new()
			.with_cpu(CpuRefreshKind::new().with_frequency())
			.with_memory(MemoryRefreshKind::new().with_ram()),
	);

	// Init sqlite db
	let sqlite_db_url = format!(
		"sqlite://{}",
		working_path.join("db").join("database.db").display()
	);
	utils::init_sqlite_db(&sqlite_db_url).await.unwrap();

	// Connect to sqlite db
	let pool = utils::build_sqlite_pool(&sqlite_db_url).await.unwrap();
	utils::init_sqlite_schema(&pool).await.unwrap();

	// Build WS connection URL
	let mut url = Url::parse("ws://127.0.0.1").unwrap();
	url.set_port(Some(port)).unwrap();
	url.set_path(&format!("/v{PROTOCOL_VERSION}"));
	url.query_pairs_mut()
		.append_pair("client_id", &client_id.to_string())
		.append_pair("datacenter_id", &datacenter_id.to_string());

	tracing::info!("connecting to ws: {url}");

	// Connect to WS
	let (ws_stream, _) = tokio_tungstenite::connect_async(url.to_string())
		.await
		.context("failed to connect to websocket")
		.unwrap();
	let (tx, rx) = ws_stream.split();

	tracing::info!("connected");

	let ctx = Ctx::new(working_path.to_path_buf(), network_ip, system, pool, tx);

	// Share reference
	{
		*ctx_wrapper.lock().await = Some(ctx.clone());
	}

	tokio::select! {
		res = ctx.start(rx) => res.unwrap(),
		_ = close_rx.changed() => {}
	}

	// Remove reference
	{
		*ctx_wrapper.lock().await = None;
	}

	tracing::info!("client stopped");
}

pub async fn build_binaries(gen_path: &Path) {
	let echo_server_crate_path = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("..")
		.join("echo");
	let status = Command::new("docker")
		.current_dir(&echo_server_crate_path)
		.arg("build")
		.arg("--platform")
		.arg("linux/amd64")
		.arg("-t")
		.arg("pegboard-echo-server")
		.arg(".")
		.status()
		.await
		.unwrap();

	assert!(status.success());

	tracing::info!("saving echo image");

	let status = Command::new("docker")
		.current_dir(&echo_server_crate_path)
		.arg("save")
		.arg("-o")
		.arg(image_path(gen_path))
		.arg("pegboard-echo-server")
		.status()
		.await
		.unwrap();

	assert!(status.success());

	// Build runner binary
	let container_runner_crate_path = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("..")
		.join("container-runner");
	let image_name = "pegboard-container-runner";
	let status = Command::new("docker")
		.current_dir(&container_runner_crate_path)
		.arg("build")
		.arg("--platform")
		.arg("linux/amd64")
		.arg("-t")
		.arg(image_name)
		.arg(".")
		.status()
		.await
		.unwrap();

	assert!(status.success());

	tracing::info!("copying runner image");

	let container_name = "temp-pegboard-container-runner-container";
	let binary_path_in_container = "/app/target/x86_64-unknown-linux-musl/release/container-runner";

	// Create a temporary container
	let status = Command::new("docker")
		.arg("create")
		.arg("--name")
		.arg(container_name)
		.arg(image_name)
		.status()
		.await
		.expect("Failed to create container");
	assert!(status.success());

	// Copy the binary from the container to the host
	let status = Command::new("docker")
		.arg("cp")
		.arg(format!("{}:{}", container_name, binary_path_in_container))
		.arg(container_runner_path(gen_path))
		.status()
		.await
		.expect("Failed to copy binary from container");
	assert!(status.success());

	// Remove the temporary container
	let status = Command::new("docker")
		.arg("rm")
		.arg(container_name)
		.status()
		.await
		.expect("Failed to remove container");
	assert!(status.success());
}

pub async fn serve_binaries(gen_path: PathBuf) {
	let make_svc = make_service_fn(|_conn| {
		let gen_path = gen_path.clone();
		async move {
			Ok::<_, Infallible>(service_fn(move |req| {
				let gen_path = gen_path.clone();
				async move {
					let gen_path = gen_path;
					let path = req.uri().path();

					let path = if path == "/0/runner" {
						container_runner_path(&gen_path)
					} else if path == "/image" {
						image_path(&gen_path)
					} else {
						panic!("invalid path");
					};

					let file = File::open(path).await?;
					let stream = ReaderStream::new(BufReader::new(file));
					let body = Body::wrap_stream(stream);

					Result::<_, std::io::Error>::Ok(Response::new(body))
				}
			}))
		}
	});

	let server = Server::bind(&([127, 0, 0, 1], ARTIFACTS_PORT).into()).serve(make_svc);

	tracing::info!(port=?ARTIFACTS_PORT, "serving binaries");

	server.await.unwrap();
}

pub async fn start_vector() {
	let config_path = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("tests")
		.join("vector.json");

	let status = Command::new("vector")
		.arg("-c")
		.arg(config_path)
		.arg("-q")
		.env("VECTOR_NO_GRACEFUL_SHUTDOWN_LIMIT", "1")
		.status()
		.await
		.unwrap();

	assert!(status.success());
}

static SETUP_DEPENDENCIES: AtomicBool = AtomicBool::new(false);
pub async fn setup_dependencies() -> Option<tempfile::TempDir> {
	if !SETUP_DEPENDENCIES.swap(true, Ordering::SeqCst) {
		let tmp_dir = tempfile::TempDir::new().unwrap();
		build_binaries(tmp_dir.path()).await;

		tokio::spawn(serve_binaries(tmp_dir.path().to_path_buf()));

		tokio::spawn(start_vector());

		Some(tmp_dir)
	} else {
		None
	}
}

pub fn container_runner_path(gen_path: &Path) -> PathBuf {
	gen_path.join("pegboard-container-runner").to_path_buf()
}

pub fn image_path(gen_path: &Path) -> PathBuf {
	gen_path.join("pegboard-echo-server").to_path_buf()
}

static SETUP_TRACING: Once = Once::new();
pub fn setup_tracing() {
	SETUP_TRACING.call_once(|| {
		tracing_subscriber::registry()
			.with(
				tracing_logfmt::builder()
					.layer()
					.with_filter(tracing_subscriber::filter::LevelFilter::INFO),
			)
			.init();
	});
}
