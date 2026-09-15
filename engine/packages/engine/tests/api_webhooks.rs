#[path = "common/mod.rs"]
mod common;

use std::collections::HashMap;
use std::sync::{
	Arc,
	atomic::{AtomicU16, Ordering},
};
use std::time::Duration;

use axum::{
	Router,
	body::Bytes,
	extract::State,
	http::{HeaderMap, StatusCode},
	routing::post,
};
use rivet_api_public::webhooks::{
	DeleteQuery, EventsQuery, RetryDeliveryQuery, UpsertQuery, UpsertRequest, WebhookConfig,
	WebhookEvent, WebhookEventType,
};
use serde_json::json;
use tokio::sync::mpsc;

const WEBHOOK_NAME: &str = "test-webhook";

fn config(url: &str) -> WebhookConfig {
	WebhookConfig {
		url: url.to_string(),
		headers: HashMap::new(),
		subscriptions: vec![WebhookEventType::RunnerPoolError],
	}
}

async fn upsert_response(port: u16, namespace: &str, config: WebhookConfig) -> reqwest::Response {
	common::api::public::build_webhooks_upsert_request(
		port,
		WEBHOOK_NAME,
		UpsertQuery {
			namespace: namespace.to_string(),
		},
		UpsertRequest(config),
	)
	.await
	.expect("failed to build upsert request")
	.send()
	.await
	.expect("failed to send upsert request")
}

async fn events_response(port: u16, namespace: &str) -> reqwest::Response {
	common::api::public::build_webhooks_events_request(
		port,
		WEBHOOK_NAME,
		EventsQuery {
			namespace: namespace.to_string(),
			limit: None,
			cursor: None,
		},
	)
	.await
	.expect("failed to build events request")
	.send()
	.await
	.expect("failed to send events request")
}

// MARK: Upsert

#[test]
fn upsert_creates_webhook() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;
		let port = ctx.leader_dc().guard_port();

		common::api::public::webhooks_upsert(
			port,
			WEBHOOK_NAME,
			UpsertQuery {
				namespace: namespace.clone(),
			},
			UpsertRequest(config("http://127.0.0.1:1")),
		)
		.await
		.expect("failed to upsert webhook");

		let events = common::api::public::webhooks_events(
			port,
			WEBHOOK_NAME,
			EventsQuery {
				namespace,
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to get webhook events");

		assert!(events.events.is_empty());
	});
}

#[test]
fn upsert_updates_existing_webhook() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;
		let port = ctx.leader_dc().guard_port();

		for url in ["http://127.0.0.1:1", "http://127.0.0.1:2"] {
			common::api::public::webhooks_upsert(
				port,
				WEBHOOK_NAME,
				UpsertQuery {
					namespace: namespace.clone(),
				},
				UpsertRequest(config(url)),
			)
			.await
			.expect("failed to upsert webhook");
		}
	});
}

// MARK: Validation

#[test]
fn upsert_rejects_invalid_url() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;
		let port = ctx.leader_dc().guard_port();

		let response = upsert_response(port, &namespace, config("not a url")).await;
		common::assert_error_response(response, "invalid").await;

		// A rejected config must not be stored.
		let response = events_response(port, &namespace).await;
		common::assert_error_response(response, "not_found").await;
	});
}

#[test]
fn upsert_rejects_too_many_headers() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;
		let port = ctx.leader_dc().guard_port();

		let mut config = config("http://127.0.0.1:1");
		config.headers = (0..17)
			.map(|i| (format!("x-header-{i}"), "value".to_string()))
			.collect();

		let response = upsert_response(port, &namespace, config).await;
		common::assert_error_response(response, "invalid").await;
	});
}

#[test]
fn upsert_rejects_invalid_header_name() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;
		let port = ctx.leader_dc().guard_port();

		let mut config = config("http://127.0.0.1:1");
		config
			.headers
			.insert("bad header".to_string(), "value".to_string());

		let response = upsert_response(port, &namespace, config).await;
		common::assert_error_response(response, "invalid").await;
	});
}

// MARK: Delete

#[test]
fn delete_removes_webhook() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;
		let port = ctx.leader_dc().guard_port();

		common::api::public::webhooks_upsert(
			port,
			WEBHOOK_NAME,
			UpsertQuery {
				namespace: namespace.clone(),
			},
			UpsertRequest(config("http://127.0.0.1:1")),
		)
		.await
		.expect("failed to upsert webhook");

		common::api::public::webhooks_delete(
			port,
			WEBHOOK_NAME,
			DeleteQuery {
				namespace: namespace.clone(),
			},
		)
		.await
		.expect("failed to delete webhook");

		let response = events_response(port, &namespace).await;
		common::assert_error_response(response, "not_found").await;
	});
}

#[test]
fn events_for_unknown_webhook_is_not_found() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;
		let port = ctx.leader_dc().guard_port();

		let response = events_response(port, &namespace).await;
		common::assert_error_response(response, "not_found").await;
	});
}

// MARK: Delivery

type ReceivedRequest = (HeaderMap, serde_json::Value);

struct Receiver {
	status: AtomicU16,
	tx: mpsc::UnboundedSender<ReceivedRequest>,
}

async fn receive(
	State(state): State<Arc<Receiver>>,
	headers: HeaderMap,
	body: Bytes,
) -> StatusCode {
	let body = serde_json::from_slice(&body).expect("webhook body is not json");
	let _ = state.tx.send((headers, body));
	StatusCode::from_u16(state.status.load(Ordering::SeqCst)).expect("invalid status")
}

