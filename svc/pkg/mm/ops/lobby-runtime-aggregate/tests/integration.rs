use chirp_worker::prelude::*;

#[worker_test]
async fn default(ctx: TestCtx) {
	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();

	let lobby_res2 = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();

	grace_period().await;

	let res = op!([ctx] mm_lobby_runtime_aggregate {
		namespace_ids: vec![lobby_res.namespace_id.unwrap(), lobby_res2.namespace_id.unwrap()],
		query_start: 0,
		query_end: util::timestamp::now()
	})
	.await
	.unwrap();

	assert_eq!(res.region_tier_times.len(), 2, "ns not found");
	for times in &res.region_tier_times {
		assert!(times.total_time > 0, "should have time");
	}
}

#[worker_test]
async fn missing_columns(ctx: TestCtx) {
	// Create a valid row
	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();

	let lobby_res2 = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();

	// Emulate an invalid insertion in the second namespace like a bugged
	// mm-lobby-cleanup
	let fake_namespace_id = lobby_res2.namespace_id.as_ref().unwrap().as_uuid();
	let now = util::timestamp::now();
	sqlx::query(indoc!(
		"
		UPDATE lobbies
		SET stop_ts = $1
		WHERE namespace_id = $2 AND create_ts = $3 AND lobby_id = $4
		"
	))
	.bind(now)
	.bind(fake_namespace_id)
	.bind(now - 1000)
	.bind(Uuid::new_v4())
	.execute(&ctx.crdb("db-mm-state").await.unwrap())
	.await
	.unwrap();

	grace_period().await;

	let res = op!([ctx] mm_lobby_runtime_aggregate {
		namespace_ids: vec![lobby_res.namespace_id.unwrap(), lobby_res2.namespace_id.unwrap()],
		query_start: 0,
		query_end: util::timestamp::now()
	})
	.await
	.unwrap();

	assert_eq!(2, res.region_tier_times.len(), "logs not found");
	for times in &res.region_tier_times {
		let times_ns_id = times.namespace_id.as_ref().unwrap().as_uuid();
		if times_ns_id == fake_namespace_id {
			// TODO: Calculate what the total time should be and that it doesn't
			// include the invalid lobby
		} else if times.namespace_id == lobby_res.namespace_id {
			assert!(times.total_time > 0, "should have time for normal lobby");
		} else {
			panic!("unknown time");
		}
	}
}

#[worker_test]
async fn out_of_range(ctx: TestCtx) {
	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();

	// mm-lobby-runtime-aggregate returns stale responses. Wait for changes to
	// propagate.
	tokio::time::sleep(std::time::Duration::from_secs(5)).await;

	grace_period().await;

	let res = op!([ctx] mm_lobby_runtime_aggregate {
		namespace_ids: vec![lobby_res.namespace_id.unwrap()],
		query_start: 0,
		query_end: 0
	})
	.await
	.unwrap();

	assert!(res.region_tier_times.is_empty(), "range check failed");
}

#[worker_test]
async fn min(ctx: TestCtx) {
	let lobby_id = Into::<common::Uuid>::into(Uuid::new_v4());

	let region_res = op!([ctx] faker_region {
		..Default::default()
	})
	.await
	.unwrap();

	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let version_res = op!([ctx] faker_game_version {
		game_id: game_res.game_id,
		..Default::default()
	})
	.await
	.unwrap();

	let _ = op!([ctx] faker_mm_lobby_row {
		lobby_id: Some(lobby_id),
		namespace_id: game_res.namespace_ids.first().cloned(),
		lobby_group_id: version_res.mm_config_meta.as_ref().unwrap().lobby_groups.first().unwrap().lobby_group_id,
		region_id: region_res.region_id,
		run_id: Some(Uuid::new_v4().into()),
		create_ts: Some(10),
		stop_ts: Some(20)
	})
	.await
	.unwrap();

	grace_period().await;

	let res = op!([ctx] mm_lobby_runtime_aggregate {
		namespace_ids: game_res.namespace_ids.clone(),
		query_start: 15,
		query_end: 25
	})
	.await
	.unwrap();

	assert_eq!(
		res.region_tier_times.first().unwrap().total_time,
		5,
		"minimum check failed"
	);
}

#[worker_test]
async fn max(ctx: TestCtx) {
	let lobby_id = Into::<common::Uuid>::into(Uuid::new_v4());

	let region_res = op!([ctx] faker_region {
		..Default::default()
	})
	.await
	.unwrap();

	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let version_res = op!([ctx] faker_game_version {
		game_id: game_res.game_id,
		..Default::default()
	})
	.await
	.unwrap();

	let _ = op!([ctx] faker_mm_lobby_row {
		lobby_id: Some(lobby_id),
		namespace_id: game_res.namespace_ids.first().cloned(),
		lobby_group_id: version_res.mm_config_meta.as_ref().unwrap().lobby_groups.first().unwrap().lobby_group_id,
		region_id: region_res.region_id,
		run_id: Some(Uuid::new_v4().into()),
		create_ts: Some(10),
		stop_ts: Some(20)
	})
	.await
	.unwrap();

	grace_period().await;

	let res = op!([ctx] mm_lobby_runtime_aggregate {
		namespace_ids: game_res.namespace_ids.clone(),
		query_start: 0,
		query_end: 15
	})
	.await
	.unwrap();

	assert_eq!(
		res.region_tier_times.first().unwrap().total_time,
		5,
		"minimum check failed"
	);
}

/// mm-lobby-runtime-aggregate returns stale responses for performance purposes.
/// This waits for changes to propagate.
async fn grace_period() {
	tokio::time::sleep(std::time::Duration::from_secs(5)).await;
}
