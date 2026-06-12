use std::{
	borrow::Cow,
	env, fmt, hint,
	sync::{
		Arc,
		atomic::{AtomicU64, Ordering},
	},
	time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use futures_util::{FutureExt, StreamExt};
use gas::prelude::Id;
use rivet_pools::UdbPool;
use serde::Deserialize;
use universaldb::{
	RangeOption, Subspace,
	prelude::{PackError, PackResult, TupleDepth, TuplePack, TupleUnpack, VersionstampOffset},
	utils::IsolationLevel::{Serializable, Snapshot},
};
use universalpubsub::{NextOutput, PubSub, PublishOpts, Subject, Subscriber};

const ENV_PREFIX: &str = "UPS_BROADCAST_SIM";
const TICK: Duration = Duration::from_millis(10);
const PUBLISH_MAX_IN_FLIGHT: usize = 8_192;
const DEFAULT_TUNE_PATH: &str = "/tmp/ups-broadcast-sim-tune.json";
const TUNE_POLL_INTERVAL: Duration = Duration::from_secs(1);
const TUNE_SUBJECT: &str = "rivet.ups.broadcast.sim.tune";
const TUNE_SUBJECT_ROOT: &str = "rivet.ups.broadcast.sim.tune";
const GATEWAY_MEMBERSHIP_PREFIX: &[u8] = b"rivet/ups-broadcast/sim/gateway-members";
const GATEWAY_MEMBERSHIP_TX: &str = "ups_broadcast_sim_gateway_membership";
const UDB_HOT_COUNTER_TX: &str = "ups_broadcast_sim_udb_hot_counter";
const UDB_READ_SCAN_SEED_TX: &str = "ups_broadcast_sim_udb_read_scan_seed";
const UDB_READ_SCAN_TX: &str = "ups_broadcast_sim_udb_read_scan";
const UDB_CONFLICT_SEED_TX: &str = "ups_broadcast_sim_udb_conflict_seed";
const UDB_CONFLICT_TX: &str = "ups_broadcast_sim_udb_conflict";
const UDB_READ_SCAN_SEED_BATCH_SIZE: u64 = 500;
const UDB_CONFLICT_SEED_BATCH_SIZE: u64 = 500;
const READ_SCAN_KEY_ROOT: usize = 1;
const CONFLICT_KEY_ROOT: usize = 2;
static SUBJECT_SEQ: AtomicU64 = AtomicU64::new(0);
static HOT_COUNTER_SEQ: AtomicU64 = AtomicU64::new(0);
static READ_SCAN_SEQ: AtomicU64 = AtomicU64::new(0);
static CONFLICT_SEQ: AtomicU64 = AtomicU64::new(0);

pub struct Config {
	pub force_driver: bool,
	pub disable_memory_optimization: bool,
	tune_path: Option<String>,
	gateway_subjects: usize,
	gateway_subscribers: usize,
	gateway_publish_rps: f64,
	gateway_payload_bytes: usize,
	gateway_work_delay_ms: u64,
	gateway_work_cpu_us: u64,
	gateway_spread_replicas: usize,
	gateway_spread_member_ttl_ms: u64,
	envoy_subjects: usize,
	envoy_responders: usize,
	envoy_queue_group: Option<String>,
	envoy_request_unknown_root: bool,
	envoy_request_rps: f64,
	envoy_request_payload_bytes: usize,
	envoy_request_timeout_ms: u64,
	envoy_request_max_in_flight: usize,
	envoy_work_delay_ms: u64,
	envoy_work_cpu_us: u64,
	envoy_eviction_subscribers: usize,
	envoy_eviction_broadcast_rps: f64,
	envoy_eviction_work_delay_ms: u64,
	envoy_eviction_work_cpu_us: u64,
	worker_bump_subscribers: usize,
	worker_bump_broadcast_rps: f64,
	worker_bump_work_delay_ms: u64,
	worker_bump_work_cpu_us: u64,
	serverless_subscribers: usize,
	serverless_publish_rps: f64,
	serverless_payload_bytes: usize,
	serverless_work_delay_ms: u64,
	serverless_work_cpu_us: u64,
	cache_purge_subscribers: usize,
	cache_purge_broadcast_rps: f64,
	cache_purge_payload_bytes: usize,
	cache_purge_work_delay_ms: u64,
	cache_purge_work_cpu_us: u64,
	tracing_config_subscribers: usize,
	tracing_config_broadcast_rps: f64,
	tracing_config_payload_bytes: usize,
	tracing_config_work_delay_ms: u64,
	tracing_config_work_cpu_us: u64,
	route_stopped_subscribers: usize,
	route_churn_rps: f64,
	route_ephemeral_hold_ms: u64,
	route_stopped_hold_ms: u64,
	route_max_in_flight: usize,
	route_work_delay_ms: u64,
	route_work_cpu_us: u64,
	workflow_signal_churn_rps: f64,
	workflow_signal_hold_ms: u64,
	workflow_signal_publish_rps: f64,
	workflow_signal_work_delay_ms: u64,
	workflow_signal_work_cpu_us: u64,
	workflow_complete_publish_rps: f64,
	udb_hot_counter_rps: f64,
	udb_hot_counter_max_in_flight: usize,
	udb_hot_counter_namespace_id: Id,
	udb_hot_counter_actor_name: String,
	udb_read_scan_rps: f64,
	udb_read_scan_max_in_flight: usize,
	udb_read_scan_seed_keys: u64,
	udb_read_scan_keys_per_tx: usize,
	udb_read_scan_value_bytes: usize,
	udb_read_scan_unpack_keys: bool,
	udb_conflict_rps: f64,
	udb_conflict_max_in_flight: usize,
	udb_conflict_keys: u64,
}

impl Config {
	pub fn from_env() -> Result<Option<Self>> {
		if !env_bool("ENABLED", false)? {
			return Ok(None);
		}

		let profile = env_string("PROFILE").with_context(|| {
			format!("{ENV_PREFIX}_PROFILE must be set explicitly when {ENV_PREFIX}_ENABLED=true")
		})?;
		let mut config = match profile.as_str() {
			"custom" => Self::custom(),
			"staging_peak" => Self::staging_peak(),
			other => bail!("unknown {ENV_PREFIX}_PROFILE: {other}"),
		};

		config.force_driver = env_bool("FORCE_DRIVER", config.force_driver)?;
		config.disable_memory_optimization = env_bool(
			"DISABLE_MEMORY_OPTIMIZATION",
			config.disable_memory_optimization,
		)?;
		config.tune_path = env_string("TUNE_PATH")
			.map(|x| if x.is_empty() { None } else { Some(x) })
			.unwrap_or(config.tune_path);
		config.gateway_subjects = env_usize("GATEWAY_SUBJECTS", config.gateway_subjects)?;
		config.gateway_subscribers = env_usize("GATEWAY_SUBSCRIBERS", config.gateway_subscribers)?;
		config.gateway_publish_rps = env_f64("GATEWAY_PUBLISH_RPS", config.gateway_publish_rps)?;
		config.gateway_payload_bytes =
			env_usize("GATEWAY_PAYLOAD_BYTES", config.gateway_payload_bytes)?;
		config.gateway_work_delay_ms =
			env_u64("GATEWAY_WORK_DELAY_MS", config.gateway_work_delay_ms)?;
		config.gateway_work_cpu_us = env_u64("GATEWAY_WORK_CPU_US", config.gateway_work_cpu_us)?;
		config.gateway_spread_replicas =
			env_usize("GATEWAY_SPREAD_REPLICAS", config.gateway_spread_replicas)?;
		config.gateway_spread_member_ttl_ms = env_u64(
			"GATEWAY_SPREAD_MEMBER_TTL_MS",
			config.gateway_spread_member_ttl_ms,
		)?;
		config.envoy_subjects = env_usize("ENVOY_SUBJECTS", config.envoy_subjects)?;
		config.envoy_responders = env_usize("ENVOY_RESPONDERS", config.envoy_responders)?;
		config.envoy_queue_group = env_string("ENVOY_QUEUE_GROUP")
			.map(|x| if x.is_empty() { None } else { Some(x) })
			.unwrap_or(config.envoy_queue_group);
		config.envoy_request_unknown_root = env_bool(
			"ENVOY_REQUEST_UNKNOWN_ROOT",
			config.envoy_request_unknown_root,
		)?;
		config.envoy_request_rps = env_f64("ENVOY_REQUEST_RPS", config.envoy_request_rps)?;
		config.envoy_request_payload_bytes = env_usize(
			"ENVOY_REQUEST_PAYLOAD_BYTES",
			config.envoy_request_payload_bytes,
		)?;
		config.envoy_request_timeout_ms =
			env_u64("ENVOY_REQUEST_TIMEOUT_MS", config.envoy_request_timeout_ms)?;
		config.envoy_request_max_in_flight = env_usize(
			"ENVOY_REQUEST_MAX_IN_FLIGHT",
			config.envoy_request_max_in_flight,
		)?;
		config.envoy_work_delay_ms = env_u64("ENVOY_WORK_DELAY_MS", config.envoy_work_delay_ms)?;
		config.envoy_work_cpu_us = env_u64("ENVOY_WORK_CPU_US", config.envoy_work_cpu_us)?;
		config.envoy_eviction_subscribers = env_usize(
			"ENVOY_EVICTION_SUBSCRIBERS",
			config.envoy_eviction_subscribers,
		)?;
		config.envoy_eviction_broadcast_rps = env_f64(
			"ENVOY_EVICTION_BROADCAST_RPS",
			config.envoy_eviction_broadcast_rps,
		)?;
		config.envoy_eviction_work_delay_ms = env_u64(
			"ENVOY_EVICTION_WORK_DELAY_MS",
			config.envoy_eviction_work_delay_ms,
		)?;
		config.envoy_eviction_work_cpu_us = env_u64(
			"ENVOY_EVICTION_WORK_CPU_US",
			config.envoy_eviction_work_cpu_us,
		)?;
		config.worker_bump_subscribers =
			env_usize("WORKER_BUMP_SUBSCRIBERS", config.worker_bump_subscribers)?;
		config.worker_bump_broadcast_rps = env_f64(
			"WORKER_BUMP_BROADCAST_RPS",
			config.worker_bump_broadcast_rps,
		)?;
		config.worker_bump_work_delay_ms = env_u64(
			"WORKER_BUMP_WORK_DELAY_MS",
			config.worker_bump_work_delay_ms,
		)?;
		config.worker_bump_work_cpu_us =
			env_u64("WORKER_BUMP_WORK_CPU_US", config.worker_bump_work_cpu_us)?;
		config.serverless_subscribers =
			env_usize("SERVERLESS_SUBSCRIBERS", config.serverless_subscribers)?;
		config.serverless_publish_rps =
			env_f64("SERVERLESS_PUBLISH_RPS", config.serverless_publish_rps)?;
		config.serverless_payload_bytes =
			env_usize("SERVERLESS_PAYLOAD_BYTES", config.serverless_payload_bytes)?;
		config.serverless_work_delay_ms =
			env_u64("SERVERLESS_WORK_DELAY_MS", config.serverless_work_delay_ms)?;
		config.serverless_work_cpu_us =
			env_u64("SERVERLESS_WORK_CPU_US", config.serverless_work_cpu_us)?;
		config.cache_purge_subscribers =
			env_usize("CACHE_PURGE_SUBSCRIBERS", config.cache_purge_subscribers)?;
		config.cache_purge_broadcast_rps = env_f64(
			"CACHE_PURGE_BROADCAST_RPS",
			config.cache_purge_broadcast_rps,
		)?;
		config.cache_purge_payload_bytes = env_usize(
			"CACHE_PURGE_PAYLOAD_BYTES",
			config.cache_purge_payload_bytes,
		)?;
		config.cache_purge_work_delay_ms = env_u64(
			"CACHE_PURGE_WORK_DELAY_MS",
			config.cache_purge_work_delay_ms,
		)?;
		config.cache_purge_work_cpu_us =
			env_u64("CACHE_PURGE_WORK_CPU_US", config.cache_purge_work_cpu_us)?;
		config.tracing_config_subscribers = env_usize(
			"TRACING_CONFIG_SUBSCRIBERS",
			config.tracing_config_subscribers,
		)?;
		config.tracing_config_broadcast_rps = env_f64(
			"TRACING_CONFIG_BROADCAST_RPS",
			config.tracing_config_broadcast_rps,
		)?;
		config.tracing_config_payload_bytes = env_usize(
			"TRACING_CONFIG_PAYLOAD_BYTES",
			config.tracing_config_payload_bytes,
		)?;
		config.tracing_config_work_delay_ms = env_u64(
			"TRACING_CONFIG_WORK_DELAY_MS",
			config.tracing_config_work_delay_ms,
		)?;
		config.tracing_config_work_cpu_us = env_u64(
			"TRACING_CONFIG_WORK_CPU_US",
			config.tracing_config_work_cpu_us,
		)?;
		config.route_stopped_subscribers = env_usize(
			"ROUTE_STOPPED_SUBSCRIBERS",
			config.route_stopped_subscribers,
		)?;
		config.route_churn_rps = env_f64("ROUTE_CHURN_RPS", config.route_churn_rps)?;
		config.route_ephemeral_hold_ms =
			env_u64("ROUTE_EPHEMERAL_HOLD_MS", config.route_ephemeral_hold_ms)?;
		config.route_stopped_hold_ms =
			env_u64("ROUTE_STOPPED_HOLD_MS", config.route_stopped_hold_ms)?;
		config.route_max_in_flight = env_usize("ROUTE_MAX_IN_FLIGHT", config.route_max_in_flight)?;
		config.route_work_delay_ms = env_u64("ROUTE_WORK_DELAY_MS", config.route_work_delay_ms)?;
		config.route_work_cpu_us = env_u64("ROUTE_WORK_CPU_US", config.route_work_cpu_us)?;
		config.workflow_signal_churn_rps = env_f64(
			"WORKFLOW_SIGNAL_CHURN_RPS",
			config.workflow_signal_churn_rps,
		)?;
		config.workflow_signal_hold_ms =
			env_u64("WORKFLOW_SIGNAL_HOLD_MS", config.workflow_signal_hold_ms)?;
		config.workflow_signal_publish_rps = env_f64(
			"WORKFLOW_SIGNAL_PUBLISH_RPS",
			config.workflow_signal_publish_rps,
		)?;
		config.workflow_signal_work_delay_ms = env_u64(
			"WORKFLOW_SIGNAL_WORK_DELAY_MS",
			config.workflow_signal_work_delay_ms,
		)?;
		config.workflow_signal_work_cpu_us = env_u64(
			"WORKFLOW_SIGNAL_WORK_CPU_US",
			config.workflow_signal_work_cpu_us,
		)?;
		config.workflow_complete_publish_rps = env_f64(
			"WORKFLOW_COMPLETE_PUBLISH_RPS",
			config.workflow_complete_publish_rps,
		)?;
		config.udb_hot_counter_rps = env_f64("UDB_HOT_COUNTER_RPS", config.udb_hot_counter_rps)?;
		config.udb_hot_counter_max_in_flight = env_usize(
			"UDB_HOT_COUNTER_MAX_IN_FLIGHT",
			config.udb_hot_counter_max_in_flight,
		)?;
		config.udb_hot_counter_namespace_id = env_id(
			"UDB_HOT_COUNTER_NAMESPACE_ID",
			config.udb_hot_counter_namespace_id,
		)?;
		config.udb_hot_counter_actor_name =
			env_string("UDB_HOT_COUNTER_ACTOR_NAME").unwrap_or(config.udb_hot_counter_actor_name);
		config.udb_read_scan_rps = env_f64("UDB_READ_SCAN_RPS", config.udb_read_scan_rps)?;
		config.udb_read_scan_max_in_flight = env_usize(
			"UDB_READ_SCAN_MAX_IN_FLIGHT",
			config.udb_read_scan_max_in_flight,
		)?;
		config.udb_read_scan_seed_keys =
			env_u64("UDB_READ_SCAN_SEED_KEYS", config.udb_read_scan_seed_keys)?;
		config.udb_read_scan_keys_per_tx = env_usize(
			"UDB_READ_SCAN_KEYS_PER_TX",
			config.udb_read_scan_keys_per_tx,
		)?;
		config.udb_read_scan_value_bytes = env_usize(
			"UDB_READ_SCAN_VALUE_BYTES",
			config.udb_read_scan_value_bytes,
		)?;
		config.udb_read_scan_unpack_keys = env_bool(
			"UDB_READ_SCAN_UNPACK_KEYS",
			config.udb_read_scan_unpack_keys,
		)?;
		config.udb_conflict_rps = env_f64("UDB_CONFLICT_RPS", config.udb_conflict_rps)?;
		config.udb_conflict_max_in_flight = env_usize(
			"UDB_CONFLICT_MAX_IN_FLIGHT",
			config.udb_conflict_max_in_flight,
		)?;
		config.udb_conflict_keys = env_u64("UDB_CONFLICT_KEYS", config.udb_conflict_keys)?;

		validate_rate("GATEWAY_PUBLISH_RPS", config.gateway_publish_rps)?;
		validate_rate("ENVOY_REQUEST_RPS", config.envoy_request_rps)?;
		validate_rate(
			"ENVOY_EVICTION_BROADCAST_RPS",
			config.envoy_eviction_broadcast_rps,
		)?;
		validate_rate(
			"WORKER_BUMP_BROADCAST_RPS",
			config.worker_bump_broadcast_rps,
		)?;
		validate_rate("SERVERLESS_PUBLISH_RPS", config.serverless_publish_rps)?;
		validate_rate(
			"CACHE_PURGE_BROADCAST_RPS",
			config.cache_purge_broadcast_rps,
		)?;
		validate_rate(
			"TRACING_CONFIG_BROADCAST_RPS",
			config.tracing_config_broadcast_rps,
		)?;
		validate_rate("ROUTE_CHURN_RPS", config.route_churn_rps)?;
		validate_rate(
			"WORKFLOW_SIGNAL_CHURN_RPS",
			config.workflow_signal_churn_rps,
		)?;
		validate_rate(
			"WORKFLOW_SIGNAL_PUBLISH_RPS",
			config.workflow_signal_publish_rps,
		)?;
		validate_rate(
			"WORKFLOW_COMPLETE_PUBLISH_RPS",
			config.workflow_complete_publish_rps,
		)?;
		validate_rate("UDB_HOT_COUNTER_RPS", config.udb_hot_counter_rps)?;
		validate_rate("UDB_READ_SCAN_RPS", config.udb_read_scan_rps)?;
		validate_rate("UDB_CONFLICT_RPS", config.udb_conflict_rps)?;

		Ok(Some(config))
	}

	fn custom() -> Self {
		Self {
			force_driver: true,
			disable_memory_optimization: false,
			tune_path: Some(DEFAULT_TUNE_PATH.to_string()),
			gateway_subjects: 0,
			gateway_subscribers: 0,
			gateway_publish_rps: 0.0,
			gateway_payload_bytes: 192,
			gateway_work_delay_ms: 0,
			gateway_work_cpu_us: 0,
			gateway_spread_replicas: 0,
			gateway_spread_member_ttl_ms: 15_000,
			envoy_subjects: 0,
			envoy_responders: 0,
			envoy_queue_group: None,
			envoy_request_unknown_root: true,
			envoy_request_rps: 0.0,
			envoy_request_payload_bytes: 64,
			envoy_request_timeout_ms: 30_000,
			envoy_request_max_in_flight: 8_192,
			envoy_work_delay_ms: 0,
			envoy_work_cpu_us: 0,
			envoy_eviction_subscribers: 0,
			envoy_eviction_broadcast_rps: 0.0,
			envoy_eviction_work_delay_ms: 0,
			envoy_eviction_work_cpu_us: 0,
			worker_bump_subscribers: 0,
			worker_bump_broadcast_rps: 0.0,
			worker_bump_work_delay_ms: 0,
			worker_bump_work_cpu_us: 0,
			serverless_subscribers: 0,
			serverless_publish_rps: 0.0,
			serverless_payload_bytes: 256,
			serverless_work_delay_ms: 0,
			serverless_work_cpu_us: 0,
			cache_purge_subscribers: 0,
			cache_purge_broadcast_rps: 0.0,
			cache_purge_payload_bytes: 128,
			cache_purge_work_delay_ms: 0,
			cache_purge_work_cpu_us: 0,
			tracing_config_subscribers: 0,
			tracing_config_broadcast_rps: 0.0,
			tracing_config_payload_bytes: 128,
			tracing_config_work_delay_ms: 0,
			tracing_config_work_cpu_us: 0,
			route_stopped_subscribers: 0,
			route_churn_rps: 0.0,
			route_ephemeral_hold_ms: 25,
			route_stopped_hold_ms: 7_500,
			route_max_in_flight: 4_096,
			route_work_delay_ms: 0,
			route_work_cpu_us: 0,
			workflow_signal_churn_rps: 0.0,
			workflow_signal_hold_ms: 3_000,
			workflow_signal_publish_rps: 0.0,
			workflow_signal_work_delay_ms: 0,
			workflow_signal_work_cpu_us: 0,
			workflow_complete_publish_rps: 0.0,
			udb_hot_counter_rps: 0.0,
			udb_hot_counter_max_in_flight: 1_024,
			udb_hot_counter_namespace_id: Id::nil(),
			udb_hot_counter_actor_name: "sim-hot-namespace".to_string(),
			udb_read_scan_rps: 0.0,
			udb_read_scan_max_in_flight: 512,
			udb_read_scan_seed_keys: 50_000,
			udb_read_scan_keys_per_tx: 50,
			udb_read_scan_value_bytes: 128,
			udb_read_scan_unpack_keys: true,
			udb_conflict_rps: 0.0,
			udb_conflict_max_in_flight: 512,
			udb_conflict_keys: 32,
		}
	}

	fn staging_peak() -> Self {
		Self {
			gateway_subjects: 20,
			gateway_subscribers: 20,
			gateway_publish_rps: 5_016.0,
			gateway_payload_bytes: 192,
			gateway_work_cpu_us: 100,
			gateway_spread_replicas: 10,
			envoy_subjects: 8,
			envoy_responders: 8,
			envoy_queue_group: Some("rivet-ups-broadcast-sim-envoy".to_string()),
			envoy_request_unknown_root: true,
			envoy_request_rps: 2_094.0,
			envoy_request_payload_bytes: 64,
			envoy_work_delay_ms: 10,
			envoy_work_cpu_us: 250,
			envoy_eviction_subscribers: 72,
			envoy_eviction_work_delay_ms: 1,
			worker_bump_subscribers: 10,
			worker_bump_broadcast_rps: 141.0,
			worker_bump_work_delay_ms: 225,
			worker_bump_work_cpu_us: 2_000,
			serverless_subscribers: 10,
			serverless_work_delay_ms: 2,
			serverless_work_cpu_us: 250,
			cache_purge_subscribers: 20,
			cache_purge_broadcast_rps: 0.62,
			cache_purge_work_delay_ms: 2,
			cache_purge_work_cpu_us: 250,
			tracing_config_subscribers: 20,
			tracing_config_work_delay_ms: 1,
			route_stopped_subscribers: 2_100,
			route_churn_rps: 6.5,
			route_work_delay_ms: 5,
			route_work_cpu_us: 500,
			workflow_signal_churn_rps: 281.0,
			workflow_signal_hold_ms: 2_900,
			workflow_signal_publish_rps: 0.76,
			workflow_signal_work_delay_ms: 15,
			workflow_signal_work_cpu_us: 500,
			workflow_complete_publish_rps: 0.04,
			..Self::custom()
		}
	}
}

#[derive(Clone)]
struct Rate {
	value: Arc<AtomicU64>,
}

impl Rate {
	fn new(value: f64) -> Self {
		Self {
			value: Arc::new(AtomicU64::new(value.to_bits())),
		}
	}

	fn load(&self) -> f64 {
		f64::from_bits(self.value.load(Ordering::Relaxed))
	}

	fn store(&self, value: f64) {
		self.value.store(value.to_bits(), Ordering::Relaxed);
	}
}

struct Rates {
	gateway_publish_rps: Rate,
	envoy_request_rps: Rate,
	envoy_eviction_broadcast_rps: Rate,
	worker_bump_broadcast_rps: Rate,
	serverless_publish_rps: Rate,
	cache_purge_broadcast_rps: Rate,
	tracing_config_broadcast_rps: Rate,
	route_churn_rps: Rate,
	workflow_signal_churn_rps: Rate,
	workflow_signal_publish_rps: Rate,
	workflow_complete_publish_rps: Rate,
	udb_hot_counter_rps: Rate,
	udb_read_scan_rps: Rate,
	udb_conflict_rps: Rate,
}

impl Rates {
	fn new(config: &Config) -> Self {
		Self {
			gateway_publish_rps: Rate::new(config.gateway_publish_rps),
			envoy_request_rps: Rate::new(config.envoy_request_rps),
			envoy_eviction_broadcast_rps: Rate::new(config.envoy_eviction_broadcast_rps),
			worker_bump_broadcast_rps: Rate::new(config.worker_bump_broadcast_rps),
			serverless_publish_rps: Rate::new(config.serverless_publish_rps),
			cache_purge_broadcast_rps: Rate::new(config.cache_purge_broadcast_rps),
			tracing_config_broadcast_rps: Rate::new(config.tracing_config_broadcast_rps),
			route_churn_rps: Rate::new(config.route_churn_rps),
			workflow_signal_churn_rps: Rate::new(config.workflow_signal_churn_rps),
			workflow_signal_publish_rps: Rate::new(config.workflow_signal_publish_rps),
			workflow_complete_publish_rps: Rate::new(config.workflow_complete_publish_rps),
			udb_hot_counter_rps: Rate::new(config.udb_hot_counter_rps),
			udb_read_scan_rps: Rate::new(config.udb_read_scan_rps),
			udb_conflict_rps: Rate::new(config.udb_conflict_rps),
		}
	}
}

#[derive(Clone)]
struct Workload {
	delay_ms: Arc<AtomicU64>,
	cpu_us: Arc<AtomicU64>,
}

impl Workload {
	fn new(delay_ms: u64, cpu_us: u64) -> Self {
		Self {
			delay_ms: Arc::new(AtomicU64::new(delay_ms)),
			cpu_us: Arc::new(AtomicU64::new(cpu_us)),
		}
	}

	fn store_delay_ms(&self, value: u64) {
		self.delay_ms.store(value, Ordering::Relaxed);
	}

	fn store_cpu_us(&self, value: u64) {
		self.cpu_us.store(value, Ordering::Relaxed);
	}

	async fn run(&self) {
		let cpu_us = self.cpu_us.load(Ordering::Relaxed);
		if cpu_us > 0 {
			burn_cpu(Duration::from_micros(cpu_us));
		}

		let delay_ms = self.delay_ms.load(Ordering::Relaxed);
		if delay_ms > 0 {
			tokio::time::sleep(Duration::from_millis(delay_ms)).await;
		}
	}
}

struct Workloads {
	gateway: Workload,
	envoy: Workload,
	envoy_eviction: Workload,
	worker_bump: Workload,
	serverless: Workload,
	cache_purge: Workload,
	tracing_config: Workload,
	route: Workload,
	workflow_signal: Workload,
}

impl Workloads {
	fn new(config: &Config) -> Self {
		Self {
			gateway: Workload::new(config.gateway_work_delay_ms, config.gateway_work_cpu_us),
			envoy: Workload::new(config.envoy_work_delay_ms, config.envoy_work_cpu_us),
			envoy_eviction: Workload::new(
				config.envoy_eviction_work_delay_ms,
				config.envoy_eviction_work_cpu_us,
			),
			worker_bump: Workload::new(
				config.worker_bump_work_delay_ms,
				config.worker_bump_work_cpu_us,
			),
			serverless: Workload::new(
				config.serverless_work_delay_ms,
				config.serverless_work_cpu_us,
			),
			cache_purge: Workload::new(
				config.cache_purge_work_delay_ms,
				config.cache_purge_work_cpu_us,
			),
			tracing_config: Workload::new(
				config.tracing_config_work_delay_ms,
				config.tracing_config_work_cpu_us,
			),
			route: Workload::new(config.route_work_delay_ms, config.route_work_cpu_us),
			workflow_signal: Workload::new(
				config.workflow_signal_work_delay_ms,
				config.workflow_signal_work_cpu_us,
			),
		}
	}
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TunePatch {
	gateway_publish_rps: Option<f64>,
	gateway_work_delay_ms: Option<u64>,
	gateway_work_cpu_us: Option<u64>,
	envoy_request_rps: Option<f64>,
	envoy_work_delay_ms: Option<u64>,
	envoy_work_cpu_us: Option<u64>,
	envoy_eviction_broadcast_rps: Option<f64>,
	envoy_eviction_work_delay_ms: Option<u64>,
	envoy_eviction_work_cpu_us: Option<u64>,
	worker_bump_broadcast_rps: Option<f64>,
	worker_bump_work_delay_ms: Option<u64>,
	worker_bump_work_cpu_us: Option<u64>,
	serverless_publish_rps: Option<f64>,
	serverless_work_delay_ms: Option<u64>,
	serverless_work_cpu_us: Option<u64>,
	cache_purge_broadcast_rps: Option<f64>,
	cache_purge_work_delay_ms: Option<u64>,
	cache_purge_work_cpu_us: Option<u64>,
	tracing_config_broadcast_rps: Option<f64>,
	tracing_config_work_delay_ms: Option<u64>,
	tracing_config_work_cpu_us: Option<u64>,
	route_churn_rps: Option<f64>,
	route_work_delay_ms: Option<u64>,
	route_work_cpu_us: Option<u64>,
	workflow_signal_churn_rps: Option<f64>,
	workflow_signal_publish_rps: Option<f64>,
	workflow_signal_work_delay_ms: Option<u64>,
	workflow_signal_work_cpu_us: Option<u64>,
	workflow_complete_publish_rps: Option<f64>,
	udb_hot_counter_rps: Option<f64>,
	udb_read_scan_rps: Option<f64>,
	udb_conflict_rps: Option<f64>,
}

impl TunePatch {
	fn apply(&self, rates: &Rates, workloads: &Workloads) -> Result<()> {
		apply_rate(
			"gateway_publish_rps",
			self.gateway_publish_rps,
			&rates.gateway_publish_rps,
		)?;
		apply_workload(
			"gateway",
			self.gateway_work_delay_ms,
			self.gateway_work_cpu_us,
			&workloads.gateway,
		);
		apply_rate(
			"envoy_request_rps",
			self.envoy_request_rps,
			&rates.envoy_request_rps,
		)?;
		apply_workload(
			"envoy",
			self.envoy_work_delay_ms,
			self.envoy_work_cpu_us,
			&workloads.envoy,
		);
		apply_rate(
			"envoy_eviction_broadcast_rps",
			self.envoy_eviction_broadcast_rps,
			&rates.envoy_eviction_broadcast_rps,
		)?;
		apply_workload(
			"envoy_eviction",
			self.envoy_eviction_work_delay_ms,
			self.envoy_eviction_work_cpu_us,
			&workloads.envoy_eviction,
		);
		apply_rate(
			"worker_bump_broadcast_rps",
			self.worker_bump_broadcast_rps,
			&rates.worker_bump_broadcast_rps,
		)?;
		apply_workload(
			"worker_bump",
			self.worker_bump_work_delay_ms,
			self.worker_bump_work_cpu_us,
			&workloads.worker_bump,
		);
		apply_rate(
			"serverless_publish_rps",
			self.serverless_publish_rps,
			&rates.serverless_publish_rps,
		)?;
		apply_workload(
			"serverless",
			self.serverless_work_delay_ms,
			self.serverless_work_cpu_us,
			&workloads.serverless,
		);
		apply_rate(
			"cache_purge_broadcast_rps",
			self.cache_purge_broadcast_rps,
			&rates.cache_purge_broadcast_rps,
		)?;
		apply_workload(
			"cache_purge",
			self.cache_purge_work_delay_ms,
			self.cache_purge_work_cpu_us,
			&workloads.cache_purge,
		);
		apply_rate(
			"tracing_config_broadcast_rps",
			self.tracing_config_broadcast_rps,
			&rates.tracing_config_broadcast_rps,
		)?;
		apply_workload(
			"tracing_config",
			self.tracing_config_work_delay_ms,
			self.tracing_config_work_cpu_us,
			&workloads.tracing_config,
		);
		apply_rate(
			"route_churn_rps",
			self.route_churn_rps,
			&rates.route_churn_rps,
		)?;
		apply_workload(
			"route",
			self.route_work_delay_ms,
			self.route_work_cpu_us,
			&workloads.route,
		);
		apply_rate(
			"workflow_signal_churn_rps",
			self.workflow_signal_churn_rps,
			&rates.workflow_signal_churn_rps,
		)?;
		apply_rate(
			"workflow_signal_publish_rps",
			self.workflow_signal_publish_rps,
			&rates.workflow_signal_publish_rps,
		)?;
		apply_workload(
			"workflow_signal",
			self.workflow_signal_work_delay_ms,
			self.workflow_signal_work_cpu_us,
			&workloads.workflow_signal,
		);
		apply_rate(
			"workflow_complete_publish_rps",
			self.workflow_complete_publish_rps,
			&rates.workflow_complete_publish_rps,
		)?;
		apply_rate(
			"udb_hot_counter_rps",
			self.udb_hot_counter_rps,
			&rates.udb_hot_counter_rps,
		)?;
		apply_rate(
			"udb_read_scan_rps",
			self.udb_read_scan_rps,
			&rates.udb_read_scan_rps,
		)?;
		apply_rate(
			"udb_conflict_rps",
			self.udb_conflict_rps,
			&rates.udb_conflict_rps,
		)?;

		Ok(())
	}
}

fn apply_rate(name: &'static str, value: Option<f64>, rate: &Rate) -> Result<()> {
	if let Some(value) = value {
		validate_rate(name, value)?;
		rate.store(value);
	}

	Ok(())
}

fn apply_workload(
	name: &'static str,
	delay_ms: Option<u64>,
	cpu_us: Option<u64>,
	workload: &Workload,
) {
	if let Some(delay_ms) = delay_ms {
		workload.store_delay_ms(delay_ms);
		tracing::info!(name, delay_ms, "updated UPS simulation workload delay");
	}
	if let Some(cpu_us) = cpu_us {
		workload.store_cpu_us(cpu_us);
		tracing::info!(name, cpu_us, "updated UPS simulation workload CPU");
	}
}

pub async fn pubsub_for_sim(
	config: &rivet_config::Config,
	existing: &PubSub,
	force_driver: bool,
	disable_memory_optimization: bool,
) -> Result<PubSub> {
	if !force_driver {
		return Ok(existing.clone());
	}

	let mut root = (**config).clone();
	let mut pubsub = config.pubsub().clone();
	match &mut pubsub {
		rivet_config::config::PubSub::Nats(nats) => {
			nats.disable_memory_optimization = disable_memory_optimization;
		}
		rivet_config::config::PubSub::PostgresNotify(postgres) => {
			postgres.disable_memory_optimization = disable_memory_optimization;
		}
		rivet_config::config::PubSub::Memory(memory) => {
			memory.disable_memory_optimization = disable_memory_optimization;
		}
	}
	root.pubsub = Some(pubsub);

	let sim_config = rivet_config::Config::from_root(root);
	rivet_pools::db::ups::setup(&sim_config, "rivet-ups-broadcast-sim")
		.await
		.context("failed to create UPS simulation pubsub")
}

pub fn spawn(ups: PubSub, udb: Option<UdbPool>, config: Config) {
	let rates = Arc::new(Rates::new(&config));
	let workloads = Arc::new(Workloads::new(&config));
	let workflow_signal_subjects = ActiveSubjects::default();

	tracing::info!(
		force_driver = config.force_driver,
		disable_memory_optimization = config.disable_memory_optimization,
		tune_path = ?config.tune_path,
		gateway_publish_rps = config.gateway_publish_rps,
		gateway_spread_replicas = config.gateway_spread_replicas,
		envoy_request_rps = config.envoy_request_rps,
		worker_bump_broadcast_rps = config.worker_bump_broadcast_rps,
		worker_bump_work_delay_ms = config.worker_bump_work_delay_ms,
		envoy_work_delay_ms = config.envoy_work_delay_ms,
		workflow_signal_work_delay_ms = config.workflow_signal_work_delay_ms,
		workflow_signal_churn_rps = config.workflow_signal_churn_rps,
		udb_hot_counter_rps = config.udb_hot_counter_rps,
		udb_read_scan_rps = config.udb_read_scan_rps,
		udb_read_scan_seed_keys = config.udb_read_scan_seed_keys,
		udb_read_scan_keys_per_tx = config.udb_read_scan_keys_per_tx,
		udb_read_scan_unpack_keys = config.udb_read_scan_unpack_keys,
		udb_conflict_rps = config.udb_conflict_rps,
		udb_conflict_keys = config.udb_conflict_keys,
		"starting UPS broadcast traffic simulator"
	);

	spawn_tuner(
		ups.clone(),
		rates.clone(),
		workloads.clone(),
		config.tune_path.clone(),
	);

	spawn_gateway_subscribers(
		ups.clone(),
		udb.clone(),
		config.gateway_subjects,
		config.gateway_subscribers,
		config.gateway_spread_replicas,
		Duration::from_millis(config.gateway_spread_member_ttl_ms),
		workloads.gateway.clone(),
	);
	let envoy_queue_group = config.envoy_queue_group.clone().map(Arc::new);
	spawn_subject_subscribers(
		ups.clone(),
		"envoy",
		"pegboard.envoy",
		"pegboard.envoy.sim",
		config.envoy_subjects,
		config.envoy_responders,
		envoy_queue_group,
		Some(Arc::new(Vec::new())),
		workloads.envoy.clone(),
	);
	spawn_subject_subscribers(
		ups.clone(),
		"envoy eviction",
		"pegboard.envoy.eviction",
		"pegboard.envoy.eviction.sim",
		config.envoy_subjects,
		config.envoy_eviction_subscribers,
		None,
		None,
		workloads.envoy_eviction.clone(),
	);
	spawn_worker_bump_subscribers(
		ups.clone(),
		SimSubject::new("gasoline.worker.bump", "gasoline.worker.bump"),
		config.worker_bump_subscribers,
		workloads.worker_bump.clone(),
	);
	spawn_same_subject_subscribers(
		ups.clone(),
		"serverless outbound",
		SimSubject::new(
			"pegboard.serverless.outbound",
			"pegboard.serverless.outbound",
		),
		config.serverless_subscribers,
		None,
		workloads.serverless.clone(),
	);
	spawn_same_subject_subscribers(
		ups.clone(),
		"cache purge",
		SimSubject::new("rivet.cache.purge", "rivet.cache.purge"),
		config.cache_purge_subscribers,
		None,
		workloads.cache_purge.clone(),
	);
	spawn_same_subject_subscribers(
		ups.clone(),
		"tracing config",
		SimSubject::new("rivet.debug.tracing.config", "rivet.debug.tracing.config"),
		config.tracing_config_subscribers,
		None,
		workloads.tracing_config.clone(),
	);
	spawn_subject_subscribers(
		ups.clone(),
		"route stopped",
		"gasoline.msg.pegboard_actor2_stopped",
		"gasoline.msg.pegboard_actor2_stopped:actor",
		config.route_stopped_subscribers,
		config.route_stopped_subscribers,
		None,
		None,
		Workload::new(0, 0),
	);

	let gateway_subjects = subjects(
		"pegboard.gateway",
		"pegboard.gateway.sim",
		config.gateway_subjects,
	);
	spawn_publish_rate(
		ups.clone(),
		"gateway publish",
		gateway_subjects,
		PublishOpts::one(),
		rates.gateway_publish_rps.clone(),
		Arc::new(payload(config.gateway_payload_bytes)),
	);

	if config.envoy_request_unknown_root {
		spawn_request_rate(
			ups.clone(),
			raw_subjects("pegboard.envoy.sim", config.envoy_subjects),
			rates.envoy_request_rps.clone(),
			Arc::new(payload(config.envoy_request_payload_bytes)),
			Duration::from_millis(config.envoy_request_timeout_ms),
			config.envoy_request_max_in_flight,
		);
	} else {
		spawn_request_rate(
			ups.clone(),
			subjects(
				"pegboard.envoy",
				"pegboard.envoy.sim",
				config.envoy_subjects,
			),
			rates.envoy_request_rps.clone(),
			Arc::new(payload(config.envoy_request_payload_bytes)),
			Duration::from_millis(config.envoy_request_timeout_ms),
			config.envoy_request_max_in_flight,
		);
	}

	let envoy_eviction_subjects = subjects(
		"pegboard.envoy.eviction",
		"pegboard.envoy.eviction.sim",
		config.envoy_subjects,
	);
	spawn_publish_rate(
		ups.clone(),
		"envoy eviction broadcast",
		envoy_eviction_subjects,
		PublishOpts::broadcast(),
		rates.envoy_eviction_broadcast_rps.clone(),
		Arc::new(Vec::new()),
	);
	spawn_publish_rate(
		ups.clone(),
		"worker bump broadcast",
		vec![SimSubject::new(
			"gasoline.worker.bump",
			"gasoline.worker.bump",
		)],
		PublishOpts::broadcast(),
		rates.worker_bump_broadcast_rps.clone(),
		Arc::new(Vec::new()),
	);
	spawn_publish_rate(
		ups.clone(),
		"serverless publish",
		vec![SimSubject::new(
			"pegboard.serverless.outbound",
			"pegboard.serverless.outbound",
		)],
		PublishOpts::one(),
		rates.serverless_publish_rps.clone(),
		Arc::new(payload(config.serverless_payload_bytes)),
	);
	spawn_publish_rate(
		ups.clone(),
		"cache purge broadcast",
		vec![SimSubject::new("rivet.cache.purge", "rivet.cache.purge")],
		PublishOpts::broadcast(),
		rates.cache_purge_broadcast_rps.clone(),
		Arc::new(payload(config.cache_purge_payload_bytes)),
	);
	spawn_publish_rate(
		ups.clone(),
		"tracing config broadcast",
		vec![SimSubject::new(
			"rivet.debug.tracing.config",
			"rivet.debug.tracing.config",
		)],
		PublishOpts::broadcast(),
		rates.tracing_config_broadcast_rps.clone(),
		Arc::new(payload(config.tracing_config_payload_bytes)),
	);
	spawn_publish_active_rate(
		ups.clone(),
		"workflow signal broadcast",
		workflow_signal_subjects.clone(),
		PublishOpts::broadcast(),
		rates.workflow_signal_publish_rps.clone(),
		Arc::new(Vec::new()),
	);
	spawn_publish_rate(
		ups.clone(),
		"workflow complete broadcast",
		vec![unique_subject(
			"gasoline.workflow.complete",
			"gasoline.workflow.complete",
		)],
		PublishOpts::broadcast(),
		rates.workflow_complete_publish_rps.clone(),
		Arc::new(Vec::new()),
	);

	spawn_route_churn(
		ups.clone(),
		rates.route_churn_rps.clone(),
		Duration::from_millis(config.route_ephemeral_hold_ms),
		Duration::from_millis(config.route_stopped_hold_ms),
		config.route_max_in_flight,
		workloads.route.clone(),
	);
	spawn_subscription_churn(
		ups,
		"workflow signal churn",
		"gasoline.signal.for-workflow",
		"gasoline.signal.for-workflow",
		rates.workflow_signal_churn_rps.clone(),
		Duration::from_millis(config.workflow_signal_hold_ms),
		Some(workflow_signal_subjects),
		workloads.workflow_signal.clone(),
	);

	spawn_udb_hot_counter(
		udb.clone(),
		rates.udb_hot_counter_rps.clone(),
		config.udb_hot_counter_max_in_flight,
		config.udb_hot_counter_namespace_id,
		config.udb_hot_counter_actor_name,
	);
	spawn_udb_read_scan(
		udb.clone(),
		rates.udb_read_scan_rps.clone(),
		config.udb_read_scan_max_in_flight,
		config.udb_read_scan_seed_keys,
		config.udb_read_scan_keys_per_tx,
		config.udb_read_scan_value_bytes,
		config.udb_read_scan_unpack_keys,
	);
	spawn_udb_conflict(
		udb,
		rates.udb_conflict_rps.clone(),
		config.udb_conflict_max_in_flight,
		config.udb_conflict_keys,
	);
}

fn spawn_tuner(
	ups: PubSub,
	rates: Arc<Rates>,
	workloads: Arc<Workloads>,
	tune_path: Option<String>,
) {
	let tune_subject = tune_subject();
	{
		let ups = ups.clone();
		let rates = rates.clone();
		let workloads = workloads.clone();
		let tune_subject = tune_subject.clone();
		tokio::spawn(async move {
			loop {
				let mut sub = match ups.subscribe(tune_subject.clone()).await {
					Ok(sub) => sub,
					Err(err) => {
						tracing::warn!(?err, "failed to subscribe to UPS simulation tune subject");
						tokio::time::sleep(Duration::from_secs(2)).await;
						continue;
					}
				};

				loop {
					match sub.next().await {
						Ok(NextOutput::Message(message)) => {
							if let Err(err) =
								apply_tune_patch_bytes(&message.payload, &rates, &workloads)
							{
								tracing::warn!(?err, "failed to apply UPS simulation tune message");
							}
						}
						Ok(NextOutput::Unsubscribed | NextOutput::NoResponders) => break,
						Err(err) => {
							tracing::warn!(?err, "UPS simulation tune subscriber failed");
							break;
						}
					}
				}
			}
		});
	}

	if let Some(path) = tune_path {
		tokio::spawn(async move {
			let mut last_payload = None::<Vec<u8>>;

			loop {
				match tokio::fs::read(&path).await {
					Ok(payload) if Some(&payload) != last_payload.as_ref() => {
						match apply_tune_patch_bytes(&payload, &rates, &workloads) {
							Ok(()) => {
								last_payload = Some(payload.clone());
								if let Err(err) = ups
									.publish(
										tune_subject.clone(),
										&payload,
										PublishOpts::broadcast(),
									)
									.await
								{
									tracing::warn!(
										?err,
										%path,
										"failed to broadcast UPS simulation tune patch"
									);
								}
							}
							Err(err) => {
								tracing::warn!(
									?err,
									%path,
									"failed to apply UPS simulation tune file"
								);
							}
						}
					}
					Ok(_) => {}
					Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
					Err(err) => {
						tracing::debug!(?err, %path, "failed to read UPS simulation tune file");
					}
				}

				tokio::time::sleep(TUNE_POLL_INTERVAL).await;
			}
		});
	}
}

fn apply_tune_patch_bytes(payload: &[u8], rates: &Rates, workloads: &Workloads) -> Result<()> {
	let patch: TunePatch =
		serde_json::from_slice(payload).context("failed to parse UPS simulation tune patch")?;
	patch.apply(rates, workloads)?;
	tracing::info!(?patch, "applied UPS simulation tune patch");
	Ok(())
}

fn tune_subject() -> SimSubject {
	SimSubject::new(TUNE_SUBJECT, TUNE_SUBJECT_ROOT)
}

fn spawn_subject_subscribers(
	ups: PubSub,
	label: &'static str,
	root: &'static str,
	prefix: &'static str,
	subject_count: usize,
	subscriber_count: usize,
	queue_group: Option<Arc<String>>,
	reply_payload: Option<Arc<Vec<u8>>>,
	workload: Workload,
) {
	if subject_count == 0 || subscriber_count == 0 {
		return;
	}

	spawn_subject_subscribers_with_offset(
		ups,
		label,
		root,
		prefix,
		subject_count,
		subscriber_count,
		0,
		queue_group,
		reply_payload,
		workload,
	);
}

fn spawn_subject_subscribers_with_offset(
	ups: PubSub,
	label: &'static str,
	root: &'static str,
	prefix: &'static str,
	subject_count: usize,
	subscriber_count: usize,
	subject_offset: usize,
	queue_group: Option<Arc<String>>,
	reply_payload: Option<Arc<Vec<u8>>>,
	workload: Workload,
) {
	for idx in 0..subscriber_count {
		let subject_idx = subject_offset.wrapping_add(idx) % subject_count;
		let subject = SimSubject::new(format!("{prefix}.{subject_idx}"), root);
		spawn_subscriber(
			ups.clone(),
			label,
			subject,
			queue_group.clone(),
			reply_payload.clone(),
			workload.clone(),
		);
	}
}

fn spawn_gateway_subscribers(
	ups: PubSub,
	udb: Option<UdbPool>,
	subject_count: usize,
	subscriber_count: usize,
	spread_replicas: usize,
	member_ttl: Duration,
	workload: Workload,
) {
	if subject_count == 0 || subscriber_count == 0 {
		return;
	}

	let Some(udb) = udb.filter(|_| spread_replicas > 1 && subject_count > subscriber_count) else {
		spawn_subject_subscribers(
			ups,
			"gateway",
			"pegboard.gateway",
			"pegboard.gateway.sim",
			subject_count,
			subscriber_count,
			None,
			None,
			workload,
		);
		return;
	};

	tokio::spawn(async move {
		let member_id = gateway_member_id();

		loop {
			match gateway_subject_offset(
				&udb,
				&member_id,
				spread_replicas,
				subscriber_count,
				member_ttl,
			)
			.await
			{
				Ok(Some(subject_offset)) => {
					spawn_gateway_membership_heartbeat(udb.clone(), member_id.clone(), member_ttl);
					tracing::info!(
						member_id,
						subject_offset,
						subject_count,
						subscriber_count,
						spread_replicas,
						"starting spread gateway UPS simulation subscribers"
					);
					spawn_subject_subscribers_with_offset(
						ups,
						"gateway",
						"pegboard.gateway",
						"pegboard.gateway.sim",
						subject_count,
						subscriber_count,
						subject_offset,
						None,
						None,
						workload,
					);
					return;
				}
				Ok(None) => {}
				Err(err) => {
					tracing::warn!(
						?err,
						member_id,
						"failed to assign gateway UPS simulation subjects"
					);
				}
			}

			tokio::time::sleep(Duration::from_secs(2)).await;
		}
	});
}

fn spawn_gateway_membership_heartbeat(udb: UdbPool, member_id: String, member_ttl: Duration) {
	let interval = (member_ttl / 3).max(Duration::from_secs(1));

	tokio::spawn(async move {
		loop {
			if let Err(err) = gateway_refresh_member(&udb, &member_id).await {
				tracing::warn!(
					?err,
					member_id,
					"failed to refresh gateway UPS simulation membership"
				);
			}

			tokio::time::sleep(interval).await;
		}
	});
}

async fn gateway_refresh_member(udb: &UdbPool, member_id: &str) -> Result<()> {
	let member_id = member_id.to_string();
	udb.txn(GATEWAY_MEMBERSHIP_TX, |tx| {
		let member_id = member_id.clone();
		async move {
			let now = now_ms();
			let group = gateway_member_group(&member_id);
			let prefix = gateway_member_prefix(&group);
			let member_key = gateway_member_key(&prefix, &member_id);
			tx.informal().set(&member_key, &now.to_be_bytes());
			Ok(())
		}
	})
	.await
}

async fn gateway_subject_offset(
	udb: &UdbPool,
	member_id: &str,
	expected_replicas: usize,
	subscriber_count: usize,
	member_ttl: Duration,
) -> Result<Option<usize>> {
	let member_id = member_id.to_string();
	let member_ttl_ms = duration_millis_u64(member_ttl);
	let members = udb
		.txn(GATEWAY_MEMBERSHIP_TX, |tx| {
			let member_id = member_id.clone();
			async move {
				let now = now_ms();
				let group = gateway_member_group(&member_id);
				let prefix = gateway_member_prefix(&group);
				let member_key = gateway_member_key(&prefix, &member_id);
				tx.informal().set(&member_key, &now.to_be_bytes());

				let mut end = prefix.clone();
				end.push(0xff);
				let mut range: RangeOption<'static> = (prefix.clone()..end).into();
				range.limit = Some(expected_replicas.saturating_mul(4).max(32));

				let min_fresh = now.saturating_sub(member_ttl_ms);
				let informal = tx.informal();
				let mut stream = informal.get_ranges_keyvalues(range, Snapshot);
				let mut members = Vec::new();
				while let Some(entry) = stream.next().await {
					let entry = entry?;
					let value = entry.value();
					if value.len() != 8 {
						continue;
					}

					let mut ts = [0; 8];
					ts.copy_from_slice(value);
					if u64::from_be_bytes(ts) < min_fresh {
						continue;
					}

					if let Some(member) = gateway_member_from_key(&prefix, entry.key()) {
						members.push(member);
					}
				}

				Ok(members)
			}
		})
		.await?;

	let mut members = members;
	members.sort();
	members.dedup();

	if members.len() < expected_replicas {
		tracing::debug!(
			member_id,
			active_members = members.len(),
			expected_replicas,
			"waiting for stable gateway UPS simulation membership"
		);
		return Ok(None);
	}

	let Some(ordinal) = members.iter().position(|member| member == &member_id) else {
		return Ok(None);
	};

	Ok(Some(
		(ordinal % expected_replicas).saturating_mul(subscriber_count),
	))
}

fn gateway_member_id() -> String {
	env::var("HOSTNAME").unwrap_or_else(|_| format!("pid-{}", std::process::id()))
}

fn gateway_member_group(member_id: &str) -> String {
	member_id
		.rsplit_once('-')
		.map(|(group, _)| group)
		.unwrap_or(member_id)
		.to_string()
}

fn gateway_member_prefix(group: &str) -> Vec<u8> {
	let mut key = GATEWAY_MEMBERSHIP_PREFIX.to_vec();
	key.push(b'/');
	key.extend_from_slice(group.as_bytes());
	key.push(b'/');
	key
}

fn gateway_member_key(prefix: &[u8], member_id: &str) -> Vec<u8> {
	let mut key = prefix.to_vec();
	key.extend_from_slice(member_id.as_bytes());
	key
}

fn gateway_member_from_key(prefix: &[u8], key: &[u8]) -> Option<String> {
	key.strip_prefix(prefix)
		.and_then(|member| std::str::from_utf8(member).ok())
		.map(ToOwned::to_owned)
}

fn spawn_same_subject_subscribers(
	ups: PubSub,
	label: &'static str,
	subject: SimSubject,
	subscriber_count: usize,
	reply_payload: Option<Arc<Vec<u8>>>,
	workload: Workload,
) {
	for _ in 0..subscriber_count {
		spawn_subscriber(
			ups.clone(),
			label,
			subject.clone(),
			None,
			reply_payload.clone(),
			workload.clone(),
		);
	}
}

fn spawn_worker_bump_subscribers(
	ups: PubSub,
	subject: SimSubject,
	subscriber_count: usize,
	workload: Workload,
) {
	for _ in 0..subscriber_count {
		let ups = ups.clone();
		let subject = subject.clone();
		let workload = workload.clone();
		tokio::spawn(async move {
			loop {
				let mut sub = match ups.subscribe(subject.clone()).await {
					Ok(sub) => sub,
					Err(err) => {
						tracing::warn!(
							?err,
							%subject,
							"failed to subscribe for UPS worker bump simulation"
						);
						tokio::time::sleep(Duration::from_secs(2)).await;
						continue;
					}
				};

				loop {
					match sub.next().await {
						Ok(NextOutput::Message(_)) => {
							drain_ready_messages(&mut sub, "worker bump").await;
							workload.run().await;
						}
						Ok(NextOutput::Unsubscribed | NextOutput::NoResponders) => break,
						Err(err) => {
							tracing::warn!(
								?err,
								%subject,
								"UPS worker bump simulation subscriber failed"
							);
							break;
						}
					}
				}
			}
		});
	}
}

fn spawn_subscriber(
	ups: PubSub,
	label: &'static str,
	subject: SimSubject,
	queue_group: Option<Arc<String>>,
	reply_payload: Option<Arc<Vec<u8>>>,
	workload: Workload,
) {
	tokio::spawn(async move {
		loop {
			let sub_res = if let Some(queue_group) = queue_group.as_ref() {
				ups.queue_subscribe(subject.clone(), queue_group.as_str())
					.await
			} else {
				ups.subscribe(subject.clone()).await
			};
			let mut sub = match sub_res {
				Ok(sub) => sub,
				Err(err) => {
					tracing::warn!(?err, %subject, label, "failed to subscribe for UPS simulation");
					tokio::time::sleep(Duration::from_secs(2)).await;
					continue;
				}
			};

			loop {
				match sub.next().await {
					Ok(NextOutput::Message(message)) => {
						if let Some(reply_payload) = &reply_payload {
							if let Err(err) = message.reply(reply_payload).await {
								tracing::debug!(
									?err,
									%subject,
									label,
									"failed to reply to UPS simulation message"
								);
							}
						}
						workload.run().await;
					}
					Ok(NextOutput::Unsubscribed | NextOutput::NoResponders) => break,
					Err(err) => {
						tracing::warn!(
							?err,
							%subject,
							label,
							"UPS simulation subscriber failed"
						);
						break;
					}
				}
			}
		}
	});
}

async fn drain_ready_messages(sub: &mut Subscriber, label: &'static str) {
	for _ in 0..1023 {
		match sub.next().now_or_never() {
			Some(Ok(NextOutput::Message(_))) => {}
			Some(Ok(NextOutput::Unsubscribed | NextOutput::NoResponders)) | None => break,
			Some(Err(err)) => {
				tracing::debug!(?err, label, "failed to drain UPS simulation messages");
				break;
			}
		}
	}
}

fn burn_cpu(duration: Duration) {
	let start = Instant::now();
	let mut value = 0u64;
	while start.elapsed() < duration {
		value = value.wrapping_add(1);
		hint::black_box(value);
	}
}

fn spawn_publish_rate<S>(
	ups: PubSub,
	label: &'static str,
	subjects: Vec<S>,
	opts: PublishOpts,
	rate: Rate,
	payload: Arc<Vec<u8>>,
) where
	S: Subject + Clone + Send + Sync + 'static,
{
	if subjects.is_empty() {
		return;
	}

	tokio::spawn(async move {
		let mut pacer = Pacer::new();
		let mut idx = 0usize;
		let semaphore = Arc::new(tokio::sync::Semaphore::new(PUBLISH_MAX_IN_FLIGHT));

		loop {
			let count = pacer.next_count(rate.load()).await;
			for _ in 0..count {
				let Ok(permit) = semaphore.clone().try_acquire_owned() else {
					continue;
				};
				let subject = subjects[idx % subjects.len()].clone();
				idx = idx.wrapping_add(1);
				let ups = ups.clone();
				let payload = payload.clone();
				tokio::spawn(async move {
					let _permit = permit;
					if let Err(err) = ups.publish(subject, &payload, opts).await {
						tracing::warn!(?err, label, "UPS simulation publish failed");
					}
				});
			}
		}
	});
}

#[derive(Clone, Default)]
struct ActiveSubjects {
	subjects: Arc<tokio::sync::RwLock<Vec<SimSubject>>>,
}

impl ActiveSubjects {
	async fn insert(&self, subject: SimSubject) {
		self.subjects.write().await.push(subject);
	}

	async fn remove(&self, subject: &SimSubject) {
		self.subjects
			.write()
			.await
			.retain(|existing| existing.subject != subject.subject);
	}

	async fn get(&self, idx: usize) -> Option<SimSubject> {
		let subjects = self.subjects.read().await;
		if subjects.is_empty() {
			None
		} else {
			Some(subjects[idx % subjects.len()].clone())
		}
	}
}

fn spawn_publish_active_rate(
	ups: PubSub,
	label: &'static str,
	subjects: ActiveSubjects,
	opts: PublishOpts,
	rate: Rate,
	payload: Arc<Vec<u8>>,
) {
	tokio::spawn(async move {
		let mut pacer = Pacer::new();
		let mut idx = 0usize;
		let semaphore = Arc::new(tokio::sync::Semaphore::new(PUBLISH_MAX_IN_FLIGHT));

		loop {
			let count = pacer.next_count(rate.load()).await;
			for _ in 0..count {
				let Some(subject) = subjects.get(idx).await else {
					continue;
				};
				idx = idx.wrapping_add(1);

				let Ok(permit) = semaphore.clone().try_acquire_owned() else {
					continue;
				};
				let ups = ups.clone();
				let payload = payload.clone();
				tokio::spawn(async move {
					let _permit = permit;
					if let Err(err) = ups.publish(subject, &payload, opts).await {
						tracing::warn!(?err, label, "UPS simulation publish failed");
					}
				});
			}
		}
	});
}

fn spawn_request_rate<S>(
	ups: PubSub,
	subjects: Vec<S>,
	rate: Rate,
	payload: Arc<Vec<u8>>,
	timeout: Duration,
	max_in_flight: usize,
) where
	S: Subject + Clone + Send + Sync + 'static,
{
	if subjects.is_empty() || max_in_flight == 0 {
		return;
	}

	tokio::spawn(async move {
		let mut pacer = Pacer::new();
		let mut idx = 0usize;
		let semaphore = Arc::new(tokio::sync::Semaphore::new(max_in_flight));

		loop {
			let count = pacer.next_count(rate.load()).await;
			for _ in 0..count {
				let Ok(permit) = semaphore.clone().try_acquire_owned() else {
					continue;
				};
				let subject = subjects[idx % subjects.len()].clone();
				idx = idx.wrapping_add(1);
				let ups = ups.clone();
				let payload = payload.clone();
				tokio::spawn(async move {
					let _permit = permit;
					if let Err(err) = ups.request_with_timeout(subject, &payload, timeout).await {
						tracing::debug!(?err, "UPS simulation request failed");
					}
				});
			}
		}
	});
}

fn spawn_udb_hot_counter(
	udb: Option<UdbPool>,
	rate: Rate,
	max_in_flight: usize,
	namespace_id: Id,
	actor_name: String,
) {
	if max_in_flight == 0 {
		return;
	}

	let Some(udb) = udb else {
		if rate.load() > 0.0 {
			tracing::warn!("UPS simulation UDB hot counter enabled without a UDB pool");
		}
		return;
	};

	tokio::spawn(async move {
		let mut pacer = Pacer::new();
		let semaphore = Arc::new(tokio::sync::Semaphore::new(max_in_flight));

		loop {
			let count = pacer.next_count(rate.load()).await;
			for _ in 0..count {
				let Ok(permit) = semaphore.clone().try_acquire_owned() else {
					continue;
				};
				let udb = udb.clone();
				let actor_name = actor_name.clone();
				tokio::spawn(async move {
					let _permit = permit;
					let is_open = HOT_COUNTER_SEQ.fetch_add(1, Ordering::Relaxed) % 2 == 0;
					let res = udb
						.txn(UDB_HOT_COUNTER_TX, |tx| {
							let actor_name = actor_name.clone();
							async move {
								let tx = tx.with_subspace(namespace::keys::subspace());
								if is_open {
									namespace::keys::metric::inc(
										&tx,
										namespace_id,
										namespace::keys::metric::Metric::Requests(
											actor_name.clone(),
											"ws".to_string(),
										),
										1,
									);
									namespace::keys::metric::inc(
										&tx,
										namespace_id,
										namespace::keys::metric::Metric::ActiveRequests(
											actor_name,
											"ws".to_string(),
										),
										1,
									);
								} else {
									namespace::keys::metric::inc(
										&tx,
										namespace_id,
										namespace::keys::metric::Metric::ActiveRequests(
											actor_name,
											"ws".to_string(),
										),
										-1,
									);
								}

								Ok(())
							}
						})
						.await;

					if let Err(err) = res {
						tracing::debug!(?err, "UPS simulation UDB hot counter transaction failed");
					}
				});
			}
		}
	});
}

#[derive(Debug, Clone, Copy)]
struct ReadScanKey {
	shard: u64,
	index: u64,
}

impl TuplePack for ReadScanKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (READ_SCAN_KEY_ROOT, self.shard, self.index);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ReadScanKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (root, shard, index)) = <(usize, u64, u64)>::unpack(input, tuple_depth)?;
		if root != READ_SCAN_KEY_ROOT {
			return Err(PackError::Message("expected READ_SCAN key root".into()));
		}

		Ok((input, Self { shard, index }))
	}
}

