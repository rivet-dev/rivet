//! Phase 1a end-to-end gate. Drives `actions::dispatch` against a real
//! `agent-os-sidecar` binary. Skips when `AGENT_OS_SIDECAR_BIN` is unset
//! (CI/dev environments where the binary isn't built).
//!
//! To run for real:
//! ```sh
//! cargo build -p agent-os-sidecar
//! AGENT_OS_SIDECAR_BIN=$(pwd)/target/debug/agent-os-sidecar \
//!     cargo test -p rivetkit-agent-os --test dispatcher_e2e -- --nocapture
//! ```

use std::io::Cursor;
use std::path::PathBuf;

use agent_os_client::{AgentOs, AgentOsConfig, FileContent};
use rivetkit::Event;
use rivetkit::start::wrap_start;
use rivetkit_agent_os::AgentOsActor;
use rivetkit_core::{ActorContext, ActorEvent, ActorStart, Reply};
use tokio::sync::{mpsc, oneshot};

fn sidecar_available() -> bool {
	if std::env::var("AGENT_OS_SIDECAR_BIN").is_err() {
		let candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.join("../../../target/debug/agent-os-sidecar");
		if candidate.exists() {
			// SAFETY: tests run single-process; env mutation here is fine.
			unsafe {
				std::env::set_var("AGENT_OS_SIDECAR_BIN", candidate);
			}
		}
	}
	std::env::var("AGENT_OS_SIDECAR_BIN")
		.map(|path| PathBuf::from(path).exists())
		.unwrap_or(false)
}

async fn new_vm() -> AgentOs {
	AgentOs::create(AgentOsConfig::default())
		.await
		.expect("create VM against real sidecar")
}

fn encode_args(values: &[serde_json::Value]) -> Vec<u8> {
	let mut buf = Vec::new();
	ciborium::into_writer(values, &mut buf).expect("encode CBOR args");
	buf
}

/// Drive one action through the dispatcher and return the encoded reply
/// bytes (or stringified error).
async fn dispatch_one(vm: &AgentOs, name: &str, args_cbor: Vec<u8>) -> Result<Vec<u8>, String> {
	// Synthesize an ActorEvent::Action and pipe it through a typed
	// Start<AgentOsActor> via wrap_start. This is the canonical
	// canned-events pattern used by the rivetkit integration tests.
	let (reply_tx, reply_rx) = oneshot::channel();
	let action_event = ActorEvent::Action {
		name: name.to_owned(),
		args: args_cbor,
		conn: None,
		reply: Reply::from(reply_tx),
	};
	let (event_tx, event_rx) = mpsc::unbounded_channel::<ActorEvent>();
	event_tx.send(action_event).expect("queue action event");
	drop(event_tx);

	let start = wrap_start::<AgentOsActor>(ActorStart {
		ctx: ActorContext::new("dispatcher-e2e", "agent-os", Vec::new(), "local"),
		input: None,
		snapshot: None,
		hibernated: Vec::new(),
		events: event_rx.into(),
		startup_ready: None,
	})
	.expect("wrap_start");
	let mut events = start.events;

	let event = events.recv().await.expect("recv typed event");
	let action = match event {
		Event::Action(action) => action,
		other => panic!("expected Action event, got {other:?}"),
	};
	rivetkit_agent_os::actions::dispatch(vm, action).await;

	match reply_rx.await.expect("await reply") {
		Ok(bytes) => Ok(bytes),
		Err(error) => Err(error.to_string()),
	}
}

#[tokio::test]
async fn dispatcher_round_trips_read_file_against_real_sidecar() {
	if !sidecar_available() {
		eprintln!("skipping: AGENT_OS_SIDECAR_BIN not present");
		return;
	}

	let vm = new_vm().await;

	// Seed a known file via the raw client (bypass the dispatcher to
	// avoid bootstrapping a writeFile arm we haven't added yet).
	let path = "/home/user/dispatcher-e2e.txt";
	let payload = b"hello world".to_vec();
	vm.write_file(path, FileContent::Bytes(payload.clone()))
		.await
		.expect("seed file");

	// readFile takes a single string arg; TS sends args as a CBOR array.
	let args = encode_args(&[serde_json::json!(path)]);
	let reply_bytes = dispatch_one(&vm, "readFile", args)
		.await
		.expect("dispatch readFile");

	// The dispatcher replies via `action.ok(&ByteBuf)` which wraps the
	// bytes per the rivetkit JSON_COMPAT_UINT8_ARRAY convention.
	// Decode the wrapped intermediate and verify the structure.
	let intermediate: serde_json::Value =
		ciborium::from_reader(Cursor::new(reply_bytes)).expect("decode reply CBOR");
	assert!(
		intermediate.is_array(),
		"expected wrapped Uint8Array, got {intermediate:?}"
	);
	assert_eq!(intermediate[0], "$Uint8Array");
	let base64 = intermediate[1].as_str().expect("base64 element");

	use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
	let decoded = BASE64.decode(base64).expect("decode base64");
	assert_eq!(decoded, payload);

	let _ = vm.shutdown().await;
}

#[tokio::test]
async fn dispatcher_round_trips_write_then_read_file() {
	if !sidecar_available() {
		eprintln!("skipping: AGENT_OS_SIDECAR_BIN not present");
		return;
	}

	let vm = new_vm().await;

	// writeFile via the dispatcher (no direct vm.write_file seeding).
	let path = "/home/user/dispatcher-roundtrip.txt";
	let payload = b"round-trip via writeFile arm".to_vec();
	let write_args = {
		let mut buf = Vec::new();
		let tuple = (
			path.to_owned(),
			serde_bytes::ByteBuf::from(payload.clone()),
		);
		ciborium::into_writer(&tuple, &mut buf).expect("encode writeFile args");
		buf
	};
	let write_reply = dispatch_one(&vm, "writeFile", write_args)
		.await
		.expect("dispatch writeFile");
	// writeFile replies with unit `()` — should encode as a single CBOR null.
	let unit: Option<serde_json::Value> =
		ciborium::from_reader(Cursor::new(write_reply)).ok();
	// We don't assert the exact unit encoding; just that it isn't an error
	// envelope and the read below succeeds.
	let _ = unit;

	// readFile via the dispatcher.
	let read_args = encode_args(&[serde_json::json!(path)]);
	let read_reply = dispatch_one(&vm, "readFile", read_args)
		.await
		.expect("dispatch readFile");

	let intermediate: serde_json::Value =
		ciborium::from_reader(Cursor::new(read_reply)).expect("decode readFile reply");
	assert_eq!(intermediate[0], "$Uint8Array");
	let base64 = intermediate[1].as_str().expect("base64 element");
	use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
	let decoded = BASE64.decode(base64).expect("decode base64");
	assert_eq!(decoded, payload);

	let _ = vm.shutdown().await;
}

#[tokio::test]
async fn dispatcher_returns_not_implemented_for_unknown_action() {
	if !sidecar_available() {
		eprintln!("skipping: AGENT_OS_SIDECAR_BIN not present");
		return;
	}
	let vm = new_vm().await;
	let args = encode_args(&[]);
	let error = dispatch_one(&vm, "definitelyNotAnAction", args)
		.await
		.expect_err("unknown action should error");
	assert!(
		error.contains("not implemented yet") || error.contains("definitelyNotAnAction"),
		"expected not-implemented error, got: {error}"
	);
	let _ = vm.shutdown().await;
}
