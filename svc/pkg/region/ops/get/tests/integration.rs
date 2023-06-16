use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let region_res = op!([ctx] faker_region {}).await.unwrap();

	let res = op!([ctx] region_get {
		region_ids: vec![region_res.region_id.unwrap(), Uuid::new_v4().into()],
	})
	.await
	.unwrap();

	assert_eq!(1, res.regions.len());
}
