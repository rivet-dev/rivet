use crate::actor::state::PersistedActor;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum PreloadedPersistedActor {
	#[default]
	NoBundle,
	BundleExistsButEmpty,
	Some(PersistedActor),
}

impl From<Option<PersistedActor>> for PreloadedPersistedActor {
	fn from(persisted: Option<PersistedActor>) -> Self {
		match persisted {
			Some(persisted) => Self::Some(persisted),
			None => Self::NoBundle,
		}
	}
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PreloadedKv {
	entries: Vec<PreloadedKvEntry>,
	requested_get_keys: Vec<Vec<u8>>,
	requested_prefixes: Vec<Vec<u8>>,
}

#[derive(Clone, Debug)]
struct PreloadedKvEntry {
	key: Vec<u8>,
	value: Vec<u8>,
}

impl PreloadedKv {
	pub(crate) fn new_with_requested_get_keys(
		entries: impl IntoIterator<Item = (Vec<u8>, Vec<u8>)>,
		requested_get_keys: Vec<Vec<u8>>,
		requested_prefixes: Vec<Vec<u8>>,
	) -> Self {
		Self {
			entries: entries
				.into_iter()
				.map(|(key, value)| PreloadedKvEntry { key, value })
				.collect(),
			requested_get_keys,
			requested_prefixes,
		}
	}

	pub(crate) fn key_entry(&self, key: &[u8]) -> Option<Option<Vec<u8>>> {
		if let Some(entry) = self.entries.iter().find(|entry| entry.key == key) {
			return Some(Some(entry.value.clone()));
		}

		if self
			.requested_get_keys
			.iter()
			.any(|requested| requested.as_slice() == key)
		{
			return Some(None);
		}

		None
	}

	pub(crate) fn prefix_entries(&self, prefix: &[u8]) -> Option<Vec<(Vec<u8>, Vec<u8>)>> {
		if !self
			.requested_prefixes
			.iter()
			.any(|requested| requested.as_slice() == prefix)
		{
			return None;
		}

		Some(
			self.entries
				.iter()
				.filter(|entry| entry.key.starts_with(prefix))
				.map(|entry| (entry.key.clone(), entry.value.clone()))
				.collect(),
		)
	}
}
