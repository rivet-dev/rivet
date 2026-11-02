use std::{sync::Arc, time::Duration};

use super::local_object_store::LocalObjectStore;
use anyhow::Result;
use depot_worker::{
	object_store_first_attempt::FirstAttemptDeadlines,
	object_store_metrics::InstrumentedHttpConnector,
};
use object_store::{
	ClientConfigKey, ObjectStore, RetryConfig,
	aws::{AmazonS3Builder, AmazonS3ConfigKey},
	azure::{AzureConfigKey, MicrosoftAzureBuilder},
	gcp::{GoogleCloudStorageBuilder, GoogleConfigKey},
	memory::InMemory,
	prefix::PrefixStore,
};
use rivet_config::{
	Config,
	config::{ObjectStorageConfig, ObjectStoreBackend},
};

#[derive(Clone)]
pub struct ObjectStores {
	pub primary: Arc<dyn ObjectStore>,
	/// The same store through its own client and connection pool, used only for topology
	/// heartbeats, discovery and lease objects. A renewal must confirm within the lease's
	/// serving window, so it must never queue behind bulk SST uploads for a connection or for
	/// the primary client's request slots; a saturated upload link once delayed renewals past
	/// the lease on every engine.
	pub lease: Arc<dyn ObjectStore>,
}

/// Build stores before EngineContext: startup needs the primary store to validate cluster config
/// and apply durable dynamic config.
pub fn create_object_stores(config: &Config) -> Result<ObjectStores> {
	let storage = config.object_storage();
	let policy = RequestPolicy::Retrying {
		first_attempts: first_attempt_deadlines(storage),
	};
	// Every production request is timed at this adapter boundary.
	let primary = depot_worker::object_store_metrics::InstrumentedObjectStore::wrap(
		build_object_store(&storage.primary, policy)?,
	);
	// A second build of a remote store is a second HTTP client with its own connection pool.
	// A local store has no connections, and a second build would not share the first one's
	// conditional-write lock (or, in memory, its objects).
	let lease = match &storage.primary {
		ObjectStoreBackend::InMemory | ObjectStoreBackend::FileSystem(_) => primary.clone(),
		ObjectStoreBackend::Gcs(_) | ObjectStoreBackend::Azure(_) | ObjectStoreBackend::S3(_) => {
			depot_worker::object_store_metrics::InstrumentedObjectStore::wrap(build_object_store(
				&storage.primary,
				policy,
			)?)
		}
	};
	Ok(ObjectStores { primary, lease })
}

fn first_attempt_deadlines(storage: &ObjectStorageConfig) -> FirstAttemptDeadlines {
	FirstAttemptDeadlines {
		read: storage.read_first_attempt_ms().map(Duration::from_millis),
		write: storage.write_first_attempt_ms().map(Duration::from_millis),
		write_bytes_per_second: storage.write_first_attempt_bytes_per_second(),
	}
}

/// Builds the store the object-store latency canary probes: the engine's client and
/// instrumentation, but one attempt per request allowed `attempt_timeout`, so a slow answer
/// is measured instead of being retried or cut at the engine's shorter client timeout.
pub fn create_canary_object_store(
	backend: &ObjectStoreBackend,
	attempt_timeout: Duration,
) -> Result<Arc<dyn ObjectStore>> {
	let store = build_object_store(
		backend,
		RequestPolicy::SingleAttempt {
			timeout: attempt_timeout,
		},
	)?;
	Ok(depot_worker::object_store_metrics::InstrumentedObjectStore::wrap(store))
}

/// How the client issues each request.
#[derive(Clone, Copy)]
enum RequestPolicy {
	/// `object_store`'s own retry loop and client timeouts, as the engine runs them, with a
	/// short first attempt for small requests that the retry loop retries on a cut-off.
	Retrying {
		first_attempts: FirstAttemptDeadlines,
	},
	/// One attempt, with connect and request timeouts of `timeout`.
	SingleAttempt { timeout: Duration },
}

