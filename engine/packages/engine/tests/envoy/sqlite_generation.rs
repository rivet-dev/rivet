use anyhow::{Result, bail};
use async_trait::async_trait;
use common::test_envoy::*;
use gas::prelude::*;
use pegboard::keys;
use rivet_envoy_protocol as protocol;
use rivet_util::Id;
use std::{
	sync::{Arc, Mutex},
	time::Duration,
};
use tokio::sync::broadcast;

use super::super::common;

const EMPTY_DB_PAGE_HEADER_PREFIX: [u8; 108] = [
	83, 81, 76, 105, 116, 101, 32, 102, 111, 114, 109, 97, 116, 32, 51, 0, 16, 0, 1, 1, 0, 64, 32,
	32, 0, 0, 0, 3, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 46, 138, 17, 13, 0, 0, 0, 0, 16, 0, 0,
];

fn empty_db_page() -> Vec<u8> {
	let mut page = vec![0; 4096];
	page[..EMPTY_DB_PAGE_HEADER_PREFIX.len()].copy_from_slice(&EMPTY_DB_PAGE_HEADER_PREFIX);
	page
}

struct SleepThenStayAwakeActor {
	ready_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

#[async_trait]
impl Actor for SleepThenStayAwakeActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		if config.generation == 1 {
			config.send_set_alarm(rivet_util::timestamp::now() + 200);
			config.send_sleep_intent();
			if let Some(tx) = self.ready_tx.lock().expect("ready tx lock").take() {
				let _ = tx.send(());
			}
		}

		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"SleepThenStayAwakeActor"
	}
}

async fn wait_for_generation(
	mut lifecycle_rx: broadcast::Receiver<ActorLifecycleEvent>,
	actor_id: &str,
	expected_generation: u32,
) -> Result<()> {
	let actor_id = actor_id.to_string();
	tokio::time::timeout(Duration::from_secs(10), async move {
		loop {
			match lifecycle_rx.recv().await {
				Ok(ActorLifecycleEvent::Started {
					actor_id: id,
					generation,
				}) if id == actor_id && generation == expected_generation => {
					return Ok(());
				}
				Ok(_) => {}
				Err(broadcast::error::RecvError::Lagged(_)) => {}
				Err(broadcast::error::RecvError::Closed) => {
					bail!("lifecycle channel closed");
				}
			}
		}
	})
	.await
	.map_err(|_| anyhow::anyhow!("timed out waiting for generation {expected_generation}"))?
}

async fn seed_empty_db_page(
	envoy: &common::test_envoy::TestEnvoy,
	actor_id: &str,
	generation: u32,
) -> u64 {
	let seed = envoy
		.sqlite_commit(protocol::SqliteCommitRequest {
			actor_id: actor_id.to_string(),
			dirty_pages: vec![protocol::SqliteDirtyPage {
				pgno: 1,
				bytes: empty_db_page(),
			}],
			db_size_pages: 1,
			now_ms: rivet_util::timestamp::now(),
			expected_generation: Some(u64::from(generation)),
			expected_head_txid: None,
		})
		.await
		.expect("seed commit request should complete");
	match seed {
		protocol::SqliteCommitResponse::SqliteCommitOk(ok) => {
			ok.head_txid.expect("seed commit should return a head txid")
		}
		protocol::SqliteCommitResponse::SqliteErrorResponse(error) => {
			panic!("seed commit failed: {}", error.message)
		}
	}
}

async fn wait_for_started_generation(
	mut rx: broadcast::Receiver<ActorLifecycleEvent>,
	actor_id: &str,
) -> u32 {
	tokio::time::timeout(Duration::from_secs(5), async {
		loop {
			match rx.recv().await {
				Ok(ActorLifecycleEvent::Started {
					actor_id: id,
					generation,
				}) if id == actor_id => return generation,
				Ok(_) => {}
				Err(broadcast::error::RecvError::Lagged(_)) => {}
				Err(err) => panic!("actor lifecycle event channel closed: {err}"),
			}
		}
	})
	.await
	.expect("timed out waiting for actor start")
}

