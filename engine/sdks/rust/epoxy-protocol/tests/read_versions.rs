use epoxy_protocol::{protocol as p, versioned};
use vbare::OwnedVersionedData;

#[test]
fn existing_requests_round_trip_at_both_wire_versions() {
	for version in [3, 4] {
		let request = p::Request {
			from_replica_id: 1,
			to_replica_id: 2,
			kind: p::RequestKind::AcceptRequest(p::AcceptRequest {
				key: b"key".to_vec(),
				value: None,
				ballot: p::Ballot {
					counter: 7,
					replica_id: 1,
				},
				mutable: true,
				version: 2,
			}),
		};
		let bytes = versioned::encode_request(request.clone(), version).unwrap();
		assert_eq!(versioned::decode_request(&bytes, version).unwrap(), request);
		let response = p::Response {
			kind: p::ResponseKind::KvGetResponse(p::KvGetResponse {
				value: Some(p::CommittedValue {
					value: None,
					version: 2,
					mutable: true,
				}),
			}),
		};
		let bytes = versioned::encode_response(response.clone(), version).unwrap();
		assert_eq!(
			versioned::decode_response(&bytes, version).unwrap(),
			response
		);
	}
}

#[test]
fn read_state_cannot_be_sent_to_an_old_peer() {
	let request = p::Request {
		from_replica_id: 1,
		to_replica_id: 2,
		kind: p::RequestKind::KvReadStateRequest(p::KvReadStateRequest {
			key: b"key".to_vec(),
			ballot: None,
			epoch: 1,
		}),
	};
	assert!(versioned::encode_request(request.clone(), 3).is_err());
	let bytes = versioned::encode_request(request.clone(), 4).unwrap();
	assert_eq!(versioned::decode_request(&bytes, 4).unwrap(), request);
}

#[test]
fn read_protocol_upgrade_keeps_storage_readable_by_v3_binaries() {
	let value = p::CommittedValue {
		value: None,
		version: 2,
		mutable: true,
	};
	let bytes = versioned::CommittedValue::wrap_latest(value.clone())
		.serialize_with_embedded_version(3)
		.unwrap();
	assert_eq!(
		versioned::CommittedValue::deserialize_with_embedded_version(&bytes).unwrap(),
		value
	);
	// Decode the v3 payload using the previous generated type as well.
	let old = epoxy_protocol::generated::v3::CommittedValue {
		value: None,
		version: 2,
		mutable: true,
	};
	let mut expected = 3u16.to_le_bytes().to_vec();
	expected.extend(serde_bare::to_vec(&old).unwrap());
	assert_eq!(bytes, expected);
}
