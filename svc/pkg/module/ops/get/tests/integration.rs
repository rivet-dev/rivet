use std::collections::HashSet;

use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let res = op!([ctx] module_get {
		module_ids: Vec::new(),
	})
	.await
	.unwrap();
	assert!(res.modules.is_empty());
}

#[worker_test]
async fn fetch(ctx: TestCtx) {
	// Generate test modules
	let module_ids = std::iter::repeat_with(Uuid::new_v4)
		.take(8)
		.collect::<HashSet<_>>();

	// Insert test modules
	for module_id in &module_ids {
		msg!([ctx] module::msg::create(module_id) -> module::msg::create_complete(module_id) {
			module_id: Some((*module_id).into()),
			name_id: "test".into(),
			team_id: Some(Uuid::new_v4().into()),
			creator_user_id: None,
		})
		.await
		.unwrap();
	}

	// Fetch the modules
	let res = op!([ctx] module_get {
		module_ids: module_ids.iter().cloned().map(|x| x.into()).collect(),
	})
	.await
	.unwrap();

	// Validate the modules
	assert_eq!(module_ids.len(), res.modules.len());
}
