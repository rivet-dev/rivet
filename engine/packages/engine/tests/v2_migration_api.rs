//! The `/migration/v1/` API against a real `pegboard_actor2` workflow.
//!
//! The park moves gasoline wake rows that the API packs itself, because gasoline's key types are
//! private. These tests read the raw gasoline keys a real sleeping actor2 workflow wrote, so a
//! change to gasoline's layout fails here instead of silently leaving parked actors wakeable.

use std::{
	sync::{Arc, Mutex},
	time::Duration,
};

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use base64::Engine as _;
use common::test_envoy::*;
use futures_util::TryStreamExt;
use gas::prelude::*;
use rivet_api_public::migration::keys as mkeys;
use rivet_envoy_protocol as protocol;
use tokio::sync::broadcast;
use universaldb::{options::StreamingMode, utils::IsolationLevel::Serializable};
use v2_migration_protocol as proto;

#[path = "common/mod.rs"]
mod common;

const ACTOR_NAME: &str = "migration-actor";

/// A value larger than one 10 KB KV chunk, so export has to reassemble chunks.
fn large_value() -> Vec<u8> {
	(0..25_000u32).map(|i| (i % 251) as u8).collect()
}

fn seeded_kv() -> Vec<(Vec<u8>, Vec<u8>)> {
	vec![
		(b"a".to_vec(), b"first".to_vec()),
		(b"b/large".to_vec(), large_value()),
		(vec![0x01, 0x00, 0xFF], vec![]),
		(b"z".to_vec(), b"last".to_vec()),
	]
}

const EMPTY_DB_PAGE_HEADER_PREFIX: [u8; 108] = [
	83, 81, 76, 105, 116, 101, 32, 102, 111, 114, 109, 97, 116, 32, 51, 0, 16, 0, 1, 1, 0, 64, 32,
	32, 0, 0, 0, 3, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 46, 138, 17, 13, 0, 0, 0, 0, 16, 0, 0,
];

/// Page one carries a SQLite header whose database size field says three pages; the other
/// pages are filler that depot stores opaquely.
fn page(pgno: u8) -> Vec<u8> {
	if pgno == 1 {
		let mut page = vec![0; 4096];
		page[..EMPTY_DB_PAGE_HEADER_PREFIX.len()].copy_from_slice(&EMPTY_DB_PAGE_HEADER_PREFIX);
		page[28..32].copy_from_slice(&3u32.to_be_bytes());
		return page;
	}

	vec![pgno; 4096]
}

/// Seeds KV on its first start, hands its config to the test, and stays awake until the test
/// asks it to sleep. Later generations stay awake and report their start.
struct SeedActor {
	config_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<ActorConfig>>>>,
}

#[async_trait]
impl Actor for SeedActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		if config.generation == 1 {
			let (keys, values): (Vec<_>, Vec<_>) = seeded_kv().into_iter().unzip();
			config.send_kv_put(keys, values).await?;
			if let Some(tx) = self.config_tx.lock().expect("config tx lock").take() {
				let _ = tx.send(config.clone());
			}
		}

		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"SeedActor"
	}
}

struct Seeded {
	namespace: String,
	actor_id: Id,
	workflow_id: Id,
	envoy: common::test_envoy::TestEnvoy,
	lifecycle_rx: broadcast::Receiver<ActorLifecycleEvent>,
	alarm_ts: i64,
	pages: Vec<Vec<u8>>,
}

fn backfill_complete_key() -> Vec<u8> {
	rivet_types::keys::backfill::subspace().pack(&rivet_types::keys::backfill::CompleteKey::new(
		pegboard::workflows::actor_migration_fix_backfill::BACKFILL_NAME,
	))
}

/// Creates an actor with KV and three SQLite pages, arms an alarm, and waits for it to sleep.
async fn seed_sleeping_actor(dc: &common::TestDatacenter, alarm_offset_ms: i64) -> Seeded {
	// The routes refuse every call until bootstrap finishes the actor migration fix backfill.
	wait_for_raw(dc, backfill_complete_key(), true).await;

	let (namespace, _) = common::setup_test_namespace(dc).await;

	let (config_tx, config_rx) = tokio::sync::oneshot::channel();
	let config_tx = Arc::new(Mutex::new(Some(config_tx)));
	let envoy = common::setup_envoy(dc, &namespace, |builder| {
		builder.with_actor_behavior(ACTOR_NAME, move |_| {
			Box::new(SeedActor {
				config_tx: config_tx.clone(),
			})
		})
	})
	.await;
	let lifecycle_rx = envoy.subscribe_lifecycle_events();

	let res = common::create_actor(
		dc.guard_port(),
		&namespace,
		ACTOR_NAME,
		envoy.pool_name(),
		rivet_types::actors::CrashPolicy::Sleep,
	)
	.await;
	let actor_id = res.actor.actor_id;

	let config = tokio::time::timeout(Duration::from_secs(10), config_rx)
		.await
		.expect("actor should start")
		.expect("actor should hand over its config");

	// Commit three SQLite pages while generation 1 runs.
	let pages = vec![page(1), page(2), page(3)];
	let commit = envoy
		.sqlite_commit(protocol::SqliteCommitRequest {
			actor_id: actor_id.to_string(),
			dirty_pages: pages
				.iter()
				.enumerate()
				.map(|(i, bytes)| protocol::SqliteDirtyPage {
					pgno: i as u32 + 1,
					bytes: bytes.clone(),
				})
				.collect(),
			db_size_pages: pages.len() as u32,
			now_ms: rivet_util::timestamp::now(),
			expected_generation: Some(1),
			expected_head_txid: None,
		})
		.await
		.expect("sqlite commit should complete");
	assert!(
		matches!(commit, protocol::SqliteCommitResponse::SqliteCommitOk(_)),
		"sqlite commit failed: {commit:?}"
	);

	let alarm_ts = rivet_util::timestamp::now() + alarm_offset_ms;
	config.send_set_alarm(alarm_ts);
	config.send_sleep_intent();

	common::wait_with_poll(Duration::from_secs(15), Duration::from_millis(50), || {
		let namespace = namespace.clone();
		async move {
			let actor = common::try_get_actor(dc.guard_port(), &actor_id.to_string(), &namespace)
				.await
				.ok()
				.flatten()?;
			actor.sleep_ts.map(|_| ())
		}
	})
	.await
	.expect("actor should sleep");

	let workflow_id = dc
		.workflow_ctx
		.udb()
		.unwrap()
		.txn("test_read_workflow_id", |tx| async move {
			tx.with_subspace(pegboard::keys::subspace())
				.read(
					&pegboard::keys::actor::WorkflowIdKey::new(actor_id),
					Serializable,
				)
				.await
		})
		.await
		.unwrap();

	// The workflow commits its sleep after the actor stops; wait for the worker to let go.
	wait_for_raw(dc, mkeys::worker_id(workflow_id), false).await;

	Seeded {
		namespace,
		actor_id,
		workflow_id,
		envoy,
		lifecycle_rx,
		alarm_ts,
		pages,
	}
}