async fn insert_pending_start_command(
	dc: &common::TestDatacenter,
	namespace_id: Id,
	envoy_key: String,
	actor_id: String,
	generation: u32,
) -> Result<()> {
	let actor_id = Id::parse(&actor_id)?;
	dc.workflow_ctx
		.udb()?
		.txn("test_engineenvoy_sqlite_generation", |tx| {
			let envoy_key = envoy_key.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace());
				tx.write(
					&keys::envoy::ActorCommandKey::new(
						namespace_id,
						envoy_key,
						actor_id,
						generation,
						i64::MAX / 2,
					),
					protocol::ActorCommandKeyData::CommandStartActor(protocol::CommandStartActor {
						config: protocol::ActorConfig {
							name: "sqlite-generation-actor".to_string(),
							key: None,
							create_ts: rivet_util::timestamp::now(),
							input: None,
						},
						hibernating_requests: Vec::new(),
						preloaded_kv: None,
					}),
				)?;
				Ok(())
			}
		})
		.await
}

fn assert_get_pages_rejected(response: protocol::SqliteGetPagesResponse) {
	match response {
		protocol::SqliteGetPagesResponse::SqliteErrorResponse(error) => {
			assert_generation_error(error.message);
		}
		protocol::SqliteGetPagesResponse::SqliteGetPagesOk(ok) => panic!(
			"stale generation get_pages should be rejected before Depot: got ok with {} pages",
			ok.pages.len()
		),
	}
}

fn assert_commit_rejected(response: protocol::SqliteCommitResponse) {
	match response {
		protocol::SqliteCommitResponse::SqliteErrorResponse(error) => {
			assert_generation_error(error.message);
		}
		protocol::SqliteCommitResponse::SqliteCommitOk(ok) => panic!(
			"stale generation commit should be rejected before Depot: got ok with head_txid={:?}",
			ok.head_txid
		),
	}
}

fn assert_generation_error(message: String) {
	assert!(
		message.contains("actor does not exist")
			|| message.contains("invalid sqlite actor generation"),
		"expected generation rejection, got: {message}"
	);
}

#[test]
#[ignore = "diagnostic reproduction for sqlite corruption plan test 1"]
fn inline_sqlite_read_with_stale_generation_is_rejected_before_depot() {
	common::run(
		common::TestOpts::new(1).with_timeout(30),
		|ctx| async move {
			let dc = ctx.leader_dc();
			let (namespace, _) = common::setup_test_namespace(dc).await;
			let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
			let ready_tx = Arc::new(Mutex::new(Some(ready_tx)));

			let envoy = common::setup_envoy(dc, &namespace, |builder| {
				builder
					.with_version(protocol::PROTOCOL_VERSION.into())
					.with_actor_behavior("sqlite-generation-actor", move |_| {
						Box::new(SleepThenStayAwakeActor {
							ready_tx: ready_tx.clone(),
						})
					})
			})
			.await;
			let lifecycle_rx = envoy.subscribe_lifecycle_events();

			let res = common::create_actor(
				dc.guard_port(),
				&namespace,
				"sqlite-generation-actor",
				envoy.pool_name(),
				rivet_types::actors::CrashPolicy::Sleep,
			)
			.await;
			let actor_id = res.actor.actor_id.to_string();

			ready_rx.await.expect("actor should sleep on generation 1");
			wait_for_generation(lifecycle_rx, &actor_id, 2)
				.await
				.expect("actor should wake as generation 2");

			let response = envoy
				.sqlite_get_pages(protocol::SqliteGetPagesRequest {
					actor_id,
					pgnos: vec![1],
					expected_generation: Some(1),
					expected_head_txid: None,
				})
				.await
				.expect("sqlite get_pages request should receive a response");

			match response {
				protocol::SqliteGetPagesResponse::SqliteErrorResponse(error) => {
					assert_ne!(
						(error.group.as_str(), error.code.as_str()),
						("depot", "database_not_found"),
						"stale generation read reached Depot instead of failing generation validation"
					);
				}
				protocol::SqliteGetPagesResponse::SqliteGetPagesOk(_) => {
					panic!("stale generation read unexpectedly succeeded");
				}
			}
		},
	);
}