#[derive(Debug, Clone, Copy)]
struct ConflictKey {
	index: u64,
}

impl TuplePack for ConflictKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (CONFLICT_KEY_ROOT, self.index);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ConflictKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (root, index)) = <(usize, u64)>::unpack(input, tuple_depth)?;
		if root != CONFLICT_KEY_ROOT {
			return Err(PackError::Message("expected CONFLICT key root".into()));
		}

		Ok((input, Self { index }))
	}
}

fn sim_read_scan_subspace() -> Subspace {
	Subspace::new(&("rivet", "ups-broadcast", "sim", "read-scan"))
}

fn sim_conflict_subspace() -> Subspace {
	Subspace::new(&("rivet", "ups-broadcast", "sim", "conflict"))
}

fn read_scan_shard() -> u64 {
	let member_id = gateway_member_id();
	let mut hash = 0xcbf2_9ce4_8422_2325u64;
	for byte in member_id.as_bytes() {
		hash ^= u64::from(*byte);
		hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
	}
	hash
}

fn spawn_udb_read_scan(
	udb: Option<UdbPool>,
	rate: Rate,
	max_in_flight: usize,
	seed_keys: u64,
	keys_per_tx: usize,
	value_bytes: usize,
	unpack_keys: bool,
) {
	if max_in_flight == 0 || keys_per_tx == 0 || seed_keys == 0 {
		if rate.load() > 0.0 {
			tracing::warn!(
				max_in_flight,
				seed_keys,
				keys_per_tx,
				"UPS simulation UDB read scan is enabled without enough configuration"
			);
		}
		return;
	}

	let Some(udb) = udb else {
		if rate.load() > 0.0 {
			tracing::warn!("UPS simulation UDB read scan enabled without a UDB pool");
		}
		return;
	};

	tokio::spawn(async move {
		let shard = read_scan_shard();
		if let Err(err) = seed_udb_read_scan(&udb, shard, seed_keys, value_bytes).await {
			tracing::warn!(
				?err,
				shard,
				"failed to seed UPS simulation UDB read scan keys"
			);
		}

		let mut pacer = Pacer::new();
		let semaphore = Arc::new(tokio::sync::Semaphore::new(max_in_flight));

		loop {
			let count = pacer.next_count(rate.load()).await;
			for _ in 0..count {
				let Ok(permit) = semaphore.clone().try_acquire_owned() else {
					continue;
				};
				let udb = udb.clone();
				tokio::spawn(async move {
					let _permit = permit;
					if let Err(err) =
						run_udb_read_scan(&udb, shard, seed_keys, keys_per_tx, unpack_keys).await
					{
						tracing::debug!(?err, "UPS simulation UDB read scan transaction failed");
					}
				});
			}
		}
	});
}

