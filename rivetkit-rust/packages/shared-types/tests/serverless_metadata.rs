use rivetkit_shared_types::serverless_metadata::{
	ActorName, ServerlessActorPreload, ServerlessActorPreloadPrefix, ServerlessMetadataEnvoy,
	ServerlessMetadataEnvoyKind, ServerlessMetadataPayload,
};

#[test]
fn serverless_metadata_matches_typescript_json_shape() {
	let payload = ServerlessMetadataPayload {
		runtime: "rivetkit".to_owned(),
		version: "test-version".to_owned(),
		envoy_protocol_version: Some(3),
		actor_names: [(
			"counter".to_owned(),
			ActorName {
				metadata: Some(serde_json::json!({
					"icon": "icon",
					"name": "Counter",
					"preload": ServerlessActorPreload {
						keys: vec![vec![1], vec![3], vec![5, 1, 1]],
						prefixes: vec![ServerlessActorPreloadPrefix {
							prefix: vec![6, 1],
							max_bytes: 131_072,
							partial: false,
						}],
					},
					"customField": { "kept": true },
				})),
			},
		)]
		.into_iter()
		.collect(),
		envoy: Some(ServerlessMetadataEnvoy {
			kind: Some(ServerlessMetadataEnvoyKind::Serverless {}),
			version: Some(1),
		}),
		runner: None,
		client_endpoint: Some("http://client.example".to_owned()),
		client_namespace: Some("default".to_owned()),
		client_token: Some("client-token".to_owned()),
	};

	let encoded = serde_json::to_value(&payload).expect("payload should encode");

	assert_eq!(
		encoded["envoy"]["kind"],
		serde_json::json!({ "serverless": {} })
	);
	assert_eq!(encoded["envoyProtocolVersion"], 3);
	assert!(encoded.get("runner").is_none());
	assert_eq!(
		encoded["actorNames"]["counter"]["metadata"]["preload"]["keys"],
		serde_json::json!([[1], [3], [5, 1, 1]])
	);
	assert_eq!(
		encoded["actorNames"]["counter"]["metadata"]["preload"]["prefixes"][0]["maxBytes"],
		131_072
	);

	let decoded: ServerlessMetadataPayload =
		serde_json::from_value(encoded).expect("payload should decode");
	let preload = decoded.actor_names["counter"]
		.metadata
		.as_ref()
		.and_then(|metadata| metadata.get("preload"))
		.cloned()
		.and_then(|preload| serde_json::from_value::<ServerlessActorPreload>(preload).ok())
		.expect("preload should be present");

	assert_eq!(preload.keys, vec![vec![1], vec![3], vec![5, 1, 1]]);
	assert_eq!(preload.prefixes[0].prefix, vec![6, 1]);
	assert_eq!(
		decoded.actor_names["counter"].metadata.as_ref().unwrap()["customField"]["kept"],
		true
	);
}
