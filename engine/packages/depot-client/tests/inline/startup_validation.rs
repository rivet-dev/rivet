use super::*;
fn snapshot() -> protocol::ActorSqliteStartup {
	let mut page = vec![0; 4096];
	page[..16].copy_from_slice(b"SQLite format 3\0");
	page[28..32].copy_from_slice(&2u32.to_be_bytes());
	protocol::ActorSqliteStartup {
		fence: protocol::SqliteStartFence {
			branch_id: "branch".into(),
			head_txid: 7,
			db_size_pages: 2,
		},
		page_limit: 2,
		cache_misses: 1,
		pages: vec![protocol::SqliteFetchedPage {
			pgno: 1,
			bytes: Some(page),
		}],
	}
}
#[test]
fn startup_snapshot_rejects_incoherent_metadata_duplicates_and_oversized_pages() {
	let valid = snapshot();
	assert!(validate_startup(&valid).is_ok());
	let mut x = valid.clone();
	x.fence.db_size_pages = 3;
	assert!(validate_startup(&x).is_err());
	let mut x = valid.clone();
	x.pages.push(x.pages[0].clone());
	assert!(validate_startup(&x).is_err());
	let mut x = valid.clone();
	x.pages[0].bytes.as_mut().unwrap().push(0);
	assert!(validate_startup(&x).is_err());
	let mut x = valid.clone();
	x.page_limit = 257;
	assert!(validate_startup(&x).is_err());
	let mut x = valid;
	x.pages.clear();
	assert!(validate_startup(&x).is_err());
}