async fn raw_exists(dc: &common::TestDatacenter, key: Vec<u8>) -> bool {
	dc.workflow_ctx
		.udb()
		.unwrap()
		.txn("test_raw_exists", |tx| {
			let key = key.clone();
			async move { Ok(tx.get(&key, Serializable).await?.is_some()) }
		})
		.await
		.unwrap()
}

async fn raw_subspace_len(
	dc: &common::TestDatacenter,
	subspace: universaldb::utils::Subspace,
) -> usize {
	dc.workflow_ctx
		.udb()
		.unwrap()
		.txn("test_raw_subspace_len", |tx| {
			let subspace = subspace.clone();
			async move {
				let rows = tx
					.get_ranges_keyvalues(
						universaldb::RangeOption {
							mode: StreamingMode::WantAll,
							..(&subspace).into()
						},
						Serializable,
					)
					.try_collect::<Vec<_>>()
					.await?;
				Ok(rows.len())
			}
		})
		.await
		.unwrap()
}

async fn wait_for_raw(dc: &common::TestDatacenter, key: Vec<u8>, present: bool) {
	common::wait_with_poll(Duration::from_secs(10), Duration::from_millis(50), || {
		let key = key.clone();
		async move {
			if raw_exists(dc, key).await == present {
				Some(())
			} else {
				None
			}
		}
	})
	.await
	.expect("raw key did not reach the expected state");
}

async fn read_wake_deadline(dc: &common::TestDatacenter, workflow_id: Id) -> Option<i64> {
	dc.workflow_ctx
		.udb()
		.unwrap()
		.txn("test_read_wake_deadline", |tx| async move {
			Ok(tx
				.get(&mkeys::wake_deadline(workflow_id), Serializable)
				.await?
				.map(|raw| i64::from_be_bytes(raw[..].try_into().unwrap())))
		})
		.await
		.unwrap()
}

async fn read_workflow_name(dc: &common::TestDatacenter, workflow_id: Id) -> String {
	dc.workflow_ctx
		.udb()
		.unwrap()
		.txn("test_read_workflow_name", |tx| async move {
			let raw = tx
				.get(&mkeys::workflow_name(workflow_id), Serializable)
				.await?
				.context("workflow name")?;
			Ok(String::from_utf8(raw.to_vec())?)
		})
		.await
		.unwrap()
}

async fn destroy_ts(dc: &common::TestDatacenter, actor_id: Id) -> Option<i64> {
	dc.workflow_ctx
		.udb()
		.unwrap()
		.txn("test_read_destroy_ts", |tx| async move {
			tx.with_subspace(pegboard::keys::subspace())
				.read_opt(
					&pegboard::keys::actor::DestroyTsKey::new(actor_id),
					Serializable,
				)
				.await
		})
		.await
		.unwrap()
}

// MARK: HTTP helpers

fn url(dc: &common::TestDatacenter, path: &str) -> String {
	format!("http://127.0.0.1:{}/migration/v1/{path}", dc.guard_port())
}

async fn get_bytes(dc: &common::TestDatacenter, path: &str) -> Result<Vec<u8>> {
	let res = reqwest::Client::new()
		.get(url(dc, path))
		.bearer_auth(common::TEST_ADMIN_TOKEN)
		.send()
		.await?;
	let status = res.status();
	let body = res.bytes().await?.to_vec();
	if !status.is_success() {
		bail!(
			"{path} returned {status}: {}",
			String::from_utf8_lossy(&body)
		);
	}
	Ok(body)
}

async fn post<Req: serde::Serialize, Res: serde::de::DeserializeOwned>(
	dc: &common::TestDatacenter,
	path: &str,
	request: &Req,
) -> Result<Res> {
	let res = reqwest::Client::new()
		.post(url(dc, path))
		.bearer_auth(common::TEST_ADMIN_TOKEN)
		.body(proto::encode(request)?)
		.send()
		.await?;
	let status = res.status();
	let body = res.bytes().await?.to_vec();
	if !status.is_success() {
		bail!(
			"{path} returned {status}: {}",
			String::from_utf8_lossy(&body)
		);
	}
	proto::decode(&body)
}

async fn candidate(dc: &common::TestDatacenter, namespace: &str, actor_id: Id) -> proto::Candidate {
	let body = get_bytes(dc, &format!("namespaces/{namespace}/candidates?limit=500"))
		.await
		.unwrap();
	let page = proto::decode::<proto::CandidatesResponse>(&body).unwrap();
	page.candidates
		.into_iter()
		.find(|c| c.actor_id == actor_id.as_bytes())
		.expect("actor should be listed")
}

struct Export {
	header: proto::ExportHeader,
	kv: Vec<proto::KvEntry>,
	pages: Vec<Vec<u8>>,
	end: Option<proto::ExportEnd>,
	abort: Option<proto::AbortReason>,
	digest: Vec<u8>,
}

