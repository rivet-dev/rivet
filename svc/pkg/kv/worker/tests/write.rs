use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde_json::json;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let namespace_id = Uuid::new_v4();
	let value = serde_json::to_vec(&json!({ "foo": "bar" })).unwrap();
	let key = "test/key".to_string();

	let msg_res = msg!([ctx] kv::msg::write(namespace_id, &key) -> kv::msg::update {
		namespace_id: Some(namespace_id.into()),
		key: key.clone(),
		value: Some(value.to_owned()),
	})
	.await
	.unwrap();

	assert_eq!(
		msg_res.namespace_id,
		Some(namespace_id.into()),
		"wrong namespace id"
	);
	assert_eq!(msg_res.key, key, "wrong key");
	assert_eq!(msg_res.value, Some(value.clone()), "wrong value written");

	let res = op!([ctx] kv_get {
		keys: vec![
			kv::get::request::Key {
				namespace_id: Some(namespace_id.into()),
				key: key.clone(),
			},
		],
	})
	.await
	.unwrap();

	assert_eq!(
		serde_json::from_slice::<serde_json::Value>(&res.values.first().unwrap().value).unwrap(),
		serde_json::from_slice::<serde_json::Value>(&value).unwrap(),
		"wrong value"
	);
}

#[worker_test]
async fn implicit_deletion_with_null(ctx: TestCtx) {
	let namespace_id = Uuid::new_v4();
	let value = serde_json::to_vec(&json!({ "foo": "bar" })).unwrap();
	let key = "test/key".to_string();

	msg!([ctx] kv::msg::write(namespace_id, &key) -> kv::msg::update {
		namespace_id: Some(namespace_id.into()),
		key: key.clone(),
		value: Some(value.to_owned()),
	})
	.await
	.unwrap();

	msg!([ctx] kv::msg::write(namespace_id, &key) -> kv::msg::update {
		namespace_id: Some(namespace_id.into()),
		key: key.clone(),
		value: Some("null".into()),
	})
	.await
	.unwrap();

	let res = op!([ctx] kv_get {
		keys: vec![
			kv::get::request::Key {
				namespace_id: Some(namespace_id.into()),
				key: key.clone(),
			},
		],
	})
	.await
	.unwrap();

	assert_eq!(res.values.first(), None, "wrong value");
}

#[worker_test]
async fn deletion(ctx: TestCtx) {
	let namespace_id = Uuid::new_v4();
	let value = serde_json::to_vec(&json!({ "foo": "bar" })).unwrap();
	let key = "test/key".to_string();

	msg!([ctx] kv::msg::write(namespace_id, &key) -> kv::msg::update {
		namespace_id: Some(namespace_id.into()),
		key: key.clone(),
		value: Some(value.to_owned()),
	})
	.await
	.unwrap();

	let msg_res = msg!([ctx] kv::msg::write(namespace_id, &key) -> kv::msg::update {
		namespace_id: Some(namespace_id.into()),
		key: key.clone(),
		value: None,
	})
	.await
	.unwrap();

	assert_eq!(
		msg_res.namespace_id,
		Some(namespace_id.into()),
		"wrong namespace id"
	);
	assert_eq!(msg_res.key, key, "wrong key");
	assert_eq!(msg_res.value, None, "wrong value written");

	let res = op!([ctx] kv_get {
		keys: vec![
			kv::get::request::Key {
				namespace_id: Some(namespace_id.into()),
				key: key.clone(),
			},
		],
	})
	.await
	.unwrap();

	assert_eq!(res.values.first(), None, "wrong value");
}
