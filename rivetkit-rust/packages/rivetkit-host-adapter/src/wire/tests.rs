//! Wire behaviour with a recording transport: replies, cancellation, leases,
//! stream credit, and call errors. The actor event loop itself needs a real
//! `ActorContext` and is covered end to end by the Java tests.

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use parking_lot::Mutex;

use super::host::Target;
use super::messages::{CancelEvent, ErrorBody, StreamEndEvent, decode, encode};
use super::runtime::error_body;
use super::{Event, EventKind, Status, Transport, Wire, streams};

#[derive(Debug, Clone)]
struct Recorded {
	kind: EventKind,
	id: u64,
	header: Vec<u8>,
	body: Vec<u8>,
	lease: u64,
}

#[derive(Debug, Clone)]
struct Completed {
	token: u64,
	status: Status,
	header: Vec<u8>,
}

#[derive(Default)]
struct Recorder {
	events: Mutex<Vec<Recorded>>,
	completions: Mutex<Vec<Completed>>,
}

struct FakeTransport(Arc<Recorder>);

impl Transport for FakeTransport {
	fn event(&self, event: Event<'_>) {
		self.0.events.lock().push(Recorded {
			kind: event.kind,
			id: event.id,
			header: event.header.to_vec(),
			body: event.body.to_vec(),
			lease: event.lease,
		});
	}

	fn complete(&self, token: u64, status: Status, header: &[u8], _body: &[u8], _lease: u64) {
		self.0.completions.lock().push(Completed {
			token,
			status,
			header: header.to_vec(),
		});
	}
}

fn setup() -> (Arc<Wire>, Arc<Recorder>) {
	let recorder = Arc::new(Recorder::default());
	let wire = Wire::new(
		Box::new(FakeTransport(recorder.clone())),
		tokio::runtime::Handle::current(),
	);
	(wire, recorder)
}

fn target(wire: &Arc<Wire>) -> Target {
	Target {
		wire: wire.clone(),
		actor_type: 1,
		actor: 99,
	}
}

async fn wait_for(mut done: impl FnMut() -> bool) {
	for _ in 0..200 {
		if done() {
			return;
		}
		tokio::time::sleep(Duration::from_millis(5)).await;
	}
	panic!("condition not reached");
}

#[tokio::test]
async fn reply_completes_the_call() {
	let (wire, recorder) = setup();
	let future = target(&wire).call(EventKind::Action, vec![0xa0], vec![0x80]);
	let event = recorder.events.lock()[0].clone();
	assert_eq!(event.kind, EventKind::Action);
	assert_eq!(event.body, vec![0x80]);
	assert_eq!(event.lease, 0, "small bodies are lent");

	wire.reply(event.id, 0, &[0xa0], &[0x05]).unwrap();
	let answer = future.await.unwrap();
	assert_eq!(answer.body, vec![0x05]);
	assert!(wire.reply(event.id, 0, &[], &[]).is_err(), "answered once");
}

#[tokio::test]
async fn dropping_the_call_cancels_the_event() {
	let (wire, recorder) = setup();
	let future = target(&wire).call(EventKind::Action, Vec::new(), Vec::new());
	let event_id = recorder.events.lock()[0].id;
	drop(future);

	let cancel = recorder.events.lock()[1].clone();
	assert_eq!(cancel.kind, EventKind::Cancel);
	let header: CancelEvent = decode(&cancel.header, "cancel").unwrap();
	assert_eq!(header.event, event_id);
	assert!(
		wire.reply(event_id, 0, &[], &[]).is_err(),
		"a cancelled event takes no reply"
	);
	assert_eq!(wire.handle_count(), 0);
}

#[tokio::test]
async fn a_pending_call_is_not_cancelled_while_awaited() {
	let (wire, recorder) = setup();
	let future = target(&wire).call(EventKind::Action, Vec::new(), Vec::new());
	let handle = tokio::spawn(future);
	tokio::time::sleep(Duration::from_millis(20)).await;
	assert_eq!(recorder.events.lock().len(), 1, "no cancel while waiting");
	let event_id = recorder.events.lock()[0].id;
	wire.reply(event_id, 0, &[], &[]).unwrap();
	handle.await.unwrap().unwrap();
	assert_eq!(recorder.events.lock().len(), 1, "no cancel after the reply");
}

#[tokio::test]
async fn error_replies_keep_group_code_and_metadata() {
	let (wire, recorder) = setup();
	let future = target(&wire).call(EventKind::Action, Vec::new(), Vec::new());
	let event_id = recorder.events.lock()[0].id;
	let sent = ErrorBody {
		group: "user".into(),
		code: "empty".into(),
		message: "text must not be blank".into(),
		metadata: Some(serde_json::json!({ "field": "text" })),
	};
	wire.reply(event_id, 1, &encode(&sent), &[]).unwrap();
	let error = future.await.unwrap_err();
	assert_eq!(error_body(&error), sent);
}

#[tokio::test]
async fn large_bodies_are_leased_until_released() {
	let (wire, recorder) = setup();
	let body = vec![7u8; super::LEASE_THRESHOLD + 1];
	wire.emit(EventKind::WsMessage, 1, 99, 5, &[], body.clone());
	let event = recorder.events.lock()[0].clone();
	assert_ne!(event.lease, 0);
	assert_eq!(event.body, body);
	assert_eq!(wire.handle_count(), 1);
	wire.lease_release(event.lease).unwrap();
	assert_eq!(wire.handle_count(), 0);
	assert!(wire.lease_release(event.lease).is_err());
}

