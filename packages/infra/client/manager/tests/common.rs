use std::{
	convert::Infallible,
	net::SocketAddr,
	path::{Path, PathBuf},
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc, Once,
	},
	time::Duration,
};

use anyhow::Context;
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use hyper::{
	service::{make_service_fn, service_fn},
	Body, Response, Server,
};
use pegboard::protocol;
use pegboard_config::*;
use pegboard_manager::{system_info, utils, Ctx};
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
		},
	)
	.await
}

pub async fn start_echo_actor(
	tx: &mut SplitSink<WebSocketStream<tokio::net::TcpStream>, Message>,
	actor_id: Uuid,
	port: u16,
) {
	let cmd = protocol::Command::StartActor {
		actor_id,
		config: Box::new(protocol::ActorConfig {
			image: protocol::Image {
				id: Uuid::nil(),
				artifact_url_stub: "/image".into(),
				fallback_artifact_url: None,
				kind: protocol::ImageKind::DockerImage,
				compression: protocol::ImageCompression::None,
			},
			root_user_enabled: false,
			env: [("PORT".to_string(), port.to_string())]
				.into_iter()
				.collect(),
			ports: [(
				"main".to_string(),
				protocol::Port {
					target: None,
					protocol: protocol::TransportProtocol::Tcp,
					routing: protocol::PortRouting::Host,
				},
			)]
			.into_iter()
			.collect(),
			network_mode: protocol::NetworkMode::Host,
			resources: protocol::Resources {
				cpu: 100,
				memory: 10 * 1024 * 1024,
				memory_max: 15 * 1024 * 1024,
				disk: 15,
			},
			owner: protocol::ActorOwner::DynamicServer {
				server_id: actor_id,
			},
			metadata: protocol::Raw::new(&protocol::ActorMetadata {
				actor: protocol::ActorMetadataActor {
					actor_id,
					tags: [("foo".to_string(), "bar".to_string())]
						.into_iter()
						.collect(),
					create_ts: 0,
				},
				project: protocol::ActorMetadataProject {
					project_id: Uuid::nil(),
					slug: "foo".to_string(),
				},
				environment: protocol::ActorMetadataEnvironment {
					env_id: Uuid::nil(),
					slug: "foo".to_string(),
				},
				datacenter: protocol::ActorMetadataDatacenter {
					name_id: "local".to_string(),
					display_name: "Local".to_string(),
				},
				cluster: protocol::ActorMetadataCluster {
					cluster_id: Uuid::nil(),
				},
				build: protocol::ActorMetadataBuild {
					build_id: Uuid::nil(),
				},
			})
			.unwrap(),
		}),
	};

	send_command(tx, cmd).await;
}

pub async fn start_js_echo_actor(
	tx: &mut SplitSink<WebSocketStream<tokio::net::TcpStream>, Message>,
	actor_id: Uuid,
) {
	let cmd = protocol::Command::StartActor {
		actor_id,
		config: Box::new(protocol::ActorConfig {
			image: protocol::Image {
				id: Uuid::nil(),
				artifact_url_stub: "/js-image".into(),
				fallback_artifact_url: None,
				kind: protocol::ImageKind::JavaScript,
				compression: protocol::ImageCompression::None,
			},
			root_user_enabled: false,
			env: Default::default(),
			ports: [(
				"main".to_string(),
				protocol::Port {
					target: None,
					protocol: protocol::TransportProtocol::Tcp,
					routing: protocol::PortRouting::Host,
				},
			)]
			.into_iter()
			.collect(),
			network_mode: protocol::NetworkMode::Host,
			resources: protocol::Resources {
				cpu: 100,
				memory: 10 * 1024 * 1024,
				memory_max: 15 * 1024 * 1024,
				disk: 15,
			},
			owner: protocol::ActorOwner::DynamicServer {
				server_id: actor_id,
			},
			metadata: protocol::Raw::new(&protocol::ActorMetadata {
				actor: protocol::ActorMetadataActor {
					actor_id,
					tags: [("foo".to_string(), "bar".to_string())]
						.into_iter()
						.collect(),
					create_ts: 0,
				},
				project: protocol::ActorMetadataProject {
					project_id: Uuid::nil(),
					slug: "foo".to_string(),
				},
				environment: protocol::ActorMetadataEnvironment {
					env_id: Uuid::nil(),
					slug: "foo".to_string(),
				},
				datacenter: protocol::ActorMetadataDatacenter {
					name_id: "local".to_string(),
					display_name: "Local".to_string(),
				},
				cluster: protocol::ActorMetadataCluster {
					cluster_id: Uuid::nil(),
				},
				build: protocol::ActorMetadataBuild {
					build_id: Uuid::nil(),
				},
			})
			.unwrap(),
		}),
	};

	send_command(tx, cmd).await;
}

