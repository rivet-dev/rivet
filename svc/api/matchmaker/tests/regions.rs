// #[tokio::test(flavor = "multi_thread")]
// async fn list_regions() {
// 	let ctx = Ctx::init().await;
// 	let http_client = ctx.http_client(ctx.ns_dev_auth_token.clone());

// 	// MARK: GET /matchmaker/regions/recommend
// 	{
// 		tracing::info!("recommending region");

// 		let _res = http_client.list_regions().send().await.unwrap();
// 	}
// }

// // NOTE: This test is identical to `recommend_region`
// #[tokio::test(flavor = "multi_thread")]
// async fn list_regions_dev() {
// 	let ctx = Ctx::init().await;
// 	let http_client = ctx.http_client(ctx.ns_dev_auth_token.clone());

// 	// MARK: GET /matchmaker/regions/recommend
// 	{
// 		tracing::info!("recommending region dev");

// 		let _res = http_client.list_regions().send().await.unwrap();
// 	}
// }
