use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let region_res = op!([ctx] faker_region {}).await.unwrap();
	let get_res = op!([ctx] region_get {
		region_ids: vec![region_res.region_id.unwrap(), Uuid::new_v4().into()],
	})
	.await
	.unwrap();
	let region = get_res.regions.first().unwrap();

	let res = op!([ctx] region_resolve {
		name_ids: vec![region.name_id.clone()],
	})
	.await
	.unwrap();

	assert_eq!(1, res.regions.len());
}
