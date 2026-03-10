use std::sync::{Arc, Mutex};

mod common;

#[test]
fn reschedule_running_actor() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("test-actor", |_| {
				Box::new(common::test_runner::EchoActor::new())
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"test-actor",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;
		let actor_id = res.actor.actor_id;

		let first_connectable_ts = loop {
			let actor = common::try_get_actor(
				ctx.leader_dc().guard_port(),
				&actor_id.to_string(),
				&namespace,
			)
			.await
			.expect("failed to get actor")
			.expect("actor should exist");

			if let Some(connectable_ts) = actor.connectable_ts {
				break connectable_ts;
			}

			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		};

		common::api::public::actors_reschedule(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::reschedule::ReschedulePath { actor_id },
			common::api_types::actors::reschedule::RescheduleQuery {
				namespace: namespace.clone(),
			},
		)
		.await
		.expect("failed to reschedule actor");

		let actor = loop {
			let actor = common::try_get_actor(
				ctx.leader_dc().guard_port(),
				&actor_id.to_string(),
				&namespace,
			)
			.await
			.expect("failed to get actor")
			.expect("actor should exist");

			if actor
				.connectable_ts
				.is_some_and(|ts| ts > first_connectable_ts)
			{
				break actor;
			}

			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		};

		assert!(
			actor.destroy_ts.is_none(),
			"rescheduled actor should remain alive",
		);
	});
}

#[test]
fn reschedule_bypasses_existing_backoff() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let crash_count = Arc::new(Mutex::new(0));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("crash-recover-actor", move |_| {
				Box::new(common::test_runner::CrashNTimesThenSucceedActor::new(
					1,
					crash_count.clone(),
				))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"crash-recover-actor",
			runner.name(),
			rivet_types::actors::CrashPolicy::Restart,
		)
		.await;
		let actor_id = res.actor.actor_id;

		let scheduled_reschedule_ts = loop {
			let actor = common::try_get_actor(
				ctx.leader_dc().guard_port(),
				&actor_id.to_string(),
				&namespace,
			)
			.await
			.expect("failed to get actor")
			.expect("actor should exist");

			if let Some(reschedule_ts) = actor.reschedule_ts {
				break reschedule_ts;
			}

			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		};

		assert!(
			scheduled_reschedule_ts > rivet_util::timestamp::now(),
			"actor should still be waiting on backoff before manual reschedule",
		);

		common::api::public::actors_reschedule(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::reschedule::ReschedulePath { actor_id },
			common::api_types::actors::reschedule::RescheduleQuery {
				namespace: namespace.clone(),
			},
		)
		.await
		.expect("failed to reschedule actor");

		let actor = loop {
			let actor = common::try_get_actor(
				ctx.leader_dc().guard_port(),
				&actor_id.to_string(),
				&namespace,
			)
			.await
			.expect("failed to get actor")
			.expect("actor should exist");

			if actor.connectable_ts.is_some() {
				break actor;
			}

			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		};

		assert!(
			actor
				.connectable_ts
				.expect("actor should have connectable_ts after recovery")
				< scheduled_reschedule_ts,
			"manual reschedule should bypass the existing backoff window",
		);
	});
}
