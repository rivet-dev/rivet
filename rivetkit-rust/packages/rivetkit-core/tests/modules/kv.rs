use super::*;

pub(crate) fn new_in_memory() -> Kv {
	Kv {
		backend: KvBackend::InMemory(Arc::new(RwLock::new(BTreeMap::new()))),
		actor_id: String::new(),
	}
}

mod moved_tests {
	use crate::types::ListOpts;

	#[tokio::test]
	async fn in_memory_backend_supports_basic_crud_and_listing() {
		let kv = super::new_in_memory();

		kv.batch_put(&[(b"alpha".as_slice(), b"1".as_slice())])
			.await
			.expect("batch put should succeed");
		kv.batch_put(&[(b"beta".as_slice(), b"2".as_slice())])
			.await
			.expect("second batch put should succeed");

		let values = kv
			.batch_get(&[b"alpha".as_slice(), b"beta".as_slice()])
			.await
			.expect("batch get should succeed");
		assert_eq!(values, vec![Some(b"1".to_vec()), Some(b"2".to_vec())]);

		let prefix = kv
			.list_prefix(b"a", ListOpts::default())
			.await
			.expect("list prefix should succeed");
		assert_eq!(prefix, vec![(b"alpha".to_vec(), b"1".to_vec())]);

		let range = kv
			.list_range(
				b"alpha",
				b"gamma",
				ListOpts {
					reverse: true,
					limit: Some(1),
				},
			)
			.await
			.expect("list range should succeed");
		assert_eq!(range, vec![(b"beta".to_vec(), b"2".to_vec())]);

		kv.delete_range(b"alpha", b"beta")
			.await
			.expect("delete range should succeed");
		kv.batch_delete(&[b"beta".as_slice()])
			.await
			.expect("batch delete should succeed");

		let remaining = kv
			.batch_get(&[b"alpha".as_slice(), b"beta".as_slice()])
			.await
			.expect("batch get after deletes should succeed");
		assert_eq!(remaining, vec![None, None]);
	}
}
