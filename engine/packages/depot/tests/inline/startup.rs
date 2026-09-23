use super::*;
#[tokio::test]
#[ignore = "requires RIVET_ACTOR_START_PRELOAD_PAGES=128"]
async fn startup_cache_is_bounded_and_never_crosses_heads_or_branches() {
	assert!(
		page_limit() >= 2,
		"run with RIVET_ACTOR_START_PRELOAD_PAGES=128"
	);
	let bucket = BucketId::new_v4();
	let branch = DatabaseBranchId::new_v4();
	remember(
		bucket,
		"startup-test",
		branch,
		1,
		300,
		None,
		(1..=300).map(|n| (n, vec![1; 4096])),
	)
	.await;
	let (pages, misses) = assemble(bucket, "startup-test", branch, 1, 300).await;
	assert_eq!(pages.len(), page_limit() as usize);
	assert_eq!(misses, 0);
	assert!(
		assemble(bucket, "startup-test", branch, 2, 300)
			.await
			.0
			.is_empty()
	);
	assert!(
		assemble(bucket, "startup-test", DatabaseBranchId::new_v4(), 1, 300)
			.await
			.0
			.is_empty()
	);
	remember(
		bucket,
		"startup-test",
		branch,
		2,
		2,
		Some(1),
		[(1, vec![2; 4096])],
	)
	.await;
	let (pages, misses) = assemble(bucket, "startup-test", branch, 2, 2).await;
	assert_eq!(misses, 0);
	assert_eq!(pages.len(), 2);
	assert_eq!(pages[&1][0], 2);
	assert_eq!(pages[&2][0], 1);
	assert_eq!(
		assemble(bucket, "startup-test", branch, 1, 300).await.0[&1][0],
		1
	);
	remember(
		bucket,
		"startup-test",
		branch,
		5,
		2,
		Some(4),
		[(1, vec![5; 4096])],
	)
	.await;
	assert_eq!(assemble(bucket, "startup-test", branch, 5, 2).await.1, 1);
}
