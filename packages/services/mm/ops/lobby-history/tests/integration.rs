use chirp_worker::prelude::*;

struct TestLogEntry {
	lobby_id: Uuid,
	lobby_group_id: Uuid,
	region_id: Uuid,
	create_ts: i64,
	remove_ts: Option<i64>,
}

impl From<(Uuid, Uuid, i64, Option<i64>)> for TestLogEntry {
	fn from(
		(lobby_group_id, region_id, create_ts, remove_ts): (Uuid, Uuid, i64, Option<i64>),
	) -> TestLogEntry {
		TestLogEntry {
			lobby_id: Uuid::new_v4(),
			lobby_group_id,
			region_id,
			create_ts,
			remove_ts,
		}
	}
}

#[worker_test]
async fn empty(ctx: TestCtx) {
	let namespace_id = Uuid::new_v4();

	let region_a = Uuid::new_v4();
	let region_b = Uuid::new_v4();

	let lgi_a = Uuid::new_v4();
	let lgi_b = Uuid::new_v4();

	// Populate database
	let entries = vec![
		(lgi_a, region_a, 25, Some(75)),
		(lgi_a, region_a, 55, None),
		(lgi_b, region_a, 15, None),
		(lgi_b, region_b, 45, None),
	]
	.into_iter()
	.map(TestLogEntry::from)
	.collect::<Vec<_>>();

	for entry in entries {
		let _ = op!([ctx] faker_mm_lobby_row {
			lobby_id: Some(entry.lobby_id.into()),
			namespace_id: Some(namespace_id.into()),
			lobby_group_id: Some(entry.lobby_group_id.into()),
			region_id: Some(entry.region_id.into()),
			run_id: Some(Uuid::new_v4().into()),
			create_ts: Some(entry.create_ts),
			stop_ts: entry.remove_ts,
		})
		.await
		.unwrap();
	}

	// Test all
	{
		grace_period().await;

		let res = op!([ctx] mm_lobby_history {
			namespace_id: Some(namespace_id.into()),
			before_create_ts: 100,
			count: 10,
		})
		.await
		.unwrap();

		assert_eq!(4, res.lobby_ids.len());
	}

	// Test count
	{
		grace_period().await;

		let res = op!([ctx] mm_lobby_history {
			namespace_id: Some(namespace_id.into()),
			before_create_ts: 100,
			count: 1,
		})
		.await
		.unwrap();

		assert_eq!(1, res.lobby_ids.len());
	}

	// Test start ts
	{
		grace_period().await;

		let res = op!([ctx] mm_lobby_history {
			namespace_id: Some(namespace_id.into()),
			before_create_ts: 40,
			count: 10,
		})
		.await
		.unwrap();

		assert_eq!(2, res.lobby_ids.len());
	}
}

/// mm-lobby-history returns stale responses for performance purposes.
/// This waits for changes to propagate.
async fn grace_period() {
	tokio::time::sleep(std::time::Duration::from_secs(5)).await;
}
