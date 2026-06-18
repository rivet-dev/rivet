//! Phase 1b end-to-end gate. Constructs `NapiActorFactory` via the NAPI
//! `from_agent_os` factory method (the same path JS uses) and drives a
//! real action through the inner `CoreActorFactory` against a live
//! agent-os sidecar. Verifies:
//!
//! 1. `from_agent_os` builds a factory whose `start(...)` actually runs
//!    the actor's event loop (not a no-op shell).
//! 2. A `writeFile` -> `readFile` round-trip dispatched through that loop
//!    lands at `actions::dispatch` and replies with the wrapped
//!    `["$Uint8Array", base64]` payload.
//! 3. The factory drains cleanly when the event channel closes.
//!
//! Sidecar-gated: skips when `AGENT_OS_SIDECAR_BIN` is unset.

/// Pure parsing tests — no sidecar required. Phase 3 prep: verifies that
/// the JSON envelope sent by the TS shim actually round-trips through
/// `parse_agent_os_options` into an `AgentOsConfig` with the right fields.
mod parsing {
	use crate::agent_os::{NapiAgentOsOptions, parse_agent_os_options};
	use agent_os_client::{
		AgentOsSidecarConfig, FsPermissions, MountConfig, PatternPermissions, RootFilesystemMode,
	};

	#[test]
	fn parse_threads_software_through_to_agent_os_config() {
		let options = NapiAgentOsOptions {
			config_json: Some(
				r#"{"software":[{"package":"node"},{"package":"python","version":"3.11"}]}"#
					.to_owned(),
			),
			sidecar_binary_path: None,
		};
		let actor_config = parse_agent_os_options(options)
			.expect("parse_agent_os_options ok with non-empty software");
		let agent_os_config = actor_config.build_options();
		assert_eq!(
			agent_os_config.software.len(),
			2,
			"software entries must be preserved across the bridge"
		);
		assert_eq!(agent_os_config.software[0].package, "node");
		assert_eq!(agent_os_config.software[0].version, None);
		assert_eq!(agent_os_config.software[1].package, "python");
		assert_eq!(agent_os_config.software[1].version.as_deref(), Some("3.11"),);
	}

	#[test]
	fn parse_preserves_all_supported_fields() {
		let options = NapiAgentOsOptions {
			config_json: Some(
				r#"{
					"software": [{"package": "coreutils"}],
					"additionalInstructions": "Be terse.",
					"moduleAccessCwd": "/home/user/workspace",
					"loopbackExemptPorts": [9000, 9001],
					"allowedNodeBuiltins": ["fs", "path"],
					"permissions": {
						"fs": "deny",
						"network": "allow"
					},
					"mounts": [{
						"path": "/data",
						"plugin": {
							"id": "host_dir",
							"config": { "hostPath": "/tmp/data" }
						},
						"readOnly": true
					}],
					"rootFilesystem": {
						"mode": "read-only",
						"disableDefaultBaseLayer": true
					},
					"limits": {
						"resources": { "maxProcesses": 5 },
						"http": { "maxFetchResponseBytes": 1024 }
					},
					"sidecar": { "pool": "zid" }
				}"#
				.to_owned(),
			),
			sidecar_binary_path: None,
		};
		let actor_config = parse_agent_os_options(options).expect("parse ok");
		let agent_os_config = actor_config.build_options();
		assert_eq!(agent_os_config.software.len(), 1);
		assert_eq!(
			agent_os_config.additional_instructions.as_deref(),
			Some("Be terse."),
		);
		assert_eq!(
			agent_os_config.module_access_cwd.as_deref(),
			Some("/home/user/workspace"),
		);
		assert_eq!(agent_os_config.loopback_exempt_ports, vec![9000, 9001]);
		assert_eq!(
			agent_os_config.allowed_node_builtins.as_deref(),
			Some(&["fs".to_owned(), "path".to_owned()][..]),
		);
		assert!(matches!(
			agent_os_config
				.permissions
				.as_ref()
				.and_then(|p| p.fs.as_ref()),
			Some(FsPermissions::Mode(agent_os_client::PermissionMode::Deny))
		));
		assert!(matches!(
			agent_os_config
				.permissions
				.as_ref()
				.and_then(|p| p.network.as_ref()),
			Some(PatternPermissions::Mode(
				agent_os_client::PermissionMode::Allow
			))
		));
		assert_eq!(agent_os_config.mounts.len(), 1);
		let MountConfig::Native {
			path,
			plugin,
			read_only,
		} = &agent_os_config.mounts[0]
		else {
			panic!("expected native mount");
		};
		assert_eq!(path, "/data");
		assert_eq!(plugin.id, "host_dir");
		assert_eq!(
			plugin
				.config
				.as_ref()
				.and_then(|config| config.get("hostPath"))
				.and_then(|value| value.as_str()),
			Some("/tmp/data"),
		);
		assert!(*read_only);
		assert_eq!(
			agent_os_config.root_filesystem.mode,
			Some(RootFilesystemMode::ReadOnly)
		);
		assert!(agent_os_config.root_filesystem.disable_default_base_layer);
		assert_eq!(
			agent_os_config
				.limits
				.as_ref()
				.and_then(|limits| limits.resources.as_ref())
				.and_then(|resources| resources.max_processes),
			Some(5)
		);
		assert!(matches!(
			agent_os_config.sidecar,
			Some(AgentOsSidecarConfig::Shared {
				pool: Some(ref pool)
			}) if pool == "zid"
		));
	}

	#[test]
	fn parse_builder_produces_fresh_config_each_call() {
		// AgentOsConfig is non-Clone, so the builder must produce a fresh
		// value per invocation. Each VM bring-up calls the builder again.
		let options = NapiAgentOsOptions {
			config_json: Some(r#"{"software":[{"package":"node"}]}"#.to_owned()),
			sidecar_binary_path: None,
		};
		let actor_config = parse_agent_os_options(options).expect("parse ok");
		let first = actor_config.build_options();
		let second = actor_config.build_options();
		assert_eq!(first.software.len(), 1);
		assert_eq!(second.software.len(), 1);
		assert_eq!(first.software[0].package, second.software[0].package);
	}

	#[test]
	fn empty_config_yields_empty_software_list() {
		let options = NapiAgentOsOptions {
			config_json: Some("{}".to_owned()),
			sidecar_binary_path: None,
		};
		let actor_config = parse_agent_os_options(options).expect("parse ok");
		let agent_os_config = actor_config.build_options();
		assert!(agent_os_config.software.is_empty());
		assert!(agent_os_config.additional_instructions.is_none());
	}
}

