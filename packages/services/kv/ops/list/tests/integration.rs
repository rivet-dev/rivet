use std::time::Duration;

use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde_json::json;

#[worker_test]
async fn basic(ctx: TestCtx) {
	let namespace_id = Uuid::new_v4();
	let value = serde_json::to_vec(&json!({ "foo": "bar" })).unwrap();

	let key1 = "directory/key/entry1";
	msg!([ctx] kv::msg::write(namespace_id, key1) -> kv::msg::update {
		namespace_id: Some(namespace_id.into()),
		key: key1.into(),
		value: Some(value.to_owned()),
	})
	.await
	.unwrap();

	let key2 = "directory/key/key2";
	msg!([ctx] kv::msg::write(namespace_id, key2) -> kv::msg::update {
		namespace_id: Some(namespace_id.into()),
		key: key2.into(),
		value: Some(value.to_owned()),
	})
	.await
	.unwrap();

	// This key will not be fetched by kv-list, it is not a child of the query directory
	let key3 = "directory/key/entry1/subentry";
	msg!([ctx] kv::msg::write(namespace_id, key3) -> kv::msg::update {
		namespace_id: Some(namespace_id.into()),
		key: key3.into(),
		value: Some(value.to_owned()),
	})
	.await
	.unwrap();

	// kv-list is not consistent for performance
	tokio::time::sleep(Duration::from_secs(2)).await;

	let res = op!([ctx] kv_list {
		namespace_id: Some(namespace_id.into()),
		directory: "directory/key".into(),
		with_values: false,
	})
	.await
	.unwrap();

	assert_eq!(res.entries.len(), 2, "wrong key count");
	assert_eq!(res.entries.first().unwrap().key, key1, "wrong first key");
	assert_eq!(res.entries.get(1).unwrap().key, key2, "wrong second key");
}

#[worker_test]
async fn root(ctx: TestCtx) {
	let namespace_id = Uuid::new_v4();
	let value = serde_json::to_vec(&json!({ "foo": "bar" })).unwrap();

	let key1 = "entry1";
	msg!([ctx] kv::msg::write(namespace_id, key1) -> kv::msg::update {
		namespace_id: Some(namespace_id.into()),
		key: key1.into(),
		value: Some(value.clone()),
	})
	.await
	.unwrap();

	let key2 = "entry2";
	msg!([ctx] kv::msg::write(namespace_id, key2) -> kv::msg::update {
		namespace_id: Some(namespace_id.into()),
		key: key2.into(),
		value: Some(value.clone()),
	})
	.await
	.unwrap();

	// kv-list is not consistent for performance
	tokio::time::sleep(Duration::from_secs(2)).await;

	let res = op!([ctx] kv_list {
		namespace_id: Some(namespace_id.into()),
		directory: "".into(),
		with_values: false,
	})
	.await
	.unwrap();

	assert_eq!(res.entries.len(), 2, "wrong key count");
}

#[worker_test]
async fn values(ctx: TestCtx) {
	let namespace_id = Uuid::new_v4();

	let key1 = "directory/key/entry1";
	let value1 = serde_json::to_vec(&json!({ "foo": "bar" })).unwrap();
	msg!([ctx] kv::msg::write(namespace_id, key1) -> kv::msg::update {
		namespace_id: Some(namespace_id.into()),
		key: key1.into(),
		value: Some(value1.to_owned()),
	})
	.await
	.unwrap();

	let key2 = "directory/key/entry2";
	let value2 = serde_json::to_vec(&json!({ "foo": "baz" })).unwrap();
	msg!([ctx] kv::msg::write(namespace_id, key2) -> kv::msg::update {
		namespace_id: Some(namespace_id.into()),
		key: key2.into(),
		value: Some(value2.to_owned()),
	})
	.await
	.unwrap();

	// kv-list is not consistent for performance
	tokio::time::sleep(Duration::from_secs(2)).await;

	let res = op!([ctx] kv_list {
		namespace_id: Some(namespace_id.into()),
		directory: "directory/key".into(),
		with_values: true,
	})
	.await
	.unwrap();

	assert_eq!(res.entries.len(), 2, "wrong key count");
	let first = res.entries.first().unwrap();
	assert_eq!(first.key, key1, "wrong first key");
	assert_eq!(
		serde_json::from_slice::<serde_json::Value>(first.value.as_ref().unwrap()).unwrap(),
		serde_json::from_slice::<serde_json::Value>(&value1).unwrap(),
		"wrong first value"
	);
	let second = res.entries.get(1).unwrap();
	assert_eq!(second.key, key2, "wrong second key");
	assert_eq!(
		serde_json::from_slice::<serde_json::Value>(second.value.as_ref().unwrap()).unwrap(),
		serde_json::from_slice::<serde_json::Value>(&value2).unwrap(),
		"wrong second value"
	);
}