async fn seed_udb_read_scan(
	udb: &UdbPool,
	shard: u64,
	seed_keys: u64,
	value_bytes: usize,
) -> Result<()> {
	let value = Arc::new(payload(value_bytes));
	let mut start = 0;

	tracing::info!(
		shard,
		seed_keys,
		value_bytes,
		"seeding UPS simulation UDB read scan keys"
	);

	while start < seed_keys {
		let end = start
			.saturating_add(UDB_READ_SCAN_SEED_BATCH_SIZE)
			.min(seed_keys);
		let value = value.clone();
		udb.txn(UDB_READ_SCAN_SEED_TX, |tx| {
			let value = value.clone();
			async move {
				let tx = tx.with_subspace(sim_read_scan_subspace());
				for index in start..end {
					let key = tx.pack(&ReadScanKey { shard, index });
					tx.set(&key, value.as_slice());
				}

				Ok(())
			}
		})
		.await?;
		start = end;
	}

	tracing::info!(shard, seed_keys, "seeded UPS simulation UDB read scan keys");
	Ok(())
}

async fn run_udb_read_scan(
	udb: &UdbPool,
	shard: u64,
	seed_keys: u64,
	keys_per_tx: usize,
	unpack_keys: bool,
) -> Result<()> {
	let keys_per_tx_u64 = u64::try_from(keys_per_tx)
		.unwrap_or(u64::MAX)
		.min(seed_keys);
	let start = READ_SCAN_SEQ.fetch_add(keys_per_tx_u64, Ordering::Relaxed) % seed_keys;
	let end = start.saturating_add(keys_per_tx_u64).min(seed_keys);
	let limit = usize::try_from(end.saturating_sub(start)).unwrap_or(keys_per_tx);

	udb.txn(UDB_READ_SCAN_TX, |tx| async move {
		let tx = tx.with_subspace(sim_read_scan_subspace());
		let begin = tx.pack(&ReadScanKey {
			shard,
			index: start,
		});
		let end = tx.pack(&ReadScanKey { shard, index: end });
		let mut range: RangeOption<'static> = (begin..end).into();
		range.limit = Some(limit);

		let informal = tx.informal();
		let mut stream = informal.get_ranges_keyvalues(range, Snapshot);
		while let Some(entry) = stream.next().await {
			let entry = entry?;
			if unpack_keys {
				let _ = tx.unpack::<ReadScanKey>(entry.key())?;
			}
			hint::black_box(entry.value().len());
		}

		Ok(())
	})
	.await
}