#[test]
#[ignore = "diagnostic reproduction for sqlite corruption plan test 6"]
fn remote_sqlite_stale_generation_is_not_allowed_by_pending_start_command() {
	common::run(
		common::TestOpts::new(1).with_timeout(30),
		|ctx| async move {
			let dc = ctx.leader_dc();
			let (namespace, namespace_id) = common::setup_test_namespace(dc).await;
			let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
			let ready_tx = Arc::new(Mutex::new(Some(ready_tx)));

			let envoy = common::setup_envoy(dc, &namespace, |builder| {
				builder
					.with_version(protocol::PROTOCOL_VERSION.into())
					.with_actor_behavior("sqlite-generation-actor", move |_| {
						Box::new(SleepThenStayAwakeActor {
							ready_tx: ready_tx.clone(),
						})
					})
			})
			.await;
			let lifecycle_rx = envoy.subscribe_lifecycle_events();

			let res = common::create_actor(
				dc.guard_port(),
				&namespace,
				"sqlite-generation-actor",
				envoy.pool_name(),
				rivet_types::actors::CrashPolicy::Sleep,
			)
			.await;
			let actor_id = res.actor.actor_id.to_string();

			ready_rx.await.expect("actor should sleep on generation 1");
			wait_for_generation(lifecycle_rx, &actor_id, 2)
				.await
				.expect("actor should wake as generation 2");

			insert_pending_start_command(
				dc,
				namespace_id,
				envoy.envoy_key.clone(),
				actor_id.clone(),
				1,
			)
			.await
			.expect("pending stale start command should be inserted");

			let response = envoy
				.remote_sqlite_execute(protocol::SqliteExecuteRequest {
					namespace_id: namespace,
					actor_id,
					generation: 1,
					sql: "CREATE TABLE IF NOT EXISTS stale_remote_sqlite (id INTEGER);".to_string(),
					params: None,
				})
				.await
				.expect("remote sqlite request should receive a response");

			match response {
				protocol::SqliteExecuteResponse::SqliteErrorResponse(error) => {
					assert_eq!(
						(error.group.as_str(), error.code.as_str()),
						("sqlite", "unknown"),
						"stale generation should fail actor validation before remote SQLite execution"
					);
					assert!(
						error.message.contains("actor does not exist"),
						"unexpected validation error: {error:?}"
					);
				}
				protocol::SqliteExecuteResponse::SqliteExecuteOk(_) => {
					panic!("stale remote SQLite request unexpectedly succeeded");
				}
			}
		},
	);
}

#[test]
#[ignore = "diagnostic reproduction for sqlite corruption plan test 2"]
fn inline_sqlite_commit_with_stale_generation_is_rejected_before_depot() {
	common::run(
		common::TestOpts::new(1).with_timeout(30),
		|ctx| async move {
			let dc = ctx.leader_dc();
			let (namespace, _) = common::setup_test_namespace(dc).await;
			let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
			let ready_tx = Arc::new(Mutex::new(Some(ready_tx)));

			let envoy = common::setup_envoy(dc, &namespace, |builder| {
				builder
					.with_version(protocol::PROTOCOL_VERSION.into())
					.with_actor_behavior("sqlite-generation-actor", move |_| {
						Box::new(SleepThenStayAwakeActor {
							ready_tx: ready_tx.clone(),
						})
					})
			})
			.await;
			let lifecycle_rx = envoy.subscribe_lifecycle_events();

			let res = common::create_actor(
				dc.guard_port(),
				&namespace,
				"sqlite-generation-actor",
				envoy.pool_name(),
				rivet_types::actors::CrashPolicy::Sleep,
			)
			.await;
			let actor_id = res.actor.actor_id.to_string();

			ready_rx.await.expect("actor should sleep on generation 1");
			wait_for_generation(lifecycle_rx, &actor_id, 2)
				.await
				.expect("actor should wake as generation 2");

			let response = envoy
				.sqlite_commit(protocol::SqliteCommitRequest {
					actor_id,
					dirty_pages: Vec::new(),
					db_size_pages: 0,
					now_ms: rivet_util::timestamp::now(),
					expected_generation: Some(1),
					expected_head_txid: None,
				})
				.await
				.expect("sqlite commit request should receive a response");

			match response {
				protocol::SqliteCommitResponse::SqliteErrorResponse(error) => {
					assert_ne!(
						(error.group.as_str(), error.code.as_str()),
						("depot", "head_fence_mismatch"),
						"stale generation commit reached Depot instead of failing generation validation"
					);
				}
				protocol::SqliteCommitResponse::SqliteCommitOk(_) => {
					panic!("stale generation commit unexpectedly succeeded");
				}
			}
		},
	);
}

