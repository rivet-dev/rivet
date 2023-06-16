use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let region_res = op!([ctx] faker_region {}).await.unwrap();
	let region_id = region_res.region_id.unwrap();

	let res = op!([ctx] region_list {
		..Default::default()
	})
	.await
	.unwrap();

	assert!(!res.region_ids.is_empty());
	assert!(res.region_ids.contains(&region_id));
}