async fn export(dc: &common::TestDatacenter, namespace: &str, actor_id: Id) -> Export {
	let body = get_bytes(
		dc,
		&format!("actors/{actor_id}/export?namespace={namespace}"),
	)
	.await
	.unwrap();

	let mut out = Export {
		header: proto::ExportHeader {
			generation: 0,
			sleep_ts: 0,
			wake_deadline_ts: None,
			name: String::new(),
			key: None,
			pool_name: String::new(),
			create_ts: 0,
			kv_bytes_estimate: 0,
			sqlite: None,
		},
		kv: Vec::new(),
		pages: Vec::new(),
		end: None,
		abort: None,
		digest: vec![0; 16],
	};

	let mut rest = &body[..];
	while let Some((frame_body, used)) = proto::split_frame(rest).unwrap() {
		match proto::decode_frame(frame_body).unwrap() {
			proto::Frame::ExportHeader(header) => out.header = header,
			proto::Frame::KvBatch(batch) => {
				out.digest = chain(&out.digest, frame_body);
				out.kv.extend(batch.entries);
			}
			proto::Frame::PageBatch(batch) => {
				out.digest = chain(&out.digest, frame_body);
				assert_eq!(batch.first_pgno as usize, out.pages.len() + 1);
				out.pages.extend(batch.pages);
			}
			proto::Frame::ExportFence(_) => {}
			proto::Frame::ExportEnd(end) => out.end = Some(end),
			proto::Frame::ExportAbort(abort) => out.abort = Some(abort.reason),
		}
		rest = &rest[used..];
	}
	assert!(rest.is_empty(), "trailing bytes after the last frame");

	out
}

fn chain(previous: &[u8], frame_body: &[u8]) -> Vec<u8> {
	let mut hasher = xxhash_rust::xxh3::Xxh3::new();
	hasher.update(previous);
	hasher.update(frame_body);
	hasher.digest128().to_be_bytes().to_vec()
}

async fn wait_for_start(
	rx: &mut broadcast::Receiver<ActorLifecycleEvent>,
	actor_id: Id,
	within: Duration,
) -> Option<u32> {
	let actor_id = actor_id.to_string();
	tokio::time::timeout(within, async {
		loop {
			match rx.recv().await {
				Ok(ActorLifecycleEvent::Started {
					actor_id: id,
					generation,
				}) if id == actor_id && generation > 1 => {
					return generation;
				}
				Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
				Err(broadcast::error::RecvError::Closed) => panic!("lifecycle channel closed"),
			}
		}
	})
	.await
	.ok()
}

// MARK: Tests