pub fn start_server<F, Fut>(
	ctx_wrapper: Arc<Mutex<Option<Arc<Ctx>>>>,
	close_tx: Arc<tokio::sync::watch::Sender<()>>,
	port: u16,
	handle_connection: F,
) where
	F: Fn(Arc<Mutex<Option<Arc<Ctx>>>>, Arc<tokio::sync::watch::Sender<()>>, TcpStream) -> Fut
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

pub async fn init_client(gen_path: &Path, working_path: &Path) -> Config {
	let container_runner_binary_path = working_path.join("bin").join("container-runner");
	let isolate_runner_binary_path = working_path.join("bin").join("isolate-runner");

	tokio::fs::create_dir(working_path.join("bin"))
		.await
		.unwrap();

	// Copy binaries
	tokio::fs::copy(
		container_runner_path(gen_path),
		&container_runner_binary_path,
	)
	.await
	.unwrap();
	tokio::fs::copy(
		isolate_v8_runner_path(gen_path),
		&isolate_runner_binary_path,
	)
	.await
	.unwrap();

	let config = Config {
		client: Client {
			data_dir: Some(working_path.to_path_buf()),
			cluster: Cluster {
				client_id: Uuid::new_v4(),
				datacenter_id: Uuid::new_v4(),
				pegboard_endpoint: Url::parse("ws://127.0.0.1:5030").unwrap(),
				// Not necessary for the test
				api_endpoint: Url::parse("http://127.0.0.1").unwrap(),
			},
			runner: Runner {
				// Not necessary for the test
				flavor: protocol::ClientFlavor::Container,
				port: None,
				use_mounts: Some(false),
				container_runner_binary_path: Some(container_runner_binary_path),
				isolate_runner_binary_path: Some(isolate_runner_binary_path),
			},
			images: Images {
				// Should match the URL in `serve_binaries`
				pull_addresses: Some(Addresses::Static(vec![format!(
					"http://127.0.0.1:{ARTIFACTS_PORT}"
				)])),
			},
			network: Network {
				bind_ip: "127.0.0.1".parse().unwrap(),
				lan_hostname: "127.0.0.1".into(),
				wan_hostname: "127.0.0.1".into(),
				lan_port_range_min: None,
				lan_port_range_max: None,
				wan_port_range_min: None,
				wan_port_range_max: None,
			},
			cni: Default::default(),
			reserved_resources: Default::default(),
			logs: Logs {
				redirect_logs: Some(false),
				retention: None,
			},
			metrics: Default::default(),
			foundationdb: FoundationDb {
				cluster_description: "fdb".into(),
				cluster_id: "fdb".into(),
				addresses: Addresses::Static(vec!["127.0.0.1:4500".into()]),
			},
			vector: Some(Vector {
				address: "127.0.0.1:5021".into(),
			}),
		},
	};

	utils::init_dir(&config).await.unwrap();

	config
}