mod e2e {
	use std::io::Cursor;
	use std::path::PathBuf;

	use base64::Engine as _;
	use base64::engine::general_purpose::STANDARD as BASE64;
	use rivetkit_core::{ActorContext, ActorEvent, ActorStart, Reply};
	use tokio::sync::{mpsc, oneshot};

	use crate::actor_factory::NapiActorFactory;
	use crate::agent_os::NapiAgentOsOptions;

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

	fn encode_cbor<T: serde::Serialize>(value: &T) -> Vec<u8> {
		let mut buf = Vec::new();
		ciborium::into_writer(value, &mut buf).expect("encode CBOR");
		buf
	}

	#[tokio::test]
	async fn napi_factory_dispatches_write_then_read_file() {
		if !sidecar_available() {
			eprintln!("skipping: AGENT_OS_SIDECAR_BIN not present");
			return;
		}

		// Build the factory the same way JS does: through the NAPI
		// `from_agent_os` static. The underlying Rust fn is callable
		// directly; `#[napi(factory)]` only adds the JS export.
		let napi_factory = NapiActorFactory::from_agent_os(
			NapiAgentOsOptions {
				config_json: Some("{}".to_owned()),
				sidecar_binary_path: None,
			},
			None,
		)
		.expect("from_agent_os ok");
		let core_factory = napi_factory.actor_factory();

		// Queue two actions on the actor's event channel:
		// 1. writeFile(path, bytes)
		// 2. readFile(path) — must return the bytes from step 1.
		let path = "/home/user/napi-factory-roundtrip.txt";
		let payload = b"NAPI factory e2e payload".to_vec();
		let write_args =
			encode_cbor(&(path.to_owned(), serde_bytes::ByteBuf::from(payload.clone())));
		let read_args = encode_cbor(&(path.to_owned(),));

		let (write_reply_tx, write_reply_rx) = oneshot::channel();
		let (read_reply_tx, read_reply_rx) = oneshot::channel();
		let (event_tx, event_rx) = mpsc::unbounded_channel::<ActorEvent>();
		event_tx
			.send(ActorEvent::Action {
				name: "writeFile".to_owned(),
				args: write_args,
				conn: None,
				reply: Reply::from(write_reply_tx),
			})
			.expect("queue writeFile");
		event_tx
			.send(ActorEvent::Action {
				name: "readFile".to_owned(),
				args: read_args,
				conn: None,
				reply: Reply::from(read_reply_tx),
			})
			.expect("queue readFile");

		let (startup_tx, startup_rx) = oneshot::channel();
		let start = ActorStart {
			ctx: ActorContext::new("napi-factory-e2e", "agent-os", Vec::new(), "local"),
			is_new: true,
			input: None,
			snapshot: None,
			hibernated: Vec::new(),
			events: event_rx.into(),
			startup_ready: Some(startup_tx),
		};

		// Spawn the factory's entry future. It drives ensure_vm on the
		// first action, then dispatches both actions in order.
		let factory = core_factory.clone();
		let join = tokio::spawn(async move { factory.start(start).await });

		// Confirm startup before draining replies.
		startup_rx
			.await
			.expect("recv startup signal")
			.expect("startup ok");

		// writeFile replies with `()` — CBOR-encoded as `null` (0xF6).
		// We don't assert the exact bytes; just that the reply arrived
		// without error (the round-trip is verified by readFile below).
		let write_reply = write_reply_rx
			.await
			.expect("recv writeFile reply")
			.expect("writeFile ok");
		assert!(
			!write_reply.is_empty(),
			"writeFile reply should encode at least the unit value"
		);

		// readFile reply: ciborium decode -> ["$Uint8Array", base64].
		let read_reply = read_reply_rx
			.await
			.expect("recv readFile reply")
			.expect("readFile ok");
		let intermediate: serde_json::Value =
			ciborium::from_reader(Cursor::new(&read_reply)).expect("decode readFile reply CBOR");
		assert!(
			intermediate.is_array(),
			"expected wrapped Uint8Array, got {intermediate:?}"
		);
		assert_eq!(intermediate[0], "$Uint8Array");
		let base64 = intermediate[1].as_str().expect("base64 element");
		let decoded = BASE64.decode(base64).expect("decode base64");
		assert_eq!(decoded, payload, "readFile bytes match writeFile bytes");

		// Drain the loop: closing the event channel triggers the loop's
		// shutdown path (shutdown_vm with reason="error").
		drop(event_tx);
		let _ = tokio::time::timeout(std::time::Duration::from_secs(30), join)
			.await
			.expect("factory task joins within 30s")
			.expect("factory task didn't panic");
	}