/// Path B from `.agent/notes/sqlite-corruption-deep-dive-findings.md` (Mechanism B):
/// Two `Conn`s on different envoy WS connections each have their own
/// `actor_dbs` cache. After envoy A warms `Db_A`'s `cache_snapshot.pidx`, envoy B
/// commits to the same shared FDB and advances head. `Db_A`'s in-memory PIDX is
/// now stale w.r.t. the FDB head. A subsequent request through envoy A goes
/// through `validate_actor_generation` (which passes because the global
/// `GenerationKey` did not change), then reuses `Db_A` from `Conn_A.actor_dbs`.
///
/// The load-bearing assertion: a follow-up commit through envoy A with
/// `expected_head_txid` set to the post-B head succeeds even though `Db_A`'s
/// PIDX cache reflects pre-B ownership. This exercises the "(stale bytes,
/// current head_txid)" pair end-to-end through the production pegboard-envoy
/// `actor_db()` path.
#[test]
#[ignore = "diagnostic reproduction for sqlite corruption plan path B (per-conn actor_dbs cache)"]
fn inline_sqlite_stale_warm_cache_rmw_lands_under_matching_head_fence() {
	common::run(
		common::TestOpts::new(1).with_timeout(45),
		|ctx| async move {
			let dc = ctx.leader_dc();
			let (namespace, _) = common::setup_test_namespace(dc).await;

			// Envoy A hosts the actor.
			let envoy_a = common::setup_envoy(dc, &namespace, |builder| {
				builder
					.with_pool_name("stale-warm-cache-pool-a")
					.with_actor_behavior("sqlite-stale-warm-actor", |_| Box::new(EchoActor::new()))
			})
			.await;
			let lifecycle_rx = envoy_a.subscribe_lifecycle_events();

			// Envoy B is a peer WS conn to the same engine. It registers a
			// different pool name so the actor stays hosted on envoy A, but
			// envoy B's `Conn` still owns its own `actor_dbs` cache for the
			// same actor id when SQLite frames originate from envoy B.
			let envoy_b = common::test_envoy::TestEnvoyBuilder::new(&namespace)
				.with_version(1)
				.with_pool_name("stale-warm-cache-pool-b")
				.with_actor_behavior("sqlite-stale-warm-actor", |_| Box::new(EchoActor::new()))
				.build(dc)
				.await
				.expect("failed to build envoy_b");
			common::upsert_normal_runner_config(dc, &namespace, envoy_b.pool_name()).await;
			envoy_b.start().await.expect("failed to start envoy_b");
			envoy_b.wait_ready().await;

			let res = common::create_actor(
				dc.guard_port(),
				&namespace,
				"sqlite-stale-warm-actor",
				envoy_a.pool_name(),
				rivet_types::actors::CrashPolicy::Sleep,
			)
			.await;
			let actor_id = res.actor.actor_id.to_string();
			let generation = wait_for_started_generation(lifecycle_rx, &actor_id).await;
			let gen_u64 = u64::from(generation);

			// Step 1: envoy A seeds page 1 (creates the database in Depot).
			// This populates Conn_A.actor_dbs[actor_id] with Db_A and commits
			// page 1 with the empty-DB header bytes through Db_A.
			let head_after_seed = seed_empty_db_page(&envoy_a, &actor_id, generation).await;

			// Step 2: warm Db_A's PIDX cache by reading page 1 through envoy A.
			// `get_pages` populates `Db_A.cache_snapshot.pidx` with the current
			// PIDX owner for pgno 1.
			let warm_read = envoy_a
				.sqlite_get_pages(protocol::SqliteGetPagesRequest {
					actor_id: actor_id.clone(),
					pgnos: vec![1],
					expected_generation: Some(gen_u64),
					expected_head_txid: Some(head_after_seed),
				})
				.await
				.expect("envoy_a warm read should complete");
			let warm_pages = match warm_read {
				protocol::SqliteGetPagesResponse::SqliteGetPagesOk(ok) => ok,
				protocol::SqliteGetPagesResponse::SqliteErrorResponse(err) => {
					panic!(
						"envoy_a warm read failed: {} ({}/{})",
						err.message, err.group, err.code
					);
				}
			};
			assert_eq!(
				warm_pages.head_txid,
				Some(head_after_seed),
				"warm read should observe seed head"
			);
			assert_eq!(
				warm_pages.pages.len(),
				1,
				"warm read should return one page"
			);

			// Step 3: envoy B commits a NEW page 1 against the shared FDB. This
			// constructs Db_B (a different `Db` instance) and advances head.
			// Db_A's `cache_snapshot.pidx` is not touched.
			let mut page_v2 = empty_db_page();
			// Mutate a non-header byte so the bytes differ from the seed page.
			let mutated_idx = EMPTY_DB_PAGE_HEADER_PREFIX.len() + 16;
			page_v2[mutated_idx] = 0xA5;
			let commit_b_resp = envoy_b
				.sqlite_commit(protocol::SqliteCommitRequest {
					actor_id: actor_id.clone(),
					dirty_pages: vec![protocol::SqliteDirtyPage {
						pgno: 1,
						bytes: page_v2.clone(),
					}],
					db_size_pages: 1,
					now_ms: rivet_util::timestamp::now(),
					expected_generation: Some(gen_u64),
					expected_head_txid: Some(head_after_seed),
				})
				.await
				.expect("envoy_b commit should complete");
			let head_after_b = match commit_b_resp {
				protocol::SqliteCommitResponse::SqliteCommitOk(ok) => ok
					.head_txid
					.expect("envoy_b commit should return a head txid"),
				protocol::SqliteCommitResponse::SqliteErrorResponse(err) => {
					panic!(
						"envoy_b commit failed: {} ({}/{})",
						err.message, err.group, err.code
					);
				}
			};
			assert_ne!(
				head_after_b, head_after_seed,
				"envoy_b commit should advance head past the seed"
			);

			// Step 4: read through envoy A again. Db_A's PIDX cache is still
			// stale; depot returns whatever Db_A's cache plans against the
			// current head. Either response is acceptable here. We only need
			// to learn the current head so the follow-up commit can be
			// matching-fence and to confirm validate_actor_generation does not
			// reject envoy A.
			let stale_read = envoy_a
				.sqlite_get_pages(protocol::SqliteGetPagesRequest {
					actor_id: actor_id.clone(),
					pgnos: vec![1],
					expected_generation: Some(gen_u64),
					expected_head_txid: Some(head_after_b),
				})
				.await
				.expect("envoy_a stale read should complete");
			let stale_read_ok = match stale_read {
				protocol::SqliteGetPagesResponse::SqliteGetPagesOk(ok) => ok,
				protocol::SqliteGetPagesResponse::SqliteErrorResponse(err) => panic!(
					"envoy_a stale read returned an error: {} ({}/{})",
					err.message, err.group, err.code
				),
			};
			let observed_head = stale_read_ok
				.head_txid
				.expect("stale read should report a head txid");

			// Step 5: envoy A commits a new dirty page derived from the stale
			// view, fencing on the head it just observed (which should be the
			// post-B head). The load-bearing assertion: this commit must NOT
			// fence-fail and must NOT be rejected by validate_actor_generation,
			// proving the stale-cache `Db_A` reached Depot under a matching
			// head fence.
			let mut page_v3 = page_v2.clone();
			page_v3[mutated_idx + 1] = 0x5A;
			let stale_commit = envoy_a
				.sqlite_commit(protocol::SqliteCommitRequest {
					actor_id: actor_id.clone(),
					dirty_pages: vec![protocol::SqliteDirtyPage {
						pgno: 1,
						bytes: page_v3,
					}],
					db_size_pages: 1,
					now_ms: rivet_util::timestamp::now(),
					expected_generation: Some(gen_u64),
					expected_head_txid: Some(observed_head),
				})
				.await
				.expect("envoy_a stale commit should complete");

			match stale_commit {
				protocol::SqliteCommitResponse::SqliteCommitOk(ok) => {
					let head_after_a = ok
						.head_txid
						.expect("envoy_a stale commit should return a head txid");
					assert!(
						head_after_a > head_after_b,
						"stale-cache commit should advance head past envoy_b's commit"
					);
					tracing::info!(
						observed_head,
						head_after_b,
						head_after_a,
						"stale-cache RMW commit succeeded under matching head fence"
					);
				}
				protocol::SqliteCommitResponse::SqliteErrorResponse(err) => {
					panic!(
						"expected stale-cache RMW commit to succeed under matching head fence, \
						but got error: {} ({}/{}); observed_head={observed_head}, head_after_b={head_after_b}",
						err.message, err.group, err.code
					);
				}
			}

			envoy_b.shutdown().await;
		},
	);
}

