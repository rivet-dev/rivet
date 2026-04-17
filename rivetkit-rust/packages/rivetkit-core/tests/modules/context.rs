use super::*;

pub(crate) fn new_with_kv(
	actor_id: impl Into<String>,
	name: impl Into<String>,
	key: ActorKey,
	region: impl Into<String>,
	kv: crate::kv::Kv,
) -> ActorContext {
	ActorContext::build(
		actor_id.into(),
		name.into(),
		key,
		region.into(),
		ActorConfig::default(),
		kv,
		SqliteDb::default(),
	)
}

mod moved_tests {
	use super::ActorContext;
	use crate::types::ListOpts;

	#[tokio::test]
	async fn kv_helpers_delegate_to_kv_wrapper() {
		let ctx = super::new_with_kv(
			"actor-1",
			"actor",
			Vec::new(),
			"local",
			crate::kv::tests::new_in_memory(),
		);

		ctx.kv_batch_put(&[(b"alpha".as_slice(), b"1".as_slice())])
			.await
			.expect("kv batch put should succeed");

		let values = ctx
			.kv_batch_get(&[b"alpha".as_slice()])
			.await
			.expect("kv batch get should succeed");
		assert_eq!(values, vec![Some(b"1".to_vec())]);

		let listed = ctx
			.kv_list_prefix(b"alp", ListOpts::default())
			.await
			.expect("kv list prefix should succeed");
		assert_eq!(listed, vec![(b"alpha".to_vec(), b"1".to_vec())]);

		ctx.kv_batch_delete(&[b"alpha".as_slice()])
			.await
			.expect("kv batch delete should succeed");
		let values = ctx
			.kv_batch_get(&[b"alpha".as_slice()])
			.await
			.expect("kv batch get after delete should succeed");
		assert_eq!(values, vec![None]);
	}

	#[tokio::test]
	async fn foreign_runtime_only_helpers_fail_explicitly_when_unconfigured() {
		let ctx = ActorContext::default();

		assert!(ctx.db_exec("select 1").await.is_err());
		assert!(ctx.db_query("select 1", None).await.is_err());
		assert!(ctx.db_run("select 1", None).await.is_err());
		assert!(ctx.client_call(b"call").await.is_err());
		assert!(ctx.set_alarm(Some(1)).is_err());
		assert!(
			ctx.ack_hibernatable_websocket_message(b"gateway", b"request", 1)
				.is_err()
		);
	}
}
