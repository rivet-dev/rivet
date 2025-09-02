mod common;

use std::time::Duration;

#[test]
fn actor_lifecycle_single_dc() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		actor_lifecycle_inner(&ctx, false).await;
	});
}

#[test]
fn actor_lifecycle_multi_dc() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		actor_lifecycle_inner(&ctx, true).await;
	});
}

async fn actor_lifecycle_inner(ctx: &common::TestCtx, multi_dc: bool) {
	let target_dc = if multi_dc {
		// Use follower for testing in multi-DC
		ctx.get_dc(2)
	} else {
		// Use leader for single DC
		ctx.leader_dc()
	};

	let (namespace, _, runner) = common::setup_test_namespace_with_runner(target_dc).await;

	let actor_id = common::create_actor(&namespace, target_dc.guard_port()).await;

	// TODO: This is a race condition. we might need to move this after the guard ping since guard
	// correctly waits for the actor to start.
	tokio::time::sleep(Duration::from_millis(500)).await;

	// Test ping via guard
	let ping_response =
		common::ping_actor_via_guard(ctx.leader_dc().guard_port(), &actor_id, "main").await;
	assert_eq!(ping_response["status"], "ok");

	// Test websocket via guard
	let ws_response =
		common::ping_actor_websocket_via_guard(ctx.leader_dc().guard_port(), &actor_id, "main")
			.await;
	assert_eq!(ws_response["status"], "ok");

	// Validate runner state
	assert!(
		runner.has_actor(&actor_id).await,
		"runner should have the actor"
	);

	// Destroy
	tracing::info!("destroying actor");
	tokio::time::sleep(Duration::from_millis(500)).await;
	common::destroy_actor(&actor_id, &namespace, target_dc.guard_port()).await;
	tokio::time::sleep(Duration::from_millis(500)).await;

	// Validate runner state
	assert!(
		!runner.has_actor(&actor_id).await,
		"Runner should not have the actor after destroy"
	);

	runner.shutdown().await;
}

enum DcChoice {
	Leader,
	Follower,
	Both,
}

#[test]
fn actor_lifecycle_with_same_key_single_dc() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		actor_lifecycle_with_same_key_inner(&ctx, DcChoice::Leader).await;
	});
}

#[test]
fn actor_lifecycle_with_same_key_multi_dc() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		actor_lifecycle_with_same_key_inner(&ctx, DcChoice::Follower).await;
	});
}

#[test]
fn actor_lifecycle_with_same_key_different_dc() {
	common::run(common::TestOpts::new(2), |ctx| async move {
		actor_lifecycle_with_same_key_inner(&ctx, DcChoice::Both).await;
	});
}

async fn actor_lifecycle_with_same_key_inner(ctx: &common::TestCtx, dc_choice: DcChoice) {
	let (target_dc1, target_dc2) = match dc_choice {
		DcChoice::Leader => (ctx.leader_dc(), ctx.leader_dc()),
		DcChoice::Follower => (ctx.get_dc(2), ctx.get_dc(2)),
		DcChoice::Both => (ctx.get_dc(2), ctx.leader_dc()),
	};

	let (namespace, _, runner) = common::setup_test_namespace_with_runner(target_dc1).await;
	let key = rand::random::<u16>().to_string();

	let actor_id1 = common::create_actor_with_options(
		common::CreateActorOptions {
			namespace: namespace.clone(),
			key: Some(key.clone()),
			..Default::default()
		},
		target_dc1.guard_port(),
	)
	.await;

	common::assert_actor_in_dc(&actor_id1, target_dc1.config.dc_label()).await;

	// TODO: This is a race condition. we might need to move this after the guard ping since guard
	// correctly waits for the actor to start.
	tokio::time::sleep(Duration::from_millis(500)).await;

	// Test ping via guard
	let ping_response =
		common::ping_actor_via_guard(ctx.leader_dc().guard_port(), &actor_id1, "main").await;
	assert_eq!(ping_response["status"], "ok");

	// Test websocket via guard
	let ws_response =
		common::ping_actor_websocket_via_guard(ctx.leader_dc().guard_port(), &actor_id1, "main")
			.await;
	assert_eq!(ws_response["status"], "ok");

	// Destroy
	tracing::info!("destroying actor");
	tokio::time::sleep(Duration::from_millis(500)).await;
	common::destroy_actor(&actor_id1, &namespace, target_dc1.guard_port()).await;
	tokio::time::sleep(Duration::from_millis(500)).await;

	let actor_id2 = common::create_actor_with_options(
		common::CreateActorOptions {
			namespace: namespace.clone(),
			key: Some(key.clone()),
			..Default::default()
		},
		target_dc2.guard_port(),
	)
	.await;

	assert_ne!(actor_id1, actor_id2, "same actor id");

	common::assert_actor_in_dc(&actor_id2, target_dc1.config.dc_label()).await;

	// TODO: This is a race condition. we might need to move this after the guard ping since guard
	// correctly waits for the actor to start.
	tokio::time::sleep(Duration::from_millis(500)).await;

	// Test ping via guard
	let ping_response =
		common::ping_actor_via_guard(ctx.leader_dc().guard_port(), &actor_id2, "main").await;
	assert_eq!(ping_response["status"], "ok");

	// Test websocket via guard
	let ws_response =
		common::ping_actor_websocket_via_guard(ctx.leader_dc().guard_port(), &actor_id2, "main")
			.await;
	assert_eq!(ws_response["status"], "ok");

	// Destroy
	tracing::info!("destroying actor");
	tokio::time::sleep(Duration::from_millis(500)).await;
	common::destroy_actor(&actor_id2, &namespace, target_dc2.guard_port()).await;
	tokio::time::sleep(Duration::from_millis(500)).await;

	runner.shutdown().await;
}