fn spawn_udb_conflict(udb: Option<UdbPool>, rate: Rate, max_in_flight: usize, key_count: u64) {
	if max_in_flight == 0 || key_count == 0 {
		if rate.load() > 0.0 {
			tracing::warn!(
				max_in_flight,
				key_count,
				"UPS simulation UDB conflict load is enabled without enough configuration"
			);
		}
		return;
	}

	let Some(udb) = udb else {
		if rate.load() > 0.0 {
			tracing::warn!("UPS simulation UDB conflict load enabled without a UDB pool");
		}
		return;
	};

	tokio::spawn(async move {
		if let Err(err) = seed_udb_conflict(&udb, key_count).await {
			tracing::warn!(
				?err,
				key_count,
				"failed to seed UPS simulation UDB conflict keys"
			);
		}

		let mut pacer = Pacer::new();
		let semaphore = Arc::new(tokio::sync::Semaphore::new(max_in_flight));

		loop {
			let count = pacer.next_count(rate.load()).await;
			for _ in 0..count {
				let Ok(permit) = semaphore.clone().try_acquire_owned() else {
					continue;
				};
				let udb = udb.clone();
				tokio::spawn(async move {
					let _permit = permit;
					let index = CONFLICT_SEQ.fetch_add(1, Ordering::Relaxed) % key_count;
					if let Err(err) = run_udb_conflict(&udb, index).await {
						tracing::debug!(?err, "UPS simulation UDB conflict transaction failed");
					}
				});
			}
		}
	});
}

