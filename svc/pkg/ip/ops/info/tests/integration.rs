use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let bogon_res = op!([ctx] ip_info {
		ip: "10.0.0.0".into(),
	})
	.await
	.unwrap();
	assert!(bogon_res.ip_info.is_none());

	let real_ip = "143.198.133.244".to_owned();

	let real_res = op!([ctx] ip_info {
		ip: real_ip.clone(),
		provider: ip::info::Provider::IpInfoIo as i32,
	})
	.await
	.unwrap();
	assert!(real_res.ip_info.is_some());

	let rows_num = sqlx::query("SELECT ip FROM db_ip_info.ips WHERE ip = $1")
		.bind(&real_ip)
		.fetch_all(&ctx.crdb().await.unwrap())
		.await
		.unwrap()
		.len();
	assert_eq!(1, rows_num, "ip info not cached"); // TODO: Add back

	let cached_res = op!([ctx] ip_info {
		ip: real_ip.clone(),
	})
	.await
	.unwrap();
	assert!(cached_res.ip_info.is_some());
}