pub async fn start_client(
	config: Config,
	ctx_wrapper: Arc<Mutex<Option<Arc<Ctx>>>>,
	mut close_rx: tokio::sync::watch::Receiver<()>,
	port: u16,
) {
	let system = system_info::fetch().await.unwrap();

	// Init sqlite db
	let sqlite_db_url = format!(
		"sqlite://{}",
		config
			.client
			.data_dir()
			.join("db")
			.join("database.db")
			.display()
	);
	utils::init_sqlite_db(&sqlite_db_url).await.unwrap();

	// Connect to sqlite db
	let pool = utils::build_sqlite_pool(&sqlite_db_url).await.unwrap();
	utils::init_sqlite_schema(&pool).await.unwrap();

	// Init FDB config files
	utils::init_fdb_config(&config).await.unwrap();

	// Fetch ATS addresses
	let pull_addresses = utils::fetch_pull_addresses(&config).await.unwrap();

	// Build WS connection URL
	let mut url = Url::parse("ws://127.0.0.1").unwrap();
	url.set_port(Some(port)).unwrap();
	url.set_path(&format!("/v{PROTOCOL_VERSION}"));
	url.query_pairs_mut()
		.append_pair("client_id", &config.client.cluster.client_id.to_string())
		.append_pair(
			"datacenter_id",
			&config.client.cluster.datacenter_id.to_string(),
		);

	tracing::info!("connecting to ws: {url}");

	// Connect to WS
	let (ws_stream, _) = tokio_tungstenite::connect_async(url.to_string())
		.await
		.context("failed to connect to websocket")
		.unwrap();
	let (tx, rx) = ws_stream.split();

	tracing::info!("connected");

	let ctx = Ctx::new(config, system, pool, tx, pull_addresses);

	// Share reference
	{
		*ctx_wrapper.lock().await = Some(ctx.clone());
	}

	tokio::select! {
		res = ctx.run(rx) => res.unwrap(),
		_ = close_rx.changed() => {}
	}

	// Remove reference
	{
		*ctx_wrapper.lock().await = None;
	}

	tracing::info!("client stopped");
}

pub async fn build_binaries(gen_path: &Path) {
	let pkg_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
	let status = Command::new("docker")
		.arg("build")
		.arg("--platform")
		.arg("linux/amd64")
		.arg("-t")
		.arg("pegboard-echo-server")
		.arg("-f")
		.arg(pkg_path.join(format!("echo")).join("Dockerfile"))
		.arg(pkg_path.join("..").join("..").join(".."))
		.status()
		.await
		.unwrap();

	assert!(status.success());

	tracing::info!("saving echo image");

	let status = Command::new("docker")
		.arg("save")
		.arg("-o")
		.arg(image_path(gen_path))
		.arg("pegboard-echo-server")
		.status()
		.await
		.unwrap();

	assert!(status.success());

	// Js image
	let status = Command::new("tar")
		.arg("-cf")
		.arg(js_image_path(gen_path))
		.arg("-C")
		.arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests"))
		.arg("index.js")
		.status()
		.await
		.unwrap();

	assert!(status.success());

	build_runner(gen_path, "container").await;
	build_runner(gen_path, "isolate-v8").await;
}