async fn seed_udb_conflict(udb: &UdbPool, key_count: u64) -> Result<()> {
	let mut start = 0;

	tracing::info!(key_count, "seeding UPS simulation UDB conflict keys");

	while start < key_count {
		let end = start
			.saturating_add(UDB_CONFLICT_SEED_BATCH_SIZE)
			.min(key_count);
		udb.txn(UDB_CONFLICT_SEED_TX, |tx| async move {
			let tx = tx.with_subspace(sim_conflict_subspace());
			for index in start..end {
				let key = tx.pack(&ConflictKey { index });
				tx.set(&key, &0u64.to_be_bytes());
			}

			Ok(())
		})
		.await?;
		start = end;
	}

	tracing::info!(key_count, "seeded UPS simulation UDB conflict keys");
	Ok(())
}

async fn run_udb_conflict(udb: &UdbPool, index: u64) -> Result<()> {
	udb.txn(UDB_CONFLICT_TX, |tx| async move {
		let tx = tx.with_subspace(sim_conflict_subspace());
		let key = tx.pack(&ConflictKey { index });
		let value = tx.get(&key, Serializable).await?;
		let next = value
			.as_ref()
			.and_then(|value| value.as_slice().try_into().ok().map(u64::from_be_bytes))
			.unwrap_or(0)
			.wrapping_add(1);
		tx.set(&key, &next.to_be_bytes());
		Ok(())
	})
	.await
}

