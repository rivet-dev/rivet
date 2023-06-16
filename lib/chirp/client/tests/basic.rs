use chirp_client::{
	endpoint::Endpoint,
	message::{MessageSubjectParameter, MessageTopic},
};
use prost::Message;
use proto::rivet::chirp;
use std::{sync::Arc, time::Duration};
use tokio::{sync::Notify, task};

const TEST_REGION: &str = "local-lcl";

#[derive(Clone, PartialEq, prost::Message)]
struct TestRequest {
	#[prost(int32, tag = "1")]
	x: i32,
}

#[derive(Clone, PartialEq, prost::Message)]
struct TestResponse {
	#[prost(int32, tag = "1")]
	y: i32,
}

struct TestEndpoint;

impl Endpoint for TestEndpoint {
	type Request = TestRequest;
	type Response = TestResponse;
	const NAME: &'static str = "test-endpoint";
	const TIMEOUT: std::time::Duration = std::time::Duration::from_millis(5000);
}

#[derive(Clone, PartialEq, prost::Message)]
struct TestMessage {
	#[prost(string, tag = "1")]
	hello: String,
}

impl chirp_client::message::Message for TestMessage {
	const NAME: &'static str = "test-message";
	const PARAMETERS: &'static [MessageSubjectParameter] =
		&[MessageSubjectParameter { wildcard: true }];
	const TOPIC: Option<MessageTopic> = None;
	const TTL: Option<i64> = None;

	const PERF_LABEL_SUBSCRIBE: &'static str = "subscribe-test-message";
	const PERF_LABEL_TAIL: &'static str = "tail-test-message";
	const PERF_LABEL_TAIL_READ: &'static str = "tail-read-test-message";
	const PERF_LABEL_TAIL_ANCHOR: &'static str = "tail-anchor-test-message";
	const PERF_LABEL_TAIL_ALL: &'static str = "tail-all-test-message";
	const PERF_LABEL_WRITE_STREAM: &'static str = "write-stream-test-message";
	const PERF_LABEL_WRITE_TAIL: &'static str = "write-tail-test-message";
	const PERF_LABEL_PUBLISH: &'static str = "publish-test-message";
}