async fn build_runner(gen_path: &Path, variant: &str) {
	tracing::info!("building {variant} runner");

	let pkg_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
	let image_name = format!("pegboard-{variant}-runner");

	// Build runner binary
	let status = Command::new("docker")
		.arg("build")
		.arg("--platform")
		.arg("linux/amd64")
		.arg("-t")
		.arg(&image_name)
		.arg("-f")
		.arg(
			pkg_path
				.join(format!("{variant}-runner"))
				.join("Dockerfile"),
		)
		.arg(pkg_path.join("..").join("..").join(".."))
		.status()
		.await
		.unwrap();

	assert!(status.success());

	tracing::info!("copying runner image");

	let container_name = format!("temp-pegboard-{variant}-runner-container");
	let binary_path_in_container = format!("/rivet-{variant}-runner");

	// Create a temporary container
	let status = Command::new("docker")
		.arg("create")
		.arg("--name")
		.arg(&container_name)
		.arg(&image_name)
		.status()
		.await
		.expect("Failed to create container");
	assert!(status.success());

	// Copy the binary from the container to the host
	let status = Command::new("docker")
		.arg("cp")
		.arg(format!("{}:{}", container_name, binary_path_in_container))
		.arg(if variant == "container" {
			container_runner_path(gen_path)
		} else {
			isolate_v8_runner_path(gen_path)
		})
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

					let path = if path == "/image" {
						image_path(&gen_path)
					} else if path == "/js-image" {
						js_image_path(&gen_path)
					} else {
						panic!("invalid path: {path}");
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

	Command::new("docker")
		.arg("rm")
		.arg("test-vector")
		.arg("--force")
		.status()
		.await
		.unwrap();

	let status = Command::new("docker")
		.arg("run")
		.arg("-v")
		.arg(format!(
			"{}:/etc/vector/vector.json:ro",
			config_path.display()
		))
		.arg("--rm")
		.arg("-p")
		.arg("5020:5020")
		.arg("-p")
		.arg("5021:5021")
		.arg("--name")
		.arg("test-vector")
		.arg("timberio/vector:0.42.0-debian")
		.arg("-c")
		.arg("/etc/vector/vector.json")
		.status()
		.await
		.unwrap();

	assert!(status.success());
}

pub async fn start_fdb() {
	Command::new("docker")
		.arg("rm")
		.arg("test-fdb")
		.arg("--force")
		.status()
		.await
		.unwrap();

	let status = Command::new("docker")
		.arg("run")
		.arg("--rm")
		.arg("-p")
		.arg("4500:4500")
		.arg("--name")
		.arg("test-fdb")
		.arg("-e")
		.arg("FDB_CLUSTER_FILE_CONTENTS=fdb:fdb@127.0.0.1:4500")
		// See docs-internal/infrastructure/fdb/AVX.md
		.arg("foundationdb/foundationdb:7.1.60")
		.status()
		.await
		.unwrap();

	assert!(status.success());
}

pub async fn create_fdb_db() {
	loop {
		// Create db
		let status = Command::new("docker")
			.arg("exec")
			.arg("test-fdb")
			.arg("fdbcli")
			.arg("--exec")
			.arg(r#"configure new single ssd"#)
			.status()
			.await
			.unwrap();

		if status.success() {
			break;
		} else {
			tracing::error!("failed to create fdb database");
		}

		tokio::time::sleep(Duration::from_secs(1)).await;
	}
}

static SETUP_DEPENDENCIES: AtomicBool = AtomicBool::new(false);
static mut TEMP_DIR_PATH: Option<PathBuf> = None;

pub async fn setup_dependencies() -> (Option<tempfile::TempDir>, PathBuf) {
	if !SETUP_DEPENDENCIES.swap(true, Ordering::SeqCst) {
		let tmp_dir = tempfile::TempDir::new().unwrap();
		let tmp_dir_path = tmp_dir.path().to_path_buf();

		// SAFETY: We are the only thread that can modify TEMP_DIR_PATH at this point, as we have just
		// swapped SETUP_DEPENDENCIES to true.
		unsafe {
			TEMP_DIR_PATH = Some(tmp_dir_path.clone());
		}

		build_binaries(tmp_dir.path()).await;

		tokio::spawn(serve_binaries(tmp_dir.path().to_path_buf()));

		tokio::spawn(start_vector());

		tokio::spawn(start_fdb());
		create_fdb_db().await;

		(Some(tmp_dir), tmp_dir_path)
	} else {
		// SAFETY: Once SETUP_DEPENDENCIES is true, TEMP_DIR_PATH is guaranteed to be initialized.
		(None, unsafe { TEMP_DIR_PATH.clone().unwrap() })
	}
}

pub fn container_runner_path(gen_path: &Path) -> PathBuf {
	gen_path.join("pegboard-container-runner").to_path_buf()
}

pub fn isolate_v8_runner_path(gen_path: &Path) -> PathBuf {
	gen_path.join("pegboard-isolate-v8-runner").to_path_buf()
}

pub fn image_path(gen_path: &Path) -> PathBuf {
	gen_path.join("pegboard-echo-server").to_path_buf()
}

pub fn js_image_path(gen_path: &Path) -> PathBuf {
	gen_path.join("pegboard-js-echo-server.js").to_path_buf()
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
