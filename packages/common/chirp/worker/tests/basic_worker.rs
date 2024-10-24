// TODO: Add back

// use async_trait::async_trait;
// use proto::rivet::{chirp, common};
// use std::sync::Arc;
// use testcontainers::core::*;
// use tokio::{
// 	sync::{mpsc, Mutex, Notify},
// 	task, time,
// };
// use uuid::Uuid;

// const TEST_NAMESPACE: &str = "test";
// const TEST_REGION: &str = "local";

// #[derive(prost::Message, Clone, PartialEq, Eq)]
// struct TestRequest {
// 	#[prost(int32, tag = "1")]
// 	x: i32,
// }

// #[derive(prost::Message, Clone)]
// struct TestResponse {
// 	#[prost(int32, tag = "1")]
// 	y: i32,
// }

// #[derive(Clone)]
// struct TestWorker {
// 	success_tx: Arc<Mutex<mpsc::Sender<TestRequest>>>,
// }

// #[async_trait]
// impl chirp_worker::Worker for TestWorker {
// 	type Request = TestRequest;
// 	type Response = TestResponse;

// 	async fn handle<'a>(
// 		&self,
// 		req: &chirp_worker::Request<Self::Request>,
// 	) -> Result<Self::Response, chirp_worker::GlobalError>
// 	where
// 		Self::Response: 'a,
// 	{
// 		// Send success
// 		self.success_tx
// 			.lock()
// 			.await
// 			.send((*req).clone())
// 			.await
// 			.expect("send success");

// 		// Send response
// 		Ok(TestResponse { y: req.x * 2 })
// 	}
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn it_can_rpc() {
// 	// Setup logging
// 	tracing_subscriber::fmt()
// 		.pretty()
// 		.with_max_level(tracing::Level::INFO)
// 		.with_target(false)
// 		.without_time()
// 		.init();

// 	// Setup NATS
// 	tracing::info!("starting nats");
// 	let docker = testcontainers::clients::Cli::default();
// 	let container = docker.run(rivet_test_images::Nats::default());
// 	let nats_port = container.get_host_port(21200).expect("get host port");

// 	// Build worker
// 	let (success_tx, mut success_rx) = mpsc::channel::<TestRequest>(1);
// 	let worker = TestWorker {
// 		success_tx: Arc::new(Mutex::new(success_tx)),
// 	};

// 	// Start manager
// 	tracing::info!("spawning manager");
// 	let config = chirp_worker::Config {
// 		nats_url: format!("nats://0.0.0.0:{}", nats_port),
// 		namespace: TEST_NAMESPACE.into(),
// 		region: TEST_REGION.into(),
// 		worker_name: "test-worker".into(),
// 		worker_group: "test".into(),
// 		..Default::default()
// 	};
// 	let manager_config = config.clone();
// 	let ready_notify = Arc::new(Notify::default());
// 	let ready_notify2 = ready_notify.clone();
// 	let shutdown_notify = Arc::new(Notify::default());
// 	let shutdown_notify2 = shutdown_notify.clone();
// 	task::spawn(async move {
// 		// Setup worker and notify on ready
// 		let manager = chirp_worker::Manager::new(manager_config, worker)
// 			.await
// 			.expect("create manager");
// 		ready_notify.notify_one();

// 		// Wait for shutdown notification
// 		let manager_conn = manager.__test_conn();
// 		let start_handle = tokio::task::Builder::new()
// 			.name("chirp-worker::manager_start")
// 			.spawn(manager.start());
// 		tokio::select! {
// 			res = start_handle => {
// 				panic!("manager start finished early: {:?}", res);
// 			}
// 			_ = shutdown_notify2.notified() => {
// 				tokio::task::spawn_blocking(move || manager_conn.close()).await.unwrap();
// 			}
// 		}
// 	});
// 	ready_notify2.notified().await;

// 	// Serialize response
// 	let req_body = TestRequest { x: 5 };
// 	let mut req_body_buf = Vec::with_capacity(prost::Message::encoded_len(&req_body));
// 	prost::Message::encode(&req_body, &mut req_body_buf).expect("encode req body");

// 	let req = chirp::Request {
// 		req_id: Some(common::Uuid {
// 			uuid: Uuid::new_v4().as_bytes().to_vec(),
// 		}),
// 		ray_id: Some(common::Uuid {
// 			uuid: Uuid::new_v4().as_bytes().to_vec(),
// 		}),
// 		body: req_body_buf,
// 	};
// 	let mut req_buf = Vec::with_capacity(prost::Message::encoded_len(&req));
// 	prost::Message::encode(&req, &mut req_buf).expect("encode req");

// 	// Build connection
// 	tracing::info!("connecting to nats");
// 	let nats_url = config.nats_url.clone();
// 	let conn = tokio::task::spawn_blocking(move || nats::connect(&nats_url).expect("connect nats"))
// 		.await
// 		.unwrap();
// 	let subject = chirp_client::endpoint::subject(TEST_NAMESPACE, TEST_REGION, &config.worker_name);

// 	// Send request
// 	tracing::info!(?subject, "sending request");
// 	let msg = {
// 		let conn = conn.clone();
// 		let subject = subject.clone();
// 		tokio::task::spawn_blocking(move || conn.request(&subject, &req_buf).expect("request"))
// 			.awai
// 			.unwrap()
// 	};

// 	// Wait for success
// 	let worker_req = time::timeout(time::Duration::from_secs(5), success_rx.recv())
// 		.await
// 		.expect("timeout")
// 		.expect("success");
// 	assert_eq!(worker_req, req_body, "mismatching req");

// 	let res = <chirp::Response as prost::Message>::decode(msg.data.as_slice()).expect("res decode");
// 	let res_body_buf = match res.kind {
// 		Some(chirp::response::Kind::Ok(v)) => v.body,
// 		Some(chirp::response::Kind::Err(err)) => panic!("response error: {:?}", err),
// 		None => panic!("missing response kind"),
// 	};
// 	let res_body =
// 		<TestResponse as prost::Message>::decode(res_body_buf.as_slice()).expect("res body decode");
// 	assert_eq!(res_body.y, req_body.x * 2, "wrong output");

// 	tracing::info!("complete");
// 	shutdown_notify.notify_one();
// }
