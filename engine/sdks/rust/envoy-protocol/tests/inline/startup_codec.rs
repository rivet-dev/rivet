use super::ActorCommandKeyData;
use crate::generated::v9;
use anyhow::Result;
use vbare::OwnedVersionedData;

#[test]
fn preload_round_trip_preserves_fence_and_cannot_downgrade() -> Result<()> {
	let start = v9::CommandStartActor {
		config: v9::ActorConfig {
			name: "preloaded".into(),
			key: None,
			input: None,
			create_ts: 1,
		},
		hibernating_requests: Vec::new(),
		preloaded_kv: None,
		waiting_requests: Vec::new(),
		sqlite_fence: Some(v9::SqliteStartFence {
			branch_id: "branch".into(),
			head_txid: 17,
			db_size_pages: 1,
		}),
		sqlite_startup: Some(v9::ActorSqliteStartup {
			fence: v9::SqliteStartFence {
				branch_id: "branch".into(),
				head_txid: 17,
				db_size_pages: 1,
			},
			page_limit: 128,
			cache_misses: 0,
			pages: vec![v9::SqliteFetchedPage {
				pgno: 1,
				bytes: Some(vec![7; 4096]),
			}],
		}),
	};
	let value = v9::ActorCommandKeyData::CommandStartActor(start);
	assert!(
		ActorCommandKeyData::wrap_latest(value.clone())
			.serialize(8)
			.is_err()
	);
	let encoded = ActorCommandKeyData::wrap_latest(value).serialize(9)?;
	let v9::ActorCommandKeyData::CommandStartActor(decoded) =
		ActorCommandKeyData::deserialize(&encoded, 9)?
	else {
		panic!("expected Start")
	};
	assert_eq!(decoded.sqlite_fence.unwrap().head_txid, 17);
	assert_eq!(
		decoded.sqlite_startup.unwrap().pages[0]
			.bytes
			.as_ref()
			.unwrap()
			.len(),
		4096
	);
	Ok(())
}