fn spawn_route_churn(
	ups: PubSub,
	rate: Rate,
	ephemeral_hold: Duration,
	stopped_hold: Duration,
	max_in_flight: usize,
	workload: Workload,
) {
	if max_in_flight == 0 {
		return;
	}

	tokio::spawn(async move {
		let mut pacer = Pacer::new();
		let semaphore = Arc::new(tokio::sync::Semaphore::new(max_in_flight));

		loop {
			let count = pacer.next_count(rate.load()).await;
			for _ in 0..count {
				let Ok(permit) = semaphore.clone().try_acquire_owned() else {
					continue;
				};
				let ups = ups.clone();
				let workload = workload.clone();
				tokio::spawn(async move {
					let _permit = permit;
					let route_id = SUBJECT_SEQ.fetch_add(1, Ordering::Relaxed);
					let mut ephemeral = Vec::new();
					for (root, prefix) in ROUTE_SUBJECTS {
						let subject =
							SimSubject::new(format!("{prefix}:actor_id:{route_id}"), *root);
						match ups.subscribe(subject).await {
							Ok(sub) => ephemeral.push(sub),
							Err(err) => tracing::debug!(
								?err,
								"failed to create UPS simulation route subscription"
							),
						}
					}

					let stopped = ups
						.subscribe(SimSubject::new(
							format!("gasoline.msg.pegboard_actor2_stopped:actor_id:{route_id}"),
							"gasoline.msg.pegboard_actor2_stopped",
						))
						.await
						.ok();

					workload.run().await;
					tokio::time::sleep(ephemeral_hold).await;
					drop(ephemeral);
					tokio::time::sleep(stopped_hold).await;
					drop(stopped);
				});
			}
		}
	});
}

