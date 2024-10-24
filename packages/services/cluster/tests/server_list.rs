use chirp_workflow::prelude::*;

#[workflow_test]
async fn server_list(ctx: TestCtx) {
	ctx.op(cluster::ops::server::list::Input {
		filter: cluster::types::Filter {
			server_ids: None,
			datacenter_ids: None,
			cluster_ids: Some(vec![util::uuid::parse(
				"00000000-0000-0000-0000-000000000000",
			)
			.unwrap()]),
			pool_types: None,
			public_ips: None,
		},
		include_destroyed: false,
	})
	.await
	.unwrap();
}