#[tokio::test(flavor = "multi_thread")]
async fn basic_client() {
	// Setup logging
	tracing_subscriber::fmt()
		.pretty()
		.with_max_level(tracing::Level::INFO)
		.with_target(false)
		.without_time()
		.init();

	set_env_vars().await;

	// Build client
	let pools = rivet_pools::from_env("chirp-test").await.unwrap();
	let nats = pools.nats().unwrap();
	let redis_chirp = pools.redis_chirp().unwrap();
	let redis_cache = pools.redis_cache().unwrap();

	let shared_client = chirp_client::SharedClient::new(
		nats.clone(),
		redis_chirp.clone(),
		redis_cache.clone(),
		TEST_REGION.to_owned(),
	);
	let client = shared_client.clone().wrap_new("chirp-test");

	// RPC
	{
		// Send request
		let req_body = TestRequest { x: 5 };
		tracing::info!(?req_body, "sending request");
		let req_body_clone = req_body.clone();
		let rpc_handle = {
			let client = client.clone();
			task::spawn(async move { client.rpc::<TestEndpoint>(None, req_body_clone).await })
		};

		// Read message
		let subject = chirp_client::endpoint::subject(TEST_REGION, TestEndpoint::NAME);
		tracing::info!(?subject, "reading message");
		let msg = {
			let nats = nats.clone();
			tokio::task::Builder::new()
				.name("chirp_client::nats_queue_subscribe")
				.spawn_blocking(move || {
					nats.queue_subscribe(&subject, "test")
						.expect("queue sub")
						.next_timeout(Duration::from_secs(1))
						.expect("recv rpc")
				})
				.await
				.unwrap()
		};
		tracing::info!(?msg, "received msg");

		let req = chirp::Request::decode(msg.data.as_slice()).expect("decode req");
		assert!(msg.reply.is_some(), "missing reply");
		let received_req_body = TestRequest::decode(req.body.as_slice()).expect("decode req body");
		assert_eq!(
			received_req_body, req_body,
			"body doesn't match what's sent"
		);

		// Send response
		let res_body = TestResponse { y: req_body.x * 2 };
		let mut res_body_buf = Vec::with_capacity(prost::Message::encoded_len(&res_body));
		prost::Message::encode(&res_body, &mut res_body_buf).expect("encode res body");
		let res = chirp::Response {
			kind: Some(chirp::response::Kind::Ok(chirp::response::Ok {
				body: res_body_buf,
			})),
		};

		let mut res_buf = Vec::with_capacity(prost::Message::encoded_len(&res));
		prost::Message::encode(&res, &mut res_buf).expect("encode res");

		tokio::task::Builder::new()
			.name("chirp_client::nats_respond")
			.spawn_blocking(move || msg.respond(&res_buf).expect("respond"))
			.await
			.unwrap();

		// Join response
		let rpc_res = rpc_handle.await.expect("join rpc").expect("rpc res");
		assert_eq!(*rpc_res, res_body, "responses don't match");
	}

	// Message
	{
		// Start listening and wait for subscriber to register
		let notify = Arc::new(Notify::new());
		let notify2 = notify.clone();
		let listen_handle = {
			let client = client.clone();
			task::spawn(async move {
				// Create sub
				let sub = client
					.subscribe::<TestMessage>(vec!["parameter".to_owned()])
					.await
					.expect("subscribe");

				// Notify sub created so we can start publishing
				notify.notify_one();

				// Wait for next message
				sub.next().await.expect("next")
			})
		};
		notify2.notified().await;

		// Should be ignored
		client
			.message(
				vec!["other-parameter".to_owned()],
				TestMessage {
					hello: "nobody".to_owned(),
				},
			)
			.await
			.unwrap();

		// Will trigger the message
		client
			.message(
				vec!["parameter".to_owned()],
				TestMessage {
					hello: "world".to_owned(),
				},
			)
			.await
			.unwrap();

		let message = listen_handle.await.expect("join listen");
		assert_eq!(message.hello, "world", "received wrong message");
	}

	// Traced messages
	{
		let client2 = shared_client.wrap_new("chirp-test2");

		// Start listening and wait for subscriber to register
		let notify = Arc::new(Notify::new());
		let notify2 = notify.clone();
		let notify3 = Arc::new(Notify::new());
		let notify4 = notify3.clone();
		let listen_handle = {
			let client = client.clone();
			task::spawn(async move {
				// Create sub
				let sub = client
					.subscribe::<TestMessage>(vec!["parameter".to_owned()])
					.await
					.expect("subscribe");

				// Notify sub created so we can start publishing
				notify.notify_one();

				// Wait for next message
				sub.next().await.expect("next")
			})
		};
		let listen_with_trace_handle = {
			let client = client.clone();
			task::spawn(async move {
				// Create sub
				let sub = client
					.subscribe::<TestMessage>(vec!["parameter".to_owned()])
					.await
					.expect("subscribe");

				// Notify sub created so we can start publishing
				notify3.notify_one();

				// Wait for next message WITH TRACE
				sub.next_with_trace(true).await.expect("next")
			})
		};
		tokio::join!(notify2.notified(), notify4.notified());

		// Should be ignored on client1, comes from different client
		client2
			.message(
				vec!["parameter".to_owned()],
				TestMessage {
					hello: "nobody".to_owned(),
				},
			)
			.await
			.unwrap();

		// Will trigger the message
		client
			.message(
				vec!["parameter".to_owned()],
				TestMessage {
					hello: "world".to_owned(),
				},
			)
			.await
			.unwrap();

		let message = listen_with_trace_handle.await.expect("join listen");
		assert_eq!(message.hello, "world", "received wrong message");

		// Sometimes this assertion will fail because the `world` message is received before the `nobody`
		// message. Not sure why this is happening but the test is still successful if the previous
		// assertion is correct.
		let message = listen_handle.await.expect("join listen");
		assert_eq!(message.hello, "nobody", "received wrong message");
	}
}

// Set test env vars to mimic actual env
async fn set_env_vars() {
	std::env::set_var("RIVET_SOURCE_HASH", "00000000");
	std::env::set_var("RIVET_DOMAIN_MAIN", "127.0.0.1:8080");

	std::env::set_var("CHIRP_SERVICE_NAME", "chirp-test");
	std::env::set_var("CHIRP_REGION", &*TEST_REGION);

	std::env::set_var("NATS_URL", "listen.nats.service.consul");
	std::env::set_var("NATS_USERNAME", "chirp");
	std::env::set_var("NATS_PASSWORD", "password");
	std::env::set_var(
		"REDIS_URL_REDIS_CHIRP",
		"redis://listen.redis.service.consul:6379",
	);
	std::env::set_var(
		"REDIS_URL_REDIS_CACHE",
		"redis://listen.redis.service.consul:6379",
	);
}