#[tokio::test]
async fn streams_wait_for_released_credit() {
	let (wire, recorder) = setup();
	let chunk = Bytes::from(vec![1u8; 64 * 1024]);
	let chunks: Vec<anyhow::Result<Bytes>> = (0..8).map(|_| Ok(chunk.clone())).collect();
	let reserved = streams::reserve(&wire, 99);
	let stream_id = reserved.id;
	streams::start(&wire, 1, reserved, Box::pin(futures::stream::iter(chunks)));

	let chunk_events = |recorder: &Recorder| {
		recorder
			.events
			.lock()
			.iter()
			.filter(|event| event.kind == EventKind::StreamChunk)
			.cloned()
			.collect::<Vec<_>>()
	};
	// The 256 KiB window holds four 64 KiB chunks.
	wait_for(|| chunk_events(&recorder).len() == 4).await;
	tokio::time::sleep(Duration::from_millis(30)).await;
	assert_eq!(
		chunk_events(&recorder).len(),
		4,
		"no credit, no more chunks"
	);

	let first = chunk_events(&recorder)[0].clone();
	assert_eq!(first.id, stream_id);
	assert_ne!(first.lease, 0, "stream chunks are always leased");
	wire.lease_release(first.lease).unwrap();
	wait_for(|| chunk_events(&recorder).len() == 5).await;

	for event in chunk_events(&recorder).into_iter().skip(1) {
		wire.lease_release(event.lease).unwrap();
	}
	wait_for(|| chunk_events(&recorder).len() == 8).await;
	for event in chunk_events(&recorder).into_iter().skip(5) {
		wire.lease_release(event.lease).unwrap();
	}
	wait_for(|| {
		recorder
			.events
			.lock()
			.iter()
			.any(|event| event.kind == EventKind::StreamEnd)
	})
	.await;
	let end = recorder.events.lock().last().cloned().unwrap();
	let end: StreamEndEvent = decode(&end.header, "end").unwrap();
	assert!(end.error.is_none());
	assert_eq!(wire.handle_count(), 0, "stream and leases are gone");
}

#[tokio::test]
async fn releasing_a_stream_stops_it() {
	let (wire, recorder) = setup();
	let reserved = streams::reserve(&wire, 99);
	let stream_id = reserved.id;
	streams::start(&wire, 1, reserved, Box::pin(futures::stream::pending()));
	wire.release(stream_id).unwrap();
	tokio::time::sleep(Duration::from_millis(20)).await;
	assert!(
		recorder.events.lock().is_empty(),
		"a cancelled stream sends no end"
	);
	assert_eq!(wire.handle_count(), 0);
}

#[tokio::test]
async fn failed_streams_end_with_the_error() {
	let (wire, recorder) = setup();
	let reserved = streams::reserve(&wire, 99);
	let source = futures::stream::iter(vec![Err(anyhow::anyhow!("socket reset"))]);
	streams::start(&wire, 1, reserved, Box::pin(source));
	wait_for(|| !recorder.events.lock().is_empty()).await;
	let end = recorder.events.lock()[0].clone();
	assert_eq!(end.kind, EventKind::StreamEnd);
	let end: StreamEndEvent = decode(&end.header, "end").unwrap();
	assert!(end.error.unwrap().message.contains("socket reset"));
}

#[tokio::test]
async fn bad_calls_complete_with_an_error() {
	let (wire, recorder) = setup();
	wire.call(0, 999, &[], &[], 1);
	wire.call(
		12345,
		super::Op::Broadcast as u32,
		&encode(&serde_json::json!({ "name": "x" })),
		&[],
		2,
	);
	wire.call(0, super::Op::ClientNew as u32, &[0xff], &[], 3);
	let completions = recorder.completions.lock().clone();
	assert_eq!(completions.len(), 3);
	for (completion, token) in completions.iter().zip([1, 2, 3]) {
		assert_eq!(completion.token, token);
		assert_eq!(completion.status, Status::Error);
		let error: ErrorBody = decode(&completion.header, "error").unwrap();
		assert!(!error.message.is_empty());
	}
}

#[tokio::test]
async fn actors_and_events_are_not_host_releasable() {
	let (wire, recorder) = setup();
	let _future = target(&wire).call(EventKind::Action, Vec::new(), Vec::new());
	let event_id = recorder.events.lock()[0].id;
	assert!(wire.release(event_id).is_err());
	assert!(wire.release(424242).is_err());
}

#[tokio::test]
async fn stopping_an_actor_sweeps_its_handles() {
	let (wire, _recorder) = setup();
	let reserved = streams::reserve(&wire, 99);
	streams::start(&wire, 1, reserved, Box::pin(futures::stream::pending()));
	let _other = streams::reserve(&wire, 100);
	assert_eq!(wire.handle_count(), 2);
	wire.release_owned(99);
	assert_eq!(
		wire.handle_count(),
		1,
		"only the other actor's handle is left"
	);
}

#[tokio::test]
async fn registry_handles_create_and_stop() {
	let (wire, recorder) = setup();
	wire.call(0, super::Op::RegistryNew as u32, &[], &[], 1);
	let created: super::messages::Created =
		decode(&recorder.completions.lock()[0].header, "created").unwrap();
	wire.call(created.handle, super::Op::Stop as u32, &[], &[], 2);
	assert_eq!(recorder.completions.lock()[1].status, Status::Ok);
	wire.release(created.handle).unwrap();
	assert_eq!(wire.handle_count(), 0);
}

#[tokio::test]
async fn stopping_an_actor_keeps_leases_the_host_still_reads() {
	let (wire, recorder) = setup();
	let body = vec![3u8; super::LEASE_THRESHOLD + 1];
	wire.emit(EventKind::WsMessage, 1, 99, 5, &[], body);
	let lease = recorder.events.lock()[0].lease;
	wire.release_owned(99);
	assert_eq!(wire.handle_count(), 1, "the lease outlives the actor");
	wire.lease_release(lease).unwrap();
	assert_eq!(wire.handle_count(), 0);
}