fn spawn_subscription_churn(
	ups: PubSub,
	label: &'static str,
	root: &'static str,
	prefix: &'static str,
	rate: Rate,
	hold: Duration,
	active_subjects: Option<ActiveSubjects>,
	workload: Workload,
) {
	tokio::spawn(async move {
		let mut pacer = Pacer::new();

		loop {
			let count = pacer.next_count(rate.load()).await;
			for _ in 0..count {
				let ups = ups.clone();
				let active_subjects = active_subjects.clone();
				let workload = workload.clone();
				tokio::spawn(async move {
					let subject = unique_subject(root, prefix);
					match ups.subscribe(subject.clone()).await {
						Ok(mut sub) => {
							if let Some(active_subjects) = active_subjects.as_ref() {
								active_subjects.insert(subject.clone()).await;
							}

							let deadline = tokio::time::Instant::now() + hold;
							loop {
								tokio::select! {
									res = sub.next() => {
										match res {
											Ok(NextOutput::Message(_)) => workload.run().await,
											Ok(NextOutput::Unsubscribed | NextOutput::NoResponders) => break,
											Err(err) => {
												tracing::debug!(
													?err,
													%subject,
													label,
													"UPS simulation churn subscriber failed"
												);
												break;
											}
										}
									}
									_ = tokio::time::sleep_until(deadline) => break,
								}
							}

							if let Some(active_subjects) = active_subjects.as_ref() {
								active_subjects.remove(&subject).await;
							}
							drop(sub);
						}
						Err(err) => {
							tracing::debug!(
								?err,
								%subject,
								label,
								"failed to create UPS simulation churn subscription"
							);
						}
					}
				});
			}
		}
	});
}