#[test]
fn inline_sqlite_rejects_mismatched_generation_before_depot() {
	common::run(
		common::TestOpts::new(1).with_timeout(30),
		|ctx| async move {
			let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;
			let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
				builder
					.with_actor_behavior("sqlite-generation-actor", |_| Box::new(EchoActor::new()))
			})
			.await;
			let lifecycle_rx = envoy.subscribe_lifecycle_events();

			let res = common::create_actor(
				ctx.leader_dc().guard_port(),
				&namespace,
				"sqlite-generation-actor",
				envoy.pool_name(),
				rivet_types::actors::CrashPolicy::Sleep,
			)
			.await;
			let actor_id = res.actor.actor_id.to_string();
			let generation = wait_for_started_generation(lifecycle_rx, &actor_id).await;
			let head_txid = seed_empty_db_page(&envoy, &actor_id, generation).await;

			let stale_generation = u64::from(generation) + 1;
			let stale_read = envoy
				.sqlite_get_pages(protocol::SqliteGetPagesRequest {
					actor_id: actor_id.clone(),
					pgnos: vec![1],
					expected_generation: Some(stale_generation),
					expected_head_txid: Some(head_txid),
				})
				.await
				.expect("stale get_pages request should receive a response");
			let stale_commit = envoy
				.sqlite_commit(protocol::SqliteCommitRequest {
					actor_id,
					dirty_pages: vec![protocol::SqliteDirtyPage {
						pgno: 1,
						bytes: empty_db_page(),
					}],
					db_size_pages: 1,
					now_ms: rivet_util::timestamp::now(),
					expected_generation: Some(stale_generation),
					expected_head_txid: Some(head_txid),
				})
				.await
				.expect("stale commit request should receive a response");

			assert_get_pages_rejected(stale_read);
			assert_commit_rejected(stale_commit);
		},
	);
}