/// The full protocol on one actor: list, export, cutover, confirm (park), restore.
///
/// The alarm is 45 s out, beyond gasoline's 16 s pull-ahead window plus the confirm's margin, so the workflow commits its
/// sleep instead of waiting in a worker. While parked, the alarm must not fire, which proves the park
/// removed the deadline from gasoline's wake index. After restore the overdue deadline fires and
/// the actor wakes in V2.
#[test]
fn migration_park_and_restore_real_actor2() {
	common::run(
		common::TestOpts::new(1).with_timeout(150),
		|ctx| async move {
			let dc = ctx.leader_dc();
			let mut seeded = seed_sleeping_actor(dc, 45_000).await;
			let actor_id = seeded.actor_id;
			let workflow_id = seeded.workflow_id;
			let namespace = seeded.namespace.clone();

			// Pin gasoline's layout: the sleeping workflow has a deadline, its deadline wake row and a
			// signal listen set, under the names and tuple shapes the API packs.
			let workflow_name = read_workflow_name(dc, workflow_id).await;
			assert_eq!(workflow_name, "pegboard_actor2");
			let deadline = read_wake_deadline(dc, workflow_id)
				.await
				.expect("sleeping actor with an alarm has a wake deadline");
			assert!(
				raw_exists(
					dc,
					mkeys::deadline_wake_row(&workflow_name, workflow_id, deadline)
				)
				.await,
				"deadline wake row should exist at the packed key"
			);
			let listen_rows = raw_subspace_len(dc, mkeys::wake_signal_subspace(workflow_id)).await;
			assert!(listen_rows > 0, "sleeping actor2 listens for signals");
			assert_eq!(
				raw_subspace_len(dc, mkeys::pending_signal_subspace(workflow_id)).await,
				0
			);

			// Candidates carry the raw facts.
			let c = candidate(dc, &namespace, actor_id).await;
			assert_eq!(c.version, 2);
			assert_eq!(c.generation, 1);
			assert!(c.sleep_ts.is_some());
			assert!(!c.allocated && !c.hibernating && !c.sqlite_v1 && !c.parked);
			assert_eq!(c.destroy_ts, None);
			assert_eq!(c.wake_deadline_ts, Some(deadline));
			assert_eq!(c.name, ACTOR_NAME);
			assert_eq!(c.workflow_id, workflow_id.as_bytes());
			assert_eq!(
				c.db_size_pages, 3,
				"candidates report the SQLite head's size"
			);

			// A read by id returns the same facts as the listing, and an unknown id is not found.
			let by_id = get_bytes(
				dc,
				&format!("actors/{actor_id}/candidate?namespace={namespace}"),
			)
			.await
			.unwrap();
			// A read by id carries no listing cursor; every fact matches the listing.
			let by_id = proto::decode::<proto::Candidate>(&by_id).unwrap();
			assert!(by_id.cursor.is_empty());
			assert!(
				!c.cursor.is_empty(),
				"a listed candidate carries its resume cursor"
			);
			assert_eq!(
				proto::Candidate {
					cursor: c.cursor.clone(),
					..by_id
				},
				c
			);

			// Resuming after the candidate's own cursor continues past it.
			let after = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&c.cursor);
			let resumed = get_bytes(
				dc,
				&format!("namespaces/{namespace}/candidates?limit=500&after={after}"),
			)
			.await
			.unwrap();
			let resumed = proto::decode::<proto::CandidatesResponse>(&resumed).unwrap();
			assert!(
				resumed.candidates.iter().all(|r| r.actor_id != c.actor_id),
				"a listing resumed after a candidate's cursor does not repeat it"
			);
			let unknown = Id::new_v1(dc.config.dc_label());
			let missing = get_bytes(
				dc,
				&format!("actors/{unknown}/candidate?namespace={namespace}"),
			)
			.await
			.unwrap_err();
			assert!(
				missing.to_string().contains("actor_not_found"),
				"unknown actor: {missing:#}"
			);

			// Export returns KV in key order, reassembled, and the SQLite pages, with a matching digest.
			let exported = export(dc, &namespace, actor_id).await;
			assert_eq!(exported.abort, None);
			let mut expected_kv = seeded_kv();
			expected_kv.sort();
			let got_kv = exported
				.kv
				.iter()
				.map(|e| (e.key.clone(), e.value.clone()))
				.collect::<Vec<_>>();
			assert_eq!(got_kv, expected_kv);
			assert_eq!(exported.pages, seeded.pages);
			let sqlite = exported.header.sqlite.clone().expect("actor has sqlite");
			assert_eq!(sqlite.db_size_pages, 3);
			assert_eq!(sqlite.page_size, 4096);
			let end = exported.end.expect("export should end");
			assert_eq!(end.digest, exported.digest);
			assert_eq!(end.kv_entries, expected_kv.len() as u64);
			assert_eq!(end.pages, 3);
			assert_eq!(exported.header.generation, 1);

			// Cutover is idempotent by cutoverTs, and a stale generation is refused.
			let cutover_ts = rivet_util::timestamp::now();
			let stale: proto::CutoverResponse = post(
				dc,
				&format!("actors/{actor_id}/cutover?namespace={namespace}"),
				&proto::CutoverRequest {
					generation: 7,
					cutover_ts,
				},
			)
			.await
			.unwrap();
			assert_eq!(stale, proto::CutoverResponse::Woke);
			for expected in [
				proto::CutoverResponse::Cut,
				proto::CutoverResponse::AlreadyCut,
			] {
				let res: proto::CutoverResponse = post(
					dc,
					&format!("actors/{actor_id}/cutover?namespace={namespace}"),
					&proto::CutoverRequest {
						generation: 1,
						cutover_ts,
					},
				)
				.await
				.unwrap();
				assert_eq!(res, expected);
			}
			assert_eq!(destroy_ts(dc, actor_id).await, Some(cutover_ts));

			// The gateway lookup now sees a destroyed actor and will not wake it.
			let gateway = dc
				.workflow_ctx
				.op(pegboard::ops::actor::get_for_gateway::Input { actor_id })
				.await
				.unwrap()
				.unwrap();
			assert!(gateway.destroyed);

			// Confirm parks: the wake rows move into the parked record. A retry is idempotent.
			let confirm = proto::ConfirmRequest {
				generation: 1,
				sleep_ts: c.sleep_ts.unwrap(),
				wake_deadline_ts: c.wake_deadline_ts,
				cutover_ts,
				horizon_ms: 0,
				v3_actor_id: actor_id.as_bytes(),
			};
			for expected in [
				proto::ConfirmResponse::Confirmed,
				proto::ConfirmResponse::AlreadyConfirmed,
			] {
				let res: proto::ConfirmResponse = post(
					dc,
					&format!("actors/{actor_id}/confirm?namespace={namespace}"),
					&confirm,
				)
				.await
				.unwrap();
				assert_eq!(res, expected);
			}
			assert!(raw_exists(dc, mkeys::parked(actor_id)).await);
			assert!(
				raw_exists(dc, mkeys::silence_ts(workflow_id)).await,
				"park silences the workflow"
			);
			assert_eq!(read_wake_deadline(dc, workflow_id).await, None);
			assert!(
				!raw_exists(
					dc,
					mkeys::deadline_wake_row(&workflow_name, workflow_id, deadline)
				)
				.await
			);
			assert_eq!(
				raw_subspace_len(dc, mkeys::wake_signal_subspace(workflow_id)).await,
				0
			);
			assert!(candidate(dc, &namespace, actor_id).await.parked);

			// The alarm passes while parked and the actor must not wake.
			let until_alarm = (seeded.alarm_ts - rivet_util::timestamp::now()).max(0) as u64;
			assert_eq!(
				wait_for_start(
					&mut seeded.lifecycle_rx,
					actor_id,
					Duration::from_millis(until_alarm + 3_000)
				)
				.await,
				None,
				"a parked actor must not wake on its alarm"
			);

			// Restore puts the rows back and clears the cutover. The overdue deadline then fires.
			let res: proto::RestoreResponse = post(
				dc,
				&format!("actors/{actor_id}/restore?namespace={namespace}"),
				&proto::RestoreRequest {
					cutover_ts: Some(cutover_ts),
					v3_actor_id: Some(actor_id.as_bytes()),
				},
			)
			.await
			.unwrap();
			assert_eq!(res, proto::RestoreResponse::Restored);
			assert!(!raw_exists(dc, mkeys::parked(actor_id)).await);
			assert!(!raw_exists(dc, mkeys::silence_ts(workflow_id)).await);
			assert_eq!(destroy_ts(dc, actor_id).await, None);

			// A late duplicate of the original cutover must not cut the restored actor again.
			let late: proto::CutoverResponse = post(
				dc,
				&format!("actors/{actor_id}/cutover?namespace={namespace}"),
				&proto::CutoverRequest {
					generation: 1,
					cutover_ts,
				},
			)
			.await
			.unwrap();
			assert_eq!(late, proto::CutoverResponse::Woke);
			assert_eq!(destroy_ts(dc, actor_id).await, None);

			let woke =
				wait_for_start(&mut seeded.lifecycle_rx, actor_id, Duration::from_secs(15)).await;
			assert_eq!(woke, Some(2), "restored actor wakes on its overdue alarm");

			// A second restore finds nothing to do.
			let res: proto::RestoreResponse = post(
				dc,
				&format!("actors/{actor_id}/restore?namespace={namespace}"),
				&proto::RestoreRequest {
					cutover_ts: Some(cutover_ts),
					v3_actor_id: Some(actor_id.as_bytes()),
				},
			)
			.await
			.unwrap();
			assert_eq!(res, proto::RestoreResponse::NotParked);

			// Restore returned the original data.
			let after = export_running_kv(dc, actor_id).await;
			let mut expected_kv = seeded_kv();
			expected_kv.sort();
			assert_eq!(after, expected_kv);

			drop(seeded.envoy);
		},
	);
}

