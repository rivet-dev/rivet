use chirp_workflow::prelude::*;

#[workflow_test]
async fn empty(ctx: TestCtx) {
	op!([ctx] cluster_datacenter_topology_get {
		datacenter_ids: vec![],
	})
	.await
	.unwrap();
}
