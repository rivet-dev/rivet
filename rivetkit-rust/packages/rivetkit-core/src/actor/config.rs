use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use rivet_envoy_client::config::HttpRequest;

const DEFAULT_STATE_SAVE_INTERVAL: Duration = Duration::from_secs(1);
const DEFAULT_CREATE_VARS_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_CREATE_CONN_STATE_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_ON_BEFORE_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_ON_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_ON_SLEEP_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_ON_DESTROY_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_ACTION_TIMEOUT: Duration = Duration::from_secs(60);
const DEFAULT_RUN_STOP_TIMEOUT: Duration = Duration::from_secs(15);
const DEFAULT_SLEEP_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_SLEEP_GRACE_PERIOD: Duration = Duration::from_secs(15);
const DEFAULT_CONNECTION_LIVENESS_TIMEOUT: Duration = Duration::from_millis(2500);
const DEFAULT_CONNECTION_LIVENESS_INTERVAL: Duration = Duration::from_secs(5);
const DEFAULT_MAX_QUEUE_SIZE: u32 = 1000;
const DEFAULT_MAX_QUEUE_MESSAGE_SIZE: u32 = 65_536;

#[derive(Clone)]
pub enum CanHibernateWebSocket {
	Bool(bool),
	Callback(Arc<dyn Fn(&HttpRequest) -> bool + Send + Sync>),
}

impl fmt::Debug for CanHibernateWebSocket {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Bool(value) => f.debug_tuple("Bool").field(value).finish(),
			Self::Callback(_) => f.write_str("Callback(..)"),
		}
	}
}

impl Default for CanHibernateWebSocket {
	fn default() -> Self {
		Self::Bool(false)
	}
}

#[derive(Clone, Debug, Default)]
pub struct ActorConfigOverrides {
	pub sleep_grace_period: Option<Duration>,
	pub on_sleep_timeout: Option<Duration>,
	pub on_destroy_timeout: Option<Duration>,
	pub run_stop_timeout: Option<Duration>,
}

#[derive(Clone, Debug)]
pub struct ActorConfig {
	pub name: Option<String>,
	pub icon: Option<String>,
	pub can_hibernate_websocket: CanHibernateWebSocket,
	pub state_save_interval: Duration,
	pub create_vars_timeout: Duration,
	pub create_conn_state_timeout: Duration,
	pub on_before_connect_timeout: Duration,
	pub on_connect_timeout: Duration,
	pub on_sleep_timeout: Duration,
	pub on_destroy_timeout: Duration,
	pub action_timeout: Duration,
	pub run_stop_timeout: Duration,
	pub sleep_timeout: Duration,
	pub no_sleep: bool,
	pub sleep_grace_period: Option<Duration>,
	pub connection_liveness_timeout: Duration,
	pub connection_liveness_interval: Duration,
	pub max_queue_size: u32,
	pub max_queue_message_size: u32,
	pub preload_max_workflow_bytes: Option<u64>,
	pub preload_max_connections_bytes: Option<u64>,
	pub overrides: Option<ActorConfigOverrides>,
}

impl ActorConfig {
	pub fn effective_on_sleep_timeout(&self) -> Duration {
		cap_duration(
			self.on_sleep_timeout,
			self.overrides
				.as_ref()
				.and_then(|overrides| overrides.on_sleep_timeout),
		)
	}

	pub fn effective_on_destroy_timeout(&self) -> Duration {
		cap_duration(
			self.on_destroy_timeout,
			self.overrides
				.as_ref()
				.and_then(|overrides| overrides.on_destroy_timeout),
		)
	}

	pub fn effective_run_stop_timeout(&self) -> Duration {
		cap_duration(
			self.run_stop_timeout,
			self.overrides
				.as_ref()
				.and_then(|overrides| overrides.run_stop_timeout),
		)
	}

	pub fn effective_sleep_grace_period(&self) -> Duration {
		let configured = if let Some(sleep_grace_period) = self.sleep_grace_period {
			sleep_grace_period
		} else if self.on_sleep_timeout != DEFAULT_ON_SLEEP_TIMEOUT {
			self.effective_on_sleep_timeout() + DEFAULT_SLEEP_GRACE_PERIOD
		} else {
			DEFAULT_SLEEP_GRACE_PERIOD
		};

		cap_duration(
			configured,
			self.overrides
				.as_ref()
				.and_then(|overrides| overrides.sleep_grace_period),
		)
	}
}

impl Default for ActorConfig {
	fn default() -> Self {
		Self {
			name: None,
			icon: None,
			can_hibernate_websocket: CanHibernateWebSocket::default(),
			state_save_interval: DEFAULT_STATE_SAVE_INTERVAL,
			create_vars_timeout: DEFAULT_CREATE_VARS_TIMEOUT,
			create_conn_state_timeout: DEFAULT_CREATE_CONN_STATE_TIMEOUT,
			on_before_connect_timeout: DEFAULT_ON_BEFORE_CONNECT_TIMEOUT,
			on_connect_timeout: DEFAULT_ON_CONNECT_TIMEOUT,
			on_sleep_timeout: DEFAULT_ON_SLEEP_TIMEOUT,
			on_destroy_timeout: DEFAULT_ON_DESTROY_TIMEOUT,
			action_timeout: DEFAULT_ACTION_TIMEOUT,
			run_stop_timeout: DEFAULT_RUN_STOP_TIMEOUT,
			sleep_timeout: DEFAULT_SLEEP_TIMEOUT,
			no_sleep: false,
			sleep_grace_period: None,
			connection_liveness_timeout: DEFAULT_CONNECTION_LIVENESS_TIMEOUT,
			connection_liveness_interval: DEFAULT_CONNECTION_LIVENESS_INTERVAL,
			max_queue_size: DEFAULT_MAX_QUEUE_SIZE,
			max_queue_message_size: DEFAULT_MAX_QUEUE_MESSAGE_SIZE,
			preload_max_workflow_bytes: None,
			preload_max_connections_bytes: None,
			overrides: None,
		}
	}
}

fn cap_duration(duration: Duration, override_duration: Option<Duration>) -> Duration {
	if let Some(override_duration) = override_duration {
		duration.min(override_duration)
	} else {
		duration
	}
}
