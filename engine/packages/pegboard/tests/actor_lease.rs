mod common;

use anyhow::Result;
use futures_util::future::try_join_all;
use gas::prelude::*;
use pegboard::{
	actor_lease::{self, Action, Input, Phase},
	keys,
};
use rivet_envoy_protocol as protocol;
use universaldb::prelude::*;

#[tokio::test]
async fn lease_concurrency_interruption_and_generation_fencing() -> Result<()> {
	let deps = common::setup_deps().await?;

	let envoy = common::write_envoy(&deps, util::timestamp::now(), Some(8)).await?;
	let actor_id = Id::new_v1(1);
	deps.pools()
		.udb()?
		.txn("initialize_test_lease", |tx| {
			let envoy = envoy.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace());
				tx.write(
					&keys::envoy::ProtocolVersionKey::new(
						envoy.namespace_id,
						envoy.envoy_key.clone(),
					),
					protocol::PROTOCOL_VERSION,
				)?;
				actor_lease::initialize(
					&tx,
					actor_id,
					envoy.namespace_id,
					envoy.pool_name,
					protocol::ActorConfig {
						name: "lease-test".into(),
						key: None,
						input: None,
						create_ts: 1,
					},
				)
			}
		})
		.await?;

	// Dropping an unpolled acquisition cannot mutate ownership.
	let acquire = Input {
		actor_id,
		action: Action::Acquire,
	};
	drop(actor_lease::commit(deps.config(), deps.pools(), &acquire));
	let read = Input {
		actor_id,
		action: Action::Read,
	};
	assert_eq!(
		actor_lease::commit(deps.config(), deps.pools(), &read)
			.await?
			.generation,
		0
	);

	// All contenders commit, but deliberately never publish. This is the exact persisted
	// state left by process death after the transaction commits and before delivery.
	let leases =
		try_join_all((0..16).map(|_| actor_lease::commit(deps.config(), deps.pools(), &acquire)))
			.await?;
	assert!(
		leases
			.iter()
			.all(|lease| lease.generation == 1 && lease.phase == Phase::Starting)
	);
	let retry = actor_lease::commit(deps.config(), deps.pools(), &acquire).await?;
	assert_eq!(retry.command.unwrap().checkpoint.index, 1);
	let database = deps.pools().udb()?;
	let commands = keys::envoy::read_actor_commands(
		&database,
		envoy.namespace_id,
		&envoy.envoy_key,
		keys::envoy::ACTOR_COMMAND_PAGE_BYTES,
	)
	.await?;
	assert_eq!(commands.len(), 1, "start command survives missing publish");
	assert_eq!(commands[0].0.generation, 1);

	let slots = deps
		.pools()
		.udb()?
		.txn("lease_test_slots", |tx| {
			let envoy = envoy.clone();
			async move {
				tx.with_subspace(keys::subspace())
					.read(
						&keys::envoy::SlotsKey::new(envoy.namespace_id, envoy.envoy_key),
						Serializable,
					)
					.await
			}
		})
		.await?;
	assert_eq!(slots, 1, "concurrent/retried acquires reserve one slot");

	let event = |generation, index, state| Input {
		actor_id,
		action: Action::Events {
			namespace_id: envoy.namespace_id,
			envoy_key: envoy.envoy_key.clone(),
			connection_id: envoy.envoy_conn_id.unwrap(),
			events: vec![protocol::EventWrapper {
				checkpoint: protocol::ActorCheckpoint {
					actor_id: actor_id.to_string(),
					generation,
					index,
				},
				inner: protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate {
					state,
				}),
			}],
		},
	};
	let ready = event(1, 0, protocol::ActorState::ActorStateRunning);
	assert_eq!(
		actor_lease::commit(deps.config(), deps.pools(), &ready)
			.await?
			.phase,
		Phase::Running
	);
	let sleep = Input {
		actor_id,
		action: Action::Sleep,
	};
	assert_eq!(
		actor_lease::commit(deps.config(), deps.pools(), &sleep)
			.await?
			.phase,
		Phase::Stopping
	);
	let waiting = actor_lease::commit(deps.config(), deps.pools(), &acquire).await?;
	assert_eq!(
		waiting.generation, 1,
		"wake racing stop must retain ownership"
	);
	assert_eq!(waiting.phase, Phase::Stopping);
	let stopped = event(
		1,
		1,
		protocol::ActorState::ActorStateStopped(protocol::ActorStateStopped {
			code: protocol::StopCode::Ok,
			message: None,
		}),
	);
	actor_lease::commit(deps.config(), deps.pools(), &stopped).await?;
	actor_lease::commit(deps.config(), deps.pools(), &stopped).await?;
	let next = actor_lease::commit(deps.config(), deps.pools(), &acquire).await?;
	assert_eq!(next.generation, 2);
	let stale = actor_lease::commit(deps.config(), deps.pools(), &stopped).await?;
	assert_eq!(stale.generation, 2);
	assert_eq!(
		stale.phase,
		Phase::Starting,
		"old stop cannot release new owner"
	);

	let wrong_namespace = Input {
		actor_id,
		action: Action::Events {
			namespace_id: Id::new_v1(1),
			envoy_key: envoy.envoy_key.clone(),
			connection_id: envoy.envoy_conn_id.unwrap(),
			events: Vec::new(),
		},
	};
	assert!(
		actor_lease::commit(deps.config(), deps.pools(), &wrong_namespace)
			.await
			.is_err()
	);

	// Expire the owner without deleting data. A separate healthy Envoy must win the
	// next generation, and late events from the expired owner cannot release it.
	let replacement = common::EnvoyFixture {
		envoy_key: "replacement-envoy".into(),
		envoy_conn_id: Some(Id::new_v1(1)),
		last_ping_ts: util::timestamp::now(),
		..envoy.clone()
	};
	common::reregister_envoy(
		&deps,
		&replacement,
		replacement.envoy_conn_id.unwrap(),
		replacement.last_ping_ts,
	)
	.await?;
	deps.pools()
		.udb()?
		.txn("expire_test_lease_owner", |tx| {
			let envoy = envoy.clone();
			let replacement = replacement.clone();
			async move {
				let tx = tx.with_subspace(keys::subspace());
				tx.write(
					&keys::envoy::LastPingTsKey::new(envoy.namespace_id, envoy.envoy_key),
					0,
				)?;
				tx.write(
					&keys::envoy::ProtocolVersionKey::new(
						replacement.namespace_id,
						replacement.envoy_key,
					),
					protocol::PROTOCOL_VERSION,
				)?;
				Ok(())
			}
		})
		.await?;
	let failover = actor_lease::commit(deps.config(), deps.pools(), &acquire).await?;
	assert_eq!(failover.generation, 3);
	assert_eq!(failover.envoy_key.as_deref(), Some("replacement-envoy"));
	let old_ready = event(2, 10, protocol::ActorState::ActorStateRunning);
	assert_eq!(
		actor_lease::commit(deps.config(), deps.pools(), &old_ready)
			.await?
			.phase,
		Phase::Starting
	);

	let destroy = Input {
		actor_id,
		action: Action::Destroy,
	};
	actor_lease::commit(deps.config(), deps.pools(), &destroy).await?;
	assert!(
		actor_lease::commit(deps.config(), deps.pools(), &acquire)
			.await
			.is_err()
	);
	Ok(())
}
