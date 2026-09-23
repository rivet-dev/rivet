use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use tokio::sync::mpsc;

use super::{ResponseBody, ResponseBodyError};

fn completion_counter() -> (Arc<AtomicUsize>, impl FnOnce() + Send + 'static) {
	let count = Arc::new(AtomicUsize::new(0));
	let callback_count = count.clone();
	(count, move || {
		callback_count.fetch_add(1, Ordering::SeqCst);
	})
}

#[tokio::test]
async fn completion_runs_once_at_eof() {
	let (count, callback) = completion_counter();
	let body = ResponseBody::Full(Full::new(Bytes::from_static(b"done"))).with_completion(callback);

	assert_eq!(
		body.collect().await.unwrap().to_bytes(),
		Bytes::from_static(b"done")
	);
	assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn completion_runs_once_on_error() {
	let (tx, rx) = mpsc::channel(1);
	tx.send(Err::<Bytes, ResponseBodyError>("stream failed".into()))
		.await
		.unwrap();
	drop(tx);

	let (count, callback) = completion_counter();
	let mut body = ResponseBody::Channel(rx).with_completion(callback);

	assert!(body.frame().await.unwrap().is_err());
	assert_eq!(count.load(Ordering::SeqCst), 1);
	drop(body);
	assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn completion_runs_once_on_drop() {
	let (_tx, rx) = mpsc::channel(1);
	let (count, callback) = completion_counter();
	let body = ResponseBody::Channel(rx).with_completion(callback);

	drop(body);
	assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn consumption_runs_only_when_data_is_yielded() {
	let (tx, rx) = mpsc::channel(1);
	tx.send(Ok::<Bytes, ResponseBodyError>(Bytes::from_static(b"data")))
		.await
		.unwrap();
	let consumed = Arc::new(AtomicUsize::new(0));
	let callback_consumed = consumed.clone();
	let mut body = ResponseBody::Channel(rx).with_consumption(move |bytes| {
		callback_consumed.fetch_add(bytes, Ordering::SeqCst);
	});

	assert_eq!(consumed.load(Ordering::SeqCst), 0);
	let frame = body
		.frame()
		.await
		.expect("response frame")
		.expect("response data");
	assert_eq!(
		frame.into_data().expect("data frame"),
		Bytes::from_static(b"data")
	);
	assert_eq!(consumed.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn terminal_error_survives_a_full_bounded_channel() {
	let (tx, mut body, terminal) = ResponseBody::channel_with_terminal(1);
	tx.send(Ok(Bytes::from_static(b"queued"))).await.unwrap();
	terminal.fail("stream failed after headers".into());
	drop(tx);

	let frame = body
		.frame()
		.await
		.expect("queued frame")
		.expect("queued data");
	assert_eq!(
		frame.into_data().expect("data frame"),
		Bytes::from_static(b"queued")
	);
	let error = body
		.frame()
		.await
		.expect("terminal frame")
		.expect_err("terminal error");
	assert_eq!(error.to_string(), "stream failed after headers");
	assert!(body.frame().await.is_none());
}

#[tokio::test]
async fn already_expired_deadline_terminates_before_the_first_frame() {
	let mut body = ResponseBody::Full(Full::new(Bytes::from_static(b"must not escape")))
		.with_authorization_deadline(Some(0));

	assert!(body.frame().await.is_none());
}

#[tokio::test(start_paused = true)]
async fn deadline_terminates_and_drops_a_streaming_body() {
	let (tx, rx) = mpsc::channel(1);
	let now = u64::try_from(rivet_util::timestamp::now() / 1_000).unwrap();
	let mut body = ResponseBody::Channel(rx).with_authorization_deadline(Some(now + 1));
	let frame = tokio::spawn(async move { body.frame().await });

	tokio::time::advance(std::time::Duration::from_secs(2)).await;
	assert!(frame.await.unwrap().is_none());
	assert!(tx.is_closed(), "expiration must drop the upstream body");
}

#[tokio::test(start_paused = true)]
async fn deadline_terminates_an_sse_stream() {
	let (_tx, rx) = mpsc::channel(1);
	let now = u64::try_from(rivet_util::timestamp::now() / 1_000).unwrap();
	let mut body = ResponseBody::Channel(rx).with_authorization_deadline(Some(now + 1));
	let frame = tokio::spawn(async move { body.frame().await });

	tokio::time::advance(std::time::Duration::from_secs(2)).await;
	assert!(frame.await.unwrap().is_none());
}

#[tokio::test(start_paused = true)]
async fn missing_deadline_leaves_admin_streams_unbounded() {
	let (tx, rx) = mpsc::channel(1);
	let mut body = ResponseBody::Channel(rx).with_authorization_deadline(None);
	tokio::time::advance(std::time::Duration::from_secs(86_400)).await;
	tx.send(Ok(Bytes::from_static(b"still authorized")))
		.await
		.unwrap();

	let frame = body.frame().await.unwrap().unwrap();
	assert_eq!(
		frame.into_data().unwrap(),
		Bytes::from_static(b"still authorized")
	);
}