/// A wake that read the actor before the cutover and lands after it must make confirm un-cut.
#[test]
fn migration_confirm_uncuts_when_a_wake_raced_the_cutover() {
	common::run(
		common::TestOpts::new(1).with_timeout(60),
		|ctx| async move {
			let dc = ctx.leader_dc();
			let mut seeded = seed_sleeping_actor(dc, 3_600_000).await;
			let actor_id = seeded.actor_id;
			let namespace = seeded.namespace.clone();
			let c = candidate(dc, &namespace, actor_id).await;

			let cutover_ts = rivet_util::timestamp::now();
			let res: proto::CutoverResponse = post(
				dc,
				&format!("actors/{actor_id}/cutover?namespace={namespace}"),
				&proto::CutoverRequest {
					generation: 1,
					cutover_ts,
				},
			)
			.await
			.unwrap();
			assert_eq!(res, proto::CutoverResponse::Cut);

			// The in-flight wake: guard read the actor as sleeping before the cutover, then sends Wake.
			dc.workflow_ctx
				.signal(pegboard::workflows::actor2::Wake {})
				.to_workflow_id(seeded.workflow_id)
				.send()
				.await
				.unwrap();

			let res: proto::ConfirmResponse = post(
				dc,
				&format!("actors/{actor_id}/confirm?namespace={namespace}"),
				&proto::ConfirmRequest {
					generation: 1,
					sleep_ts: c.sleep_ts.unwrap(),
					wake_deadline_ts: c.wake_deadline_ts,
					cutover_ts,
					horizon_ms: 0,
					v3_actor_id: actor_id.as_bytes(),
				},
			)
			.await
			.unwrap();
			assert!(
				matches!(res, proto::ConfirmResponse::ConfirmWoke(_)),
				"confirm must see the raced wake, got {res:?}"
			);
			assert_eq!(destroy_ts(dc, actor_id).await, None, "confirm un-cuts");
			assert!(!raw_exists(dc, mkeys::parked(actor_id)).await);

			// The first cutover request arrives late, after the un-cut. It must not cut again.
			let late: proto::CutoverResponse = post(
				dc,
				&format!("actors/{actor_id}/cutover?namespace={namespace}"),
				&proto::CutoverRequest {
					generation: 1,
					cutover_ts,
				},
			)
			.await
			.unwrap();
			assert_eq!(late, proto::CutoverResponse::Woke);
			assert_eq!(destroy_ts(dc, actor_id).await, None);

			// V2 keeps serving the actor: it wakes.
			let woke =
				wait_for_start(&mut seeded.lifecycle_rx, actor_id, Duration::from_secs(15)).await;
			assert_eq!(woke, Some(2));
		},
	);
}

/// A confirm whose deadline is inside the horizon un-cuts, and an export of a running actor
/// aborts with `FenceMoved`.
#[test]
fn migration_refuses_every_route_until_the_migration_fix_backfill_completes() {
	common::run(
		common::TestOpts::new(1).with_timeout(60),
		|ctx| async move {
			let dc = ctx.leader_dc();
			let seeded = seed_sleeping_actor(dc, 3_600_000).await;
			let namespace = seeded.namespace.clone();
			let actor_id = seeded.actor_id;

			// Remove the completion marker to stand in for an engine that has not finished the
			// backfill yet.
			let key = backfill_complete_key();
			let raw = dc
				.workflow_ctx
				.udb()
				.unwrap()
				.txn("test_clear_backfill_complete", |tx| {
					let key = key.clone();
					async move {
						let raw = tx.get(&key, Serializable).await?.context("marker")?;
						tx.clear(&key);
						Ok(raw.to_vec())
					}
				})
				.await
				.unwrap();

			let err = get_bytes(dc, &format!("namespaces/{namespace}/candidates"))
				.await
				.unwrap_err();
			assert!(
				format!("{err:#}").contains("backfill_incomplete"),
				"candidates should be refused: {err:#}"
			);
			let err = post::<_, proto::CutoverResponse>(
				dc,
				&format!("actors/{actor_id}/cutover?namespace={namespace}"),
				&proto::CutoverRequest {
					generation: 1,
					cutover_ts: rivet_util::timestamp::now(),
				},
			)
			.await
			.unwrap_err();
			assert!(
				format!("{err:#}").contains("backfill_incomplete"),
				"cutover should be refused: {err:#}"
			);
			assert_eq!(destroy_ts(dc, actor_id).await, None);

			dc.workflow_ctx
				.udb()
				.unwrap()
				.txn("test_restore_backfill_complete", |tx| {
					let key = key.clone();
					let raw = raw.clone();
					async move {
						tx.set(&key, &raw);
						Ok(())
					}
				})
				.await
				.unwrap();

			candidate(dc, &namespace, actor_id).await;
		},
	);
}

