mod moved_tests {
	use crate::actor::keys::PERSIST_DATA_KEY;
	use crate::actor::state::{PersistedActor, encode_persisted_actor};
	use crate::registry::envoy_callbacks::{
		PreloadedPersistedActor, decode_preloaded_persisted_actor,
	};
	use rivet_envoy_client::protocol;

	#[test]
	fn decode_preloaded_persisted_actor_distinguishes_bundle_states() {
		assert_eq!(
			decode_preloaded_persisted_actor(None).expect("no bundle should decode"),
			PreloadedPersistedActor::NoBundle
		);

		let requested_empty = protocol::PreloadedKv {
			entries: Vec::new(),
			requested_get_keys: vec![PERSIST_DATA_KEY.to_vec()],
			requested_prefixes: Vec::new(),
		};
		assert_eq!(
			decode_preloaded_persisted_actor(Some(&requested_empty))
				.expect("empty bundle should decode"),
			PreloadedPersistedActor::BundleExistsButEmpty
		);

		let not_requested = protocol::PreloadedKv {
			entries: Vec::new(),
			requested_get_keys: Vec::new(),
			requested_prefixes: Vec::new(),
		};
		assert_eq!(
			decode_preloaded_persisted_actor(Some(&not_requested))
				.expect("unrequested bundle should decode"),
			PreloadedPersistedActor::NoBundle
		);

		let persisted = PersistedActor {
			state: vec![1, 2, 3],
			..PersistedActor::default()
		};
		let with_actor = protocol::PreloadedKv {
			entries: vec![protocol::PreloadedKvEntry {
				key: PERSIST_DATA_KEY.to_vec(),
				value: encode_persisted_actor(&persisted).expect("persisted actor should encode"),
				metadata: protocol::KvMetadata {
					version: Vec::new(),
					update_ts: 0,
				},
			}],
			requested_get_keys: vec![PERSIST_DATA_KEY.to_vec()],
			requested_prefixes: Vec::new(),
		};
		assert_eq!(
			decode_preloaded_persisted_actor(Some(&with_actor))
				.expect("persisted actor bundle should decode"),
			PreloadedPersistedActor::Some(persisted)
		);
	}
}