async fn spawn_receiver(
	status: StatusCode,
) -> (
	String,
	Arc<Receiver>,
	mpsc::UnboundedReceiver<ReceivedRequest>,
) {
	let (tx, rx) = mpsc::unbounded_channel();
	let state = Arc::new(Receiver {
		status: AtomicU16::new(status.as_u16()),
		tx,
	});
	let app = Router::new()
		.route("/", post(receive))
		.with_state(state.clone());

	let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
		.await
		.expect("failed to bind webhook receiver");
	let url = format!(
		"http://{}/",
		listener
			.local_addr()
			.expect("failed to read receiver address")
	);
	tokio::spawn(async move {
		axum::serve(listener, app).await.expect("receiver error");
	});

	(url, state, rx)
}

async fn trigger(ctx: &common::TestCtx, namespace_id: rivet_util::Id) {
	ctx.leader_dc()
		.workflow_ctx
		.signal(webhook::workflows::webhook::Trigger {
			event: webhook::types::WebhookEvent {
				event_type: webhook::types::WebhookEventType::RunnerPoolError,
				subject: Some("test-runner".to_string()),
				data: json!({ "message": "boom" }),
			},
		})
		.to_workflow::<webhook::workflows::webhook::Workflow>()
		.tag("namespace_id", namespace_id)
		.tag("name", WEBHOOK_NAME)
		.send()
		.await
		.expect("failed to send trigger");
}

// The delivery record is written by the workflow after the receiver responds, and the test has
// nothing to await for that write, so poll the events endpoint until the status shows up.
async fn wait_for_delivery(port: u16, namespace: &str, status: &str) -> WebhookEvent {
	loop {
		let events = common::api::public::webhooks_events(
			port,
			WEBHOOK_NAME,
			EventsQuery {
				namespace: namespace.to_string(),
				limit: None,
				cursor: None,
			},
		)
		.await
		.expect("failed to get webhook events");

		if let Some(event) = events.events.into_iter().find(|e| e.status == status) {
			return event;
		}

		tokio::time::sleep(Duration::from_millis(50)).await;
	}
}

async fn upsert_receiver(port: u16, namespace: &str, url: &str) {
	let mut config = config(url);
	config
		.headers
		.insert("x-test".to_string(), "value".to_string());

	common::api::public::webhooks_upsert(
		port,
		WEBHOOK_NAME,
		UpsertQuery {
			namespace: namespace.to_string(),
		},
		UpsertRequest(config),
	)
	.await
	.expect("failed to upsert webhook");
}

#[test]
fn delivery_sends_cloudevent() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, namespace_id) = common::setup_test_namespace(ctx.leader_dc()).await;
		let port = ctx.leader_dc().guard_port();

		let (url, _state, mut rx) = spawn_receiver(StatusCode::OK).await;
		upsert_receiver(port, &namespace, &url).await;
		trigger(&ctx, namespace_id).await;

		let (headers, body) = rx.recv().await.expect("receiver closed");
		assert_eq!(
			headers.get("content-type").expect("missing content-type"),
			"application/cloudevents+json"
		);
		assert_eq!(
			headers.get("x-test").expect("missing custom header"),
			"value"
		);
		assert_eq!(body["specversion"], "1.0");
		assert_eq!(body["type"], "dev.rivet.runner_pool.error");
		assert_eq!(
			body["source"],
			format!("rivet:webhook:{namespace_id}:{WEBHOOK_NAME}")
		);
		assert_eq!(body["subject"], "test-runner");
		assert_eq!(body["data"], json!({ "message": "boom" }));

		let event = wait_for_delivery(port, &namespace, "succeeded").await;
		assert_eq!(body["id"], event.id);
		assert_eq!(event.event_type, "runner_pool.error");
		assert_eq!(event.attempt_count, 1);
		assert!(event.last_error.is_none());
	});
}

#[test]
fn failed_delivery_can_be_retried() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, namespace_id) = common::setup_test_namespace(ctx.leader_dc()).await;
		let port = ctx.leader_dc().guard_port();

		// A 400 is not retryable, so the delivery fails on its first attempt.
		let (url, state, mut rx) = spawn_receiver(StatusCode::BAD_REQUEST).await;
		upsert_receiver(port, &namespace, &url).await;
		trigger(&ctx, namespace_id).await;

		let (_, first_body) = rx.recv().await.expect("receiver closed");
		let failed = wait_for_delivery(port, &namespace, "failed").await;
		assert_eq!(failed.attempt_count, 1);
		assert!(failed.last_error.is_some());

		state
			.status
			.store(StatusCode::OK.as_u16(), Ordering::SeqCst);

		common::api::public::webhooks_retry_delivery(
			port,
			WEBHOOK_NAME,
			&failed.id,
			RetryDeliveryQuery {
				namespace: namespace.clone(),
			},
		)
		.await
		.expect("failed to retry delivery");

		let (_, retry_body) = rx.recv().await.expect("receiver closed");
		assert_eq!(retry_body["id"], first_body["id"]);

		let succeeded = wait_for_delivery(port, &namespace, "succeeded").await;
		assert_eq!(succeeded.id, failed.id);
	});
}

#[test]
fn retry_unknown_delivery_is_not_found() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;
		let port = ctx.leader_dc().guard_port();

		upsert_receiver(port, &namespace, "http://127.0.0.1:1").await;

		let response = common::api::public::build_webhooks_retry_delivery_request(
			port,
			WEBHOOK_NAME,
			"missing",
			RetryDeliveryQuery {
				namespace: namespace.clone(),
			},
		)
		.await
		.expect("failed to build retry request")
		.send()
		.await
		.expect("failed to send retry request");
		common::assert_error_response(response, "delivery_not_found").await;
	});
}