#[test]
fn migration_refuses_running_and_soon_waking_actors() {
	common::run(
		common::TestOpts::new(1).with_timeout(60),
		|ctx| async move {
			let dc = ctx.leader_dc();
			let mut seeded = seed_sleeping_actor(dc, 3_600_000).await;
			let actor_id = seeded.actor_id;
			let namespace = seeded.namespace.clone();
			let c = candidate(dc, &namespace, actor_id).await;

			let cutover_ts = rivet_util::timestamp::now();
			let _: proto::CutoverResponse = post(
				dc,
				&format!("actors/{actor_id}/cutover?namespace={namespace}"),
				&proto::CutoverRequest {
					generation: 1,
					cutover_ts,
				},
			)
			.await
			.unwrap();
			let res: proto::ConfirmResponse = post(
				dc,
				&format!("actors/{actor_id}/confirm?namespace={namespace}"),
				&proto::ConfirmRequest {
					generation: 1,
					sleep_ts: c.sleep_ts.unwrap(),
					wake_deadline_ts: c.wake_deadline_ts,
					cutover_ts,
					horizon_ms: 7_200_000,
					v3_actor_id: actor_id.as_bytes(),
				},
			)
			.await
			.unwrap();
			assert_eq!(
				res,
				proto::ConfirmResponse::ConfirmWoke(proto::ConfirmWoke {
					reason: proto::WokeReason::DeadlineSoon
				})
			);
			assert_eq!(destroy_ts(dc, actor_id).await, None);

			// Wake it and export while it runs.
			dc.workflow_ctx
				.signal(pegboard::workflows::actor2::Wake {})
				.to_workflow_id(seeded.workflow_id)
				.send()
				.await
				.unwrap();
			assert_eq!(
				wait_for_start(&mut seeded.lifecycle_rx, actor_id, Duration::from_secs(15)).await,
				Some(2)
			);
			let exported = export(dc, &namespace, actor_id).await;
			assert_eq!(exported.abort, Some(proto::AbortReason::FenceMoved));
			assert!(exported.kv.is_empty());

			// The cutover refuses a running actor.
			let res: proto::CutoverResponse = post(
				dc,
				&format!("actors/{actor_id}/cutover?namespace={namespace}"),
				&proto::CutoverRequest {
					generation: 2,
					cutover_ts: cutover_ts + 1,
				},
			)
			.await
			.unwrap();
			assert!(
				matches!(res, proto::CutoverResponse::Ineligible(_)),
				"got {res:?}"
			);
		},
	);
}

/// Writes a hibernating request row for `actor_id` pinged at `last_ping_ts`, as a gateway's
/// keepalive does.
async fn write_hibernating_request(
	dc: &common::TestDatacenter,
	actor_id: Id,
	last_ping_ts: i64,
	request_id: [u8; 4],
) {
	dc.workflow_ctx
		.udb()
		.unwrap()
		.txn("test_write_hibernating_request", |tx| async move {
			let tx = tx.with_subspace(pegboard::keys::subspace());
			tx.write(
				&pegboard::keys::actor::HibernatingRequestKey::new(
					actor_id,
					last_ping_ts,
					[1, 2, 3, 4],
					request_id,
				),
				(),
			)?;
			Ok(())
		})
		.await
		.unwrap();
}

/// A hibernating request row the gateway stopped pinging, as one a killed gateway leaves behind,
/// does not hold a sleeping actor in V2; a row pinged within the eligible threshold does.
#[test]
fn migration_counts_only_live_hibernating_requests() {
	common::run(
		common::TestOpts::new(1).with_timeout(60),
		|ctx| async move {
			let dc = ctx.leader_dc();
			let seeded = seed_sleeping_actor(dc, 3_600_000).await;
			let actor_id = seeded.actor_id;
			let namespace = seeded.namespace.clone();
			let threshold = dc
				.workflow_ctx
				.config()
				.pegboard()
				.hibernating_request_eligible_threshold();

			let stale_ts = rivet_util::timestamp::now() - threshold - 60_000;
			write_hibernating_request(dc, actor_id, stale_ts, [0, 0, 0, 1]).await;
			let c = candidate(dc, &namespace, actor_id).await;
			assert!(!c.hibernating, "a stale hibernating request held the actor");

			write_hibernating_request(dc, actor_id, rivet_util::timestamp::now(), [0, 0, 0, 2])
				.await;
			let c = candidate(dc, &namespace, actor_id).await;
			assert!(
				c.hibernating,
				"a live hibernating request did not hold the actor"
			);
		},
	);
}

/// An envoy key left on a sleeping actor whose workflow state holds no allocation, as a lost
/// allocate state change leaves it, does not hold the actor back from migrating.
#[test]
fn migration_ignores_an_envoy_key_the_workflow_state_does_not_hold() {
	common::run(
		common::TestOpts::new(1).with_timeout(60),
		|ctx| async move {
			let dc = ctx.leader_dc();
			let seeded = seed_sleeping_actor(dc, 3_600_000).await;
			let actor_id = seeded.actor_id;
			let namespace = seeded.namespace.clone();

			dc.workflow_ctx
				.udb()
				.unwrap()
				.txn("test_leave_envoy_key", |tx| async move {
					tx.with_subspace(pegboard::keys::subspace()).write(
						&pegboard::keys::actor::EnvoyKeyKey::new(actor_id),
						"stale-envoy".to_string(),
					)?;
					Ok(())
				})
				.await
				.unwrap();

			let c = candidate(dc, &namespace, actor_id).await;
			assert!(
				!c.allocated,
				"a stale envoy key should not read as allocated"
			);

			let res: proto::CutoverResponse = post(
				dc,
				&format!("actors/{actor_id}/cutover?namespace={namespace}"),
				&proto::CutoverRequest {
					generation: 1,
					cutover_ts: rivet_util::timestamp::now(),
				},
			)
			.await
			.unwrap();
			assert_eq!(res, proto::CutoverResponse::Cut);
		},
	);
}

/// Reads the actor's KV directly, as the actor would see it.
async fn export_running_kv(dc: &common::TestDatacenter, actor_id: Id) -> Vec<(Vec<u8>, Vec<u8>)> {
	let udb = dc.workflow_ctx.udb().unwrap();
	let actor = dc
		.workflow_ctx
		.op(pegboard::ops::actor::get::Input {
			actor_ids: vec![actor_id],
			fetch_error: false,
		})
		.await
		.unwrap()
		.actors
		.into_iter()
		.next()
		.unwrap();
	let (keys, values, _) = pegboard::actor_kv::list(
		&udb,
		&pegboard::actor_kv::Recipient {
			actor_id,
			namespace_id: actor.namespace_id,
			name: actor.name,
		},
		rivet_envoy_protocol::KvListQuery::KvListAllQuery,
		false,
		None,
	)
	.await
	.unwrap();
	keys.into_iter().zip(values).collect()
}

// MARK: Parked actor deletion

fn url_v2(dc: &common::TestDatacenter, path: &str) -> String {
	format!("http://127.0.0.1:{}/migration/v2/{path}", dc.guard_port())
}

