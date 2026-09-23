use super::*;
#[test]
fn legacy_lease_upgrades_and_latest_writes_version_two() -> Result<()> {
	let old = LeaseV1 {
		namespace_id: Id::new_v1(1),
		pool_name: "test".into(),
		config: protocol::generated::v8::ActorConfig {
			name: "test".into(),
			key: None,
			input: None,
			create_ts: 1,
		},
		generation: 7,
		protocol_version: 8,
		envoy_key: None,
		connection_id: None,
		phase: Phase::Sleeping,
		last_event: -1,
		command: None,
		sleep_ts: Some(1),
		start_ts: None,
		connectable_ts: None,
		destroy_ts: None,
		alarm_ts: None,
	};
	let upgraded = <Lease as OwnedVersionedData>::deserialize(&serde_bare::to_vec(&old)?, 1)?;
	let encoded = upgraded.serialize_with_embedded_version(2)?;
	assert_eq!(&encoded[..2], &2u16.to_le_bytes());
	let decoded = Lease::deserialize_with_embedded_version(&encoded)?;
	assert_eq!(decoded.generation, 7);
	assert_eq!(decoded.config.name, "test");
	assert!(decoded.serialize_with_embedded_version(1).is_err());
	Ok(())
}