#[test]
fn inline_sqlite_rejects_stale_generation_with_pending_start_command() {
	common::run(
		common::TestOpts::new(1).with_timeout(30),
		|ctx| async move {
			let (namespace, namespace_id) = common::setup_test_namespace(ctx.leader_dc()).await;
			let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
				builder
					.with_actor_behavior("sqlite-generation-actor", |_| Box::new(EchoActor::new()))
			})
			.await;
			let lifecycle_rx = envoy.subscribe_lifecycle_events();

			let res = common::create_actor(
				ctx.leader_dc().guard_port(),
				&namespace,
				"sqlite-generation-actor",
				envoy.pool_name(),
				rivet_types::actors::CrashPolicy::Sleep,
			)
			.await;
			let actor_id = res.actor.actor_id.to_string();
			let generation = wait_for_started_generation(lifecycle_rx, &actor_id).await;
			let head_txid = seed_empty_db_page(&envoy, &actor_id, generation).await;

			let actor_id_parsed: rivet_util::Id = actor_id.parse().expect("actor id should parse");
			let envoy_key = envoy
				.current_envoy_key()
				.await
				.expect("envoy should be running");
			let db = (*ctx
				.leader_dc()
				.workflow_ctx
				.udb()
				.expect("udb should be configured"))
			.clone();
			db.txn("test_engineenvoy_sqlite_generation", move |tx| {
				let envoy_key = envoy_key.clone();
				async move {
					let tx = tx.with_subspace(pegboard::keys::subspace());
					tx.write(
						&pegboard::keys::envoy::ActorKey::new(
							namespace_id,
							envoy_key.clone(),
							actor_id_parsed,
						),
						generation + 1,
					)?;
					tx.write(
						&pegboard::keys::envoy::ActorCommandKey::new(
							namespace_id,
							envoy_key,
							actor_id_parsed,
							generation,
							99,
						),
						protocol::ActorCommandKeyData::CommandStartActor(
							protocol::CommandStartActor {
								config: protocol::ActorConfig {
									name: "sqlite-generation-actor".to_string(),
									key: None,
									create_ts: rivet_util::timestamp::now(),
									input: None,
								},
								hibernating_requests: Vec::new(),
								preloaded_kv: None,
							},
						),
					)?;
					Ok(())
				}
			})
			.await
			.expect("pending start command should be seeded");

			let stale_read = envoy
				.sqlite_get_pages(protocol::SqliteGetPagesRequest {
					actor_id: actor_id.clone(),
					pgnos: vec![1],
					expected_generation: Some(u64::from(generation)),
					expected_head_txid: Some(head_txid),
				})
				.await
				.expect("stale get_pages request should receive a response");
			let stale_commit = envoy
				.sqlite_commit(protocol::SqliteCommitRequest {
					actor_id,
					dirty_pages: vec![protocol::SqliteDirtyPage {
						pgno: 1,
						bytes: empty_db_page(),
					}],
					db_size_pages: 1,
					now_ms: rivet_util::timestamp::now(),
					expected_generation: Some(u64::from(generation)),
					expected_head_txid: Some(head_txid),
				})
				.await
				.expect("stale commit request should receive a response");

			assert_get_pages_rejected(stale_read);
			assert_commit_rejected(stale_commit);
		},
	);
}