async fn send_v2(
	dc: &common::TestDatacenter,
	method: reqwest::Method,
	path: &str,
	body: Vec<u8>,
) -> Result<Vec<u8>> {
	let res = reqwest::Client::new()
		.request(method, url_v2(dc, path))
		.bearer_auth(common::TEST_ADMIN_TOKEN)
		.body(body)
		.send()
		.await?;
	let status = res.status();
	let body = res.bytes().await?.to_vec();
	if !status.is_success() {
		bail!(
			"{path} returned {status}: {}",
			String::from_utf8_lossy(&body)
		);
	}
	Ok(body)
}

async fn parked_page(
	dc: &common::TestDatacenter,
	namespace: &str,
	query: &str,
) -> proto::ParkedResponse {
	let body = send_v2(
		dc,
		reqwest::Method::GET,
		&format!("namespaces/{namespace}/parked?{query}"),
		Vec::new(),
	)
	.await
	.unwrap();
	proto::decode(&body).unwrap()
}

async fn delete_cursor(dc: &common::TestDatacenter, namespace: &str) -> Option<Vec<u8>> {
	let body = send_v2(
		dc,
		reqwest::Method::GET,
		&format!("namespaces/{namespace}/parked/delete-cursor"),
		Vec::new(),
	)
	.await
	.unwrap();
	proto::decode::<proto::DeleteCursor>(&body).unwrap().cursor
}

async fn put_delete_cursor(dc: &common::TestDatacenter, namespace: &str, cursor: Option<Vec<u8>>) {
	send_v2(
		dc,
		reqwest::Method::PUT,
		&format!("namespaces/{namespace}/parked/delete-cursor"),
		proto::encode(&proto::DeleteCursor { cursor }).unwrap(),
	)
	.await
	.unwrap();
}

async fn delete_parked(
	dc: &common::TestDatacenter,
	namespace: &str,
	actor_id: Id,
) -> proto::DeleteResponse {
	let body = send_v2(
		dc,
		reqwest::Method::POST,
		&format!("actors/{actor_id}/delete?namespace={namespace}"),
		proto::encode(&proto::DeleteRequest {
			v3_actor_id: actor_id.as_bytes(),
		})
		.unwrap(),
	)
	.await
	.unwrap();
	proto::decode(&body).unwrap()
}

async fn actor_namespace_id(dc: &common::TestDatacenter, actor_id: Id) -> Id {
	dc.workflow_ctx
		.udb()
		.unwrap()
		.txn("test_read_namespace_id", |tx| async move {
			tx.with_subspace(pegboard::keys::subspace())
				.read(
					&pegboard::keys::actor::NamespaceIdKey::new(actor_id),
					Serializable,
				)
				.await
		})
		.await
		.unwrap()
}

/// Cuts over and confirms a seeded actor, so V2 parks it with the actor's own id as its V3 id.
async fn park_seeded(dc: &common::TestDatacenter, seeded: &Seeded) -> i64 {
	let actor_id = seeded.actor_id;
	let namespace = &seeded.namespace;
	let c = candidate(dc, namespace, actor_id).await;
	let cutover_ts = rivet_util::timestamp::now();
	let cut: proto::CutoverResponse = post(
		dc,
		&format!("actors/{actor_id}/cutover?namespace={namespace}"),
		&proto::CutoverRequest {
			generation: 1,
			cutover_ts,
		},
	)
	.await
	.unwrap();
	assert_eq!(cut, proto::CutoverResponse::Cut);
	let confirmed: proto::ConfirmResponse = post(
		dc,
		&format!("actors/{actor_id}/confirm?namespace={namespace}"),
		&proto::ConfirmRequest {
			generation: 1,
			sleep_ts: c.sleep_ts.unwrap(),
			wake_deadline_ts: c.wake_deadline_ts,
			cutover_ts,
			horizon_ms: 0,
			v3_actor_id: actor_id.as_bytes(),
		},
	)
	.await
	.unwrap();
	assert_eq!(confirmed, proto::ConfirmResponse::Confirmed);
	cutover_ts
}

/// The name filter lists only that name's actors, and clamps a cursor from the namespace's whole
/// index order: one before the name's rows starts at its first row, one past them lists nothing.
#[test]
fn migration_lists_candidates_of_one_name() {
	common::run(
		common::TestOpts::new(1).with_timeout(60),
		|ctx| async move {
			let dc = ctx.leader_dc();
			let seeded = seed_sleeping_actor(dc, 3_600_000).await;
			let actor_id = seeded.actor_id;
			let namespace = seeded.namespace.clone();
			let namespace_id = actor_namespace_id(dc, actor_id).await;

			let list = |query: String| {
				let namespace = namespace.clone();
				async move {
					let body = get_bytes(dc, &format!("namespaces/{namespace}/candidates?{query}"))
						.await
						.unwrap();
					proto::decode::<proto::CandidatesResponse>(&body).unwrap()
				}
			};
			let ids = |page: &proto::CandidatesResponse| {
				page.candidates
					.iter()
					.map(|c| c.actor_id.clone())
					.collect::<Vec<_>>()
			};

			let named = list(format!("limit=500&name={ACTOR_NAME}")).await;
			assert_eq!(ids(&named), vec![actor_id.as_bytes()]);
			assert_eq!(named.next, None);
			let other = list("limit=500&name=other-actor".to_string()).await;
			assert!(other.candidates.is_empty());

			// Cursors from rows of other names, as an unfiltered listing would have written.
			let row = |name: &str| {
				pegboard::keys::subspace().pack(&pegboard::keys::ns::ActiveActorKey::new(
					namespace_id,
					name.to_string(),
					0,
					Id::new_v1(dc.config.dc_label()),
				))
			};
			let encode =
				|raw: Vec<u8>| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw);

			let before = list(format!(
				"limit=500&name={ACTOR_NAME}&after={}",
				encode(row("a-actor"))
			))
			.await;
			assert_eq!(
				ids(&before),
				vec![actor_id.as_bytes()],
				"a cursor before the name's rows starts at its first row"
			);

			let past = list(format!(
				"limit=500&name={ACTOR_NAME}&after={}",
				encode(row("z-actor"))
			))
			.await;
			assert!(past.candidates.is_empty());
			assert_eq!(
				past.next, None,
				"a cursor past the name's rows ends the listing"
			);

			// The actor's own cursor still resumes right after it.
			let own = list(format!(
				"limit=500&name={ACTOR_NAME}&after={}",
				encode(named.candidates[0].cursor.clone())
			))
			.await;
			assert!(own.candidates.is_empty());

			drop(seeded.envoy);
		},
	);
}

