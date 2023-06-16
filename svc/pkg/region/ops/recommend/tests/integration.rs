use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let regions_res = op!([ctx] region_list {
		..Default::default()
	})
	.await
	.unwrap();

	// op!([ctx] region_recommend {
	// 	origin_ip: Some("159.89.1.175".into()),
	// 	region_ids: regions_res.region_ids.clone(),
	// 	..Default::default()
	// })
	// .await
	// .unwrap();

	op!([ctx] region_recommend {
		latitude: Some(100.0),
		longitude: Some(200.0),
		region_ids: regions_res.region_ids.clone(),
		..Default::default()
	})
	.await
	.unwrap();
}