	#[tokio::test]
	async fn napi_factory_rejects_unknown_action_through_loop() {
		if !sidecar_available() {
			eprintln!("skipping: AGENT_OS_SIDECAR_BIN not present");
			return;
		}

		let napi_factory = NapiActorFactory::from_agent_os(
			NapiAgentOsOptions {
				config_json: Some("{}".to_owned()),
				sidecar_binary_path: None,
			},
			None,
		)
		.expect("from_agent_os ok");
		let core_factory = napi_factory.actor_factory();

		let (reply_tx, reply_rx) = oneshot::channel();
		let (event_tx, event_rx) = mpsc::unbounded_channel::<ActorEvent>();
		let args = encode_cbor(&Vec::<serde_json::Value>::new());
		event_tx
			.send(ActorEvent::Action {
				name: "totallyMadeUp".to_owned(),
				args,
				conn: None,
				reply: Reply::from(reply_tx),
			})
			.expect("queue action");

		let (startup_tx, startup_rx) = oneshot::channel();
		let start = ActorStart {
			ctx: ActorContext::new("napi-factory-unknown", "agent-os", Vec::new(), "local"),
			is_new: true,
			input: None,
			snapshot: None,
			hibernated: Vec::new(),
			events: event_rx.into(),
			startup_ready: Some(startup_tx),
		};

		let factory = core_factory.clone();
		let join = tokio::spawn(async move { factory.start(start).await });
		startup_rx
			.await
			.expect("recv startup signal")
			.expect("startup ok");

		let error = reply_rx
			.await
			.expect("recv reply")
			.expect_err("unknown action should error");
		let msg = error.to_string();
		assert!(
			msg.contains("not implemented yet") || msg.contains("totallyMadeUp"),
			"expected not-implemented error, got: {msg}"
		);

		drop(event_tx);
		let _ = tokio::time::timeout(std::time::Duration::from_secs(30), join)
			.await
			.expect("factory task joins within 30s")
			.expect("factory task didn't panic");
	}
}