/// A parked actor is listed, deleted through its own workflow with its KV, and stays listed as
/// deleted. A signal sent while it was parked must not wake it in V2, and restore is refused.
#[test]
fn migration_deletes_a_parked_real_actor2() {
	common::run(
		common::TestOpts::new(1).with_timeout(120),
		|ctx| async move {
			let dc = ctx.leader_dc();
			let mut seeded = seed_sleeping_actor(dc, 3_600_000).await;
			let actor_id = seeded.actor_id;
			let workflow_id = seeded.workflow_id;
			let namespace = seeded.namespace.clone();
			let cutover_ts = park_seeded(dc, &seeded).await;

			// A record without a delete is written as v1, which older V2 binaries read.
			let raw = dc
				.workflow_ctx
				.udb()
				.unwrap()
				.txn("test_read_parked", |tx| async move {
					Ok(tx
						.get(&mkeys::parked(actor_id), Serializable)
						.await?
						.map(|raw| raw.to_vec()))
				})
				.await
				.unwrap()
				.expect("parked record");
			assert_eq!(u16::from_le_bytes([raw[0], raw[1]]), 1);
			let record =
				<v2_migration_protocol::versioned::ParkedRecord as vbare::OwnedVersionedData>::deserialize_with_embedded_version(&raw)
					.unwrap();
			assert_eq!(record.cutover_ts, cutover_ts);
			assert_eq!(
				(record.delete_requested_at, record.deleted_at),
				(None, None)
			);

			// The listing shows the actor in its namespace only.
			let page = parked_page(dc, &namespace, "limit=500").await;
			let listed = page
				.actors
				.iter()
				.find(|a| a.actor_id == actor_id.as_bytes())
				.expect("parked actor should be listed");
			assert_eq!(listed.cutover_ts, cutover_ts);
			assert_eq!(listed.delete_requested_at, None);
			let (other_namespace, _) = common::setup_test_namespace(dc).await;
			let other = parked_page(dc, &other_namespace, "limit=500").await;
			assert!(
				other
					.actors
					.iter()
					.all(|a| a.actor_id != actor_id.as_bytes())
			);

			// The delete cursor round-trips and clears.
			assert_eq!(delete_cursor(dc, &namespace).await, None);
			put_delete_cursor(dc, &namespace, Some(listed.cursor.clone())).await;
			assert_eq!(
				delete_cursor(dc, &namespace).await,
				Some(listed.cursor.clone())
			);
			let after = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&listed.cursor);
			let resumed = parked_page(dc, &namespace, &format!("limit=500&after={after}")).await;
			assert!(
				resumed
					.actors
					.iter()
					.all(|a| a.actor_id != actor_id.as_bytes())
			);
			put_delete_cursor(dc, &namespace, None).await;
			assert_eq!(delete_cursor(dc, &namespace).await, None);

			// A wake sent while parked waits as a pending signal without waking the workflow.
			dc.workflow_ctx
				.signal(pegboard::workflows::actor2::Wake {})
				.to_workflow_id(workflow_id)
				.send()
				.await
				.unwrap();
			assert_eq!(
				raw_subspace_len(dc, mkeys::pending_signal_subspace(workflow_id)).await,
				1
			);

			assert_eq!(
				delete_parked(dc, &namespace, actor_id).await,
				proto::DeleteResponse::Deleting
			);
			assert!(!raw_exists(dc, mkeys::silence_ts(workflow_id)).await);

			common::wait_with_poll(Duration::from_secs(30), Duration::from_millis(200), || {
				let namespace = namespace.clone();
				async move {
					match delete_parked(dc, &namespace, actor_id).await {
						proto::DeleteResponse::Deleted => Some(()),
						proto::DeleteResponse::Deleting => None,
						proto::DeleteResponse::NotParked => panic!("the parked record went away"),
					}
				}
			})
			.await
			.expect("the parked actor should be deleted");

			// The workflow destroyed the actor in V2 without starting it, and cleared its KV.
			assert_eq!(
				wait_for_start(&mut seeded.lifecycle_rx, actor_id, Duration::from_secs(2)).await,
				None,
				"the stray wake must not start the actor"
			);
			assert!(
				raw_subspace_len(dc, mkeys::output_subspace(workflow_id)).await > 0,
				"the workflow completed"
			);
			assert_ne!(destroy_ts(dc, actor_id).await, Some(cutover_ts));
			assert!(export_running_kv(dc, actor_id).await.is_empty());

			// The record stays as the record of the delete, written as v2.
			let page = parked_page(dc, &namespace, "limit=500").await;
			let listed = page
				.actors
				.iter()
				.find(|a| a.actor_id == actor_id.as_bytes())
				.expect("a deleted actor stays listed");
			assert!(listed.delete_requested_at.is_some());
			assert!(listed.deleted_at.is_some());
			assert_eq!(
				delete_parked(dc, &namespace, actor_id).await,
				proto::DeleteResponse::Deleted
			);

			let restore = post::<_, proto::RestoreResponse>(
				dc,
				&format!("actors/{actor_id}/restore?namespace={namespace}"),
				&proto::RestoreRequest {
					cutover_ts: Some(cutover_ts),
					v3_actor_id: Some(actor_id.as_bytes()),
				},
			)
			.await
			.unwrap_err();
			assert!(
				restore.to_string().contains("actor_deleted"),
				"restore after delete: {restore:#}"
			);

			drop(seeded.envoy);
		},
	);
}
