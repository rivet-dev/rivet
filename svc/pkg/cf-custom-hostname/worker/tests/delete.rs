use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	if !util::feature::cf_custom_hostname() {
		return;
	}

	let game_res = op!([ctx] faker_game { }).await.unwrap();
	let namespace_id = game_res.namespace_ids.first().unwrap().as_uuid();

	let hostname = format!("{}.com", util::faker::ident());

	let res = msg!([ctx] cf_custom_hostname::msg::create(namespace_id, &hostname) -> Result<cf_custom_hostname::msg::create_complete, cf_custom_hostname::msg::create_fail> {
		namespace_id: Some(namespace_id.into()),
		hostname: hostname.clone(),
		bypass_pending_cap: false,
	}).await.unwrap().unwrap();
	let identifier = res.identifier.unwrap();

	msg!([ctx] cf_custom_hostname::msg::delete(namespace_id, &hostname) -> cf_custom_hostname::msg::delete_complete {
		namespace_id: Some(namespace_id.into()),
		hostname: hostname.clone(),
	}).await.unwrap();

	let res = op!([ctx] cf_custom_hostname_get {
		identifiers: vec![identifier],
	})
	.await
	.unwrap();
	assert!(res.custom_hostnames.is_empty());

	// Should do nothing
	msg!([ctx] cf_custom_hostname::msg::delete(namespace_id, &hostname) -> cf_custom_hostname::msg::delete_complete {
		namespace_id: Some(namespace_id.into()),
		hostname: hostname.clone(),
	}).await.unwrap();
}
