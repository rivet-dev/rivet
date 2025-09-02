mod common;

use common::THREE_REPLICAS;
use epoxy::ops::propose::ProposalResult;
use epoxy_protocol::protocol;
use gas::prelude::*;
use rivet_acl::{Verifier, config::AclConfig};
use rivet_api_builder::{ApiCtx, GlobalApiCtx};
use rivet_util::Id;

#[tokio::test(flavor = "multi_thread")]
async fn proposal() {
	let key = b"foo";
	let value = b"bar";

	let test_ctx = common::TestCtx::new_with(THREE_REPLICAS).await.unwrap();

	let replica_id = test_ctx.leader_id;
	let ctx = test_ctx.get_ctx(replica_id);

	// Send the proposal using the ops API
	let result = ctx
		.op(epoxy::ops::propose::Input {
			proposal: protocol::Proposal {
				commands: vec![protocol::Command {
					kind: protocol::CommandKind::SetCommand(protocol::SetCommand {
						key: key.to_vec(),
						value: Some(value.to_vec()),
					}),
				}],
			},
		})
		.await
		.unwrap();

	assert!(
		matches!(result, ProposalResult::Committed),
		"proposal failed"
	);
}