impl RequestPolicy {
	/// Client settings to apply, as `(key, value)` pairs, and the retry configuration to
	/// replace the builder's default with. `Retrying` changes nothing, so environment
	/// configuration read by `from_env` stays in effect.
	fn client_settings(self) -> Option<(Vec<(ClientConfigKey, String)>, RetryConfig)> {
		match self {
			RequestPolicy::Retrying { .. } => None,
			RequestPolicy::SingleAttempt { timeout } => {
				let timeout = format!("{}ms", timeout.as_millis());
				let settings = vec![
					(ClientConfigKey::Timeout, timeout.clone()),
					(ClientConfigKey::ConnectTimeout, timeout),
				];
				let retry = RetryConfig {
					max_retries: 0,
					..RetryConfig::default()
				};
				Some((settings, retry))
			}
		}
	}

	/// The connector for this policy. A single attempt measures the store's own latency, so
	/// it never cuts an attempt short.
	fn connector(self) -> InstrumentedHttpConnector {
		match self {
			RequestPolicy::Retrying { first_attempts } => {
				InstrumentedHttpConnector::with_short_first_attempts(first_attempts)
			}
			RequestPolicy::SingleAttempt { .. } => InstrumentedHttpConnector::default(),
		}
	}
}

fn build_object_store(
	config: &ObjectStoreBackend,
	policy: RequestPolicy,
) -> Result<Arc<dyn ObjectStore>> {
	let client_settings = policy.client_settings();
	match config {
		ObjectStoreBackend::InMemory => Ok(Arc::new(InMemory::new())),
		ObjectStoreBackend::FileSystem(local) => Ok(LocalObjectStore::open(&local.path)?),
		ObjectStoreBackend::Gcs(gcs) => {
			let mut builder = GoogleCloudStorageBuilder::from_env()
				.with_bucket_name(&gcs.bucket)
				.with_http_connector(policy.connector());
			if let Some(key) = &gcs.service_account_key {
				builder = builder.with_service_account_key(key.read());
			}
			if let Some((settings, retry)) = client_settings {
				for (key, value) in settings {
					builder = builder.with_config(GoogleConfigKey::Client(key), value);
				}
				builder = builder.with_retry(retry);
			}
			let store = builder.build()?.with_generation_etags();
			if let Some(prefix) = gcs.prefix.as_deref().filter(|prefix| !prefix.is_empty()) {
				Ok(Arc::new(PrefixStore::new(store, prefix)))
			} else {
				Ok(Arc::new(store))
			}
		}
		ObjectStoreBackend::Azure(azure) => {
			let mut builder = MicrosoftAzureBuilder::from_env()
				.with_account(&azure.account)
				.with_container_name(&azure.container)
				.with_http_connector(policy.connector());
			if let Some(access_key) = &azure.access_key {
				builder = builder.with_access_key(access_key.read());
			}
			if let Some((settings, retry)) = client_settings {
				for (key, value) in settings {
					builder = builder.with_config(AzureConfigKey::Client(key), value);
				}
				builder = builder.with_retry(retry);
			}
			let store = builder.build()?;
			if let Some(prefix) = azure.prefix.as_deref().filter(|prefix| !prefix.is_empty()) {
				Ok(Arc::new(PrefixStore::new(store, prefix)))
			} else {
				Ok(Arc::new(store))
			}
		}
		ObjectStoreBackend::S3(s3) => {
			let mut builder = AmazonS3Builder::from_env()
				.with_bucket_name(&s3.bucket)
				.with_http_connector(policy.connector());
			if let Some(region) = &s3.region {
				builder = builder.with_region(region);
			}
			if let Some(endpoint) = &s3.endpoint {
				builder = builder.with_endpoint(endpoint);
			}
			if let Some(access_key_id) = &s3.access_key_id {
				builder = builder.with_access_key_id(access_key_id);
			}
			if let Some(secret_access_key) = &s3.secret_access_key {
				builder = builder.with_secret_access_key(secret_access_key.read());
			}
			if let Some((settings, retry)) = client_settings {
				for (key, value) in settings {
					builder = builder.with_config(AmazonS3ConfigKey::Client(key), value);
				}
				builder = builder.with_retry(retry);
			}
			let store = builder.build()?;
			if let Some(prefix) = s3.prefix.as_deref().filter(|prefix| !prefix.is_empty()) {
				Ok(Arc::new(PrefixStore::new(store, prefix)))
			} else {
				Ok(Arc::new(store))
			}
		}
	}
}