const ROUTE_SUBJECTS: &[(&str, &str)] = &[
	(
		"gasoline.msg.pegboard_actor_failed",
		"gasoline.msg.pegboard_actor_failed",
	),
	(
		"gasoline.msg.pegboard_actor_ready",
		"gasoline.msg.pegboard_actor_ready",
	),
	(
		"gasoline.msg.pegboard_actor_stopped",
		"gasoline.msg.pegboard_actor_stopped",
	),
	(
		"gasoline.msg.pegboard_actor_destroy_started",
		"gasoline.msg.pegboard_actor_destroy_started",
	),
	(
		"gasoline.msg.pegboard_actor_migrated_to_v2",
		"gasoline.msg.pegboard_actor_migrated_to_v2",
	),
	(
		"gasoline.msg.pegboard_actor2_ready",
		"gasoline.msg.pegboard_actor2_ready",
	),
	(
		"gasoline.msg.pegboard_actor2_stopped",
		"gasoline.msg.pegboard_actor2_stopped",
	),
	(
		"gasoline.msg.pegboard_actor2_failed",
		"gasoline.msg.pegboard_actor2_failed",
	),
	(
		"gasoline.msg.pegboard_actor2_destroy_started",
		"gasoline.msg.pegboard_actor2_destroy_started",
	),
];

struct Pacer {
	interval: tokio::time::Interval,
	carry: f64,
	last: Instant,
}

impl Pacer {
	fn new() -> Self {
		let mut interval = tokio::time::interval(TICK);
		interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
		Self {
			interval,
			carry: 0.0,
			last: Instant::now(),
		}
	}

	async fn next_count(&mut self, rate_per_sec: f64) -> usize {
		self.interval.tick().await;
		let now = Instant::now();
		let elapsed = now.duration_since(self.last);
		self.last = now;
		self.carry += rate_per_sec * elapsed.as_secs_f64();
		let count = self.carry.floor() as usize;
		self.carry -= count as f64;
		count
	}
}

#[derive(Clone)]
struct SimSubject {
	subject: String,
	root: String,
}

impl SimSubject {
	fn new(subject: impl Into<String>, root: impl Into<String>) -> Self {
		Self {
			subject: subject.into(),
			root: root.into(),
		}
	}
}

impl fmt::Display for SimSubject {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.subject.fmt(f)
	}
}

impl Subject for SimSubject {
	fn subject_root<'a>(&'a self) -> Option<Cow<'a, str>> {
		Some(Cow::Borrowed(self.root.as_str()))
	}

	fn as_str(&self) -> Option<&str> {
		Some(self.subject.as_str())
	}
}

#[derive(Clone)]
struct RawSubject {
	subject: String,
}

impl RawSubject {
	fn new(subject: impl Into<String>) -> Self {
		Self {
			subject: subject.into(),
		}
	}
}

impl fmt::Display for RawSubject {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.subject.fmt(f)
	}
}

impl Subject for RawSubject {
	fn as_str(&self) -> Option<&str> {
		Some(self.subject.as_str())
	}
}

fn subjects(root: &'static str, prefix: &'static str, count: usize) -> Vec<SimSubject> {
	(0..count)
		.map(|idx| SimSubject::new(format!("{prefix}.{idx}"), root))
		.collect()
}

fn raw_subjects(prefix: &'static str, count: usize) -> Vec<RawSubject> {
	(0..count)
		.map(|idx| RawSubject::new(format!("{prefix}.{idx}")))
		.collect()
}

fn unique_subject(root: &'static str, prefix: &'static str) -> SimSubject {
	let idx = SUBJECT_SEQ.fetch_add(1, Ordering::Relaxed);
	SimSubject::new(format!("{prefix}.{idx}"), root)
}

fn payload(size: usize) -> Vec<u8> {
	vec![b'x'; size]
}

fn env_key(key: &str) -> String {
	format!("{ENV_PREFIX}_{key}")
}

fn env_string(key: &str) -> Option<String> {
	env::var(env_key(key)).ok()
}

fn env_bool(key: &str, default: bool) -> Result<bool> {
	let Some(value) = env_string(key) else {
		return Ok(default);
	};
	match value.to_ascii_lowercase().as_str() {
		"1" | "true" | "yes" | "on" => Ok(true),
		"0" | "false" | "no" | "off" => Ok(false),
		_ => bail!("{ENV_PREFIX}_{key} must be a boolean"),
	}
}

fn env_usize(key: &str, default: usize) -> Result<usize> {
	parse_env(key, default)
}

fn env_u64(key: &str, default: u64) -> Result<u64> {
	parse_env(key, default)
}

fn env_f64(key: &str, default: f64) -> Result<f64> {
	parse_env(key, default)
}

fn env_id(key: &str, default: Id) -> Result<Id> {
	let Some(value) = env_string(key) else {
		return Ok(default);
	};
	Id::parse(&value).with_context(|| format!("failed to parse {ENV_PREFIX}_{key}"))
}

fn parse_env<T>(key: &str, default: T) -> Result<T>
where
	T: std::str::FromStr,
	T::Err: std::error::Error + Send + Sync + 'static,
{
	let Some(value) = env_string(key) else {
		return Ok(default);
	};
	value
		.parse()
		.with_context(|| format!("failed to parse {ENV_PREFIX}_{key}"))
}

fn validate_rate(key: &str, rate: f64) -> Result<()> {
	if rate.is_finite() && rate >= 0.0 {
		Ok(())
	} else {
		bail!("{ENV_PREFIX}_{key} must be a finite non-negative number")
	}
}

fn now_ms() -> u64 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map(|duration| duration.as_millis() as u64)
		.unwrap_or(0)
}

fn duration_millis_u64(duration: Duration) -> u64 {
	u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}
