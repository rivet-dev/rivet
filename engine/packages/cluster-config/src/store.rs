//! Storage adapter for the existing typed cluster-config key and generated v1 BARE value.
use crate::cluster_config_machine::types::{Preparation, PreparedConfig, ReadResult, StoredConfig};
use crate::{dynamic_config, keys::s3::ClusterConfigKey};
use anyhow::{Context, Result, ensure};
use object_store::{ObjectStore, ObjectStoreExt, PutMode, UpdateVersion};
use rivet_config::{Config, DynamicConfigUpdate};
use rivet_data::generated::rivet_cluster_config_v1::Data as ClusterConfig;
use rivet_keys::keys::{KeyAddress, codec::FormalKey};
use rivet_object_store_util::conditional_write::WriteResult;

/// Keeps a malformed or unexpectedly large config out of every process's control loop.
const MAX_CONFIG_BYTES: usize = 1024 * 1024;

/// Fetch one exact object. Only NotFound means absence; all transport failures remain unknown.
pub(crate) async fn read(store: &dyn ObjectStore) -> ReadResult {
	let key = ClusterConfigKey::new();
	let result = match store.get(&key.pack()).await {
		Ok(result) => result,
		Err(object_store::Error::NotFound { .. }) => return ReadResult::Missing,
		Err(error) => return ReadResult::Unknown(error.to_string()),
	};
	let Some(etag) = result.meta.e_tag.clone().filter(|tag| !tag.is_empty()) else {
		return ReadResult::Invalid("cluster config lacks a confirmed ETag".into());
	};
	if result.meta.size > MAX_CONFIG_BYTES as u64 {
		return ReadResult::Invalid("cluster config exceeds byte limit".into());
	}
	let bytes = match result.bytes().await {
		Ok(bytes) => bytes,
		Err(error) => return ReadResult::Unknown(error.to_string()),
	};
	if bytes.len() > MAX_CONFIG_BYTES {
		return ReadResult::Invalid("cluster config exceeds byte limit".into());
	}
	if let Err(error) = key.deserialize(bytes.clone()) {
		return ReadResult::Invalid(error.to_string());
	}
	ReadResult::Found(StoredConfig {
		bytes: bytes.to_vec(),
		etag,
	})
}

/// Send one immutable conditional request; the state machine owns all retry/readback decisions.
pub(crate) async fn write(store: &dyn ObjectStore, request: PreparedConfig) -> WriteResult {
	let mode = match request.expected_etag {
		Some(etag) => PutMode::Update(UpdateVersion {
			e_tag: Some(etag),
			version: None,
		}),
		None => PutMode::Create,
	};
	match store
		.put_opts(
			&ClusterConfigKey::new().pack(),
			request.bytes.into(),
			mode.into(),
		)
		.await
	{
		Ok(result) => result
			.e_tag
			.filter(|tag| !tag.is_empty())
			.map_or(WriteResult::Unknown, |etag| WriteResult::Confirmed { etag }),
		Err(_) => WriteResult::Unknown,
	}
}

/// Merge the existing typed update format once. This never rebases an uncertain prepared mutation.
pub(crate) fn prepare(current: &Config, preparation: Preparation) -> Result<PreparedConfig> {
	let key = ClusterConfigKey::new();
	let (mut config, expected_etag, patch) = match preparation {
		Preparation::Bootstrap => (ClusterConfig { dynamic: None }, None, None),
		Preparation::Update { base, patch } => (
			key.deserialize(base.bytes.into())?,
			Some(base.etag),
			Some(patch),
		),
	};
	if let Some(patch) = patch {
		let patch: DynamicConfigUpdate = serde_json::from_slice(&patch)?;
		let mut saved = serde_json::to_value(dynamic_config(&config)?)?;
		let patch = serde_json::to_value(patch)?;
		let saved_fields = saved
			.as_object_mut()
			.context("dynamic config must be an object")?;
		for (name, value) in patch
			.as_object()
			.context("dynamic update must be an object")?
		{
			if let Some(fields) = value.as_object()
				&& let Some(existing) = saved_fields.get_mut(name).and_then(|v| v.as_object_mut())
			{
				existing.extend(fields.clone());
				continue;
			}
			saved_fields.insert(name.clone(), value.clone());
		}
		let saved: DynamicConfigUpdate = serde_json::from_value(saved)?;
		config.dynamic = Some((serde_json::to_vec(&saved)?).into());
	}
	current.validate_dynamic(&dynamic_config(&config)?)?;
	let bytes = key.serialize(config)?.to_vec();
	ensure!(
		bytes.len() <= MAX_CONFIG_BYTES,
		"cluster config exceeds byte limit"
	);
	Ok(PreparedConfig {
		bytes,
		expected_etag,
	})
}

/// Decode and validate before changing live settings; failed validation leaves the last snapshot intact.
pub(crate) fn apply(current: &Config, snapshot: StoredConfig) -> Result<()> {
	let config = ClusterConfigKey::new().deserialize(snapshot.bytes.into())?;
	crate::validate_and_apply_config(&config, current)
}
