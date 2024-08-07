use std::time::{SystemTime, UNIX_EPOCH};

use global_error::{macros::*, GlobalError, GlobalResult};
use rand::Rng;
use tokio::time::{self, Duration};
use uuid::Uuid;

use crate::error::WorkflowError;

pub type Location = Box<[usize]>;

// How often the `inject_fault` function fails in percent
const FAULT_RATE: usize = 80;

/// Allows for checking if a global error returned from an activity is recoverable.
pub trait GlobalErrorExt {
	fn is_workflow_recoverable(&self) -> bool;
}

impl GlobalErrorExt for GlobalError {
	fn is_workflow_recoverable(&self) -> bool {
		match self {
			GlobalError::Raw(inner_err) => inner_err
				.downcast_ref::<WorkflowError>()
				.map(|err| err.is_recoverable())
				.unwrap_or_default(),
			_ => false,
		}
	}
}

impl<T> GlobalErrorExt for GlobalResult<T> {
	fn is_workflow_recoverable(&self) -> bool {
		match self {
			Err(GlobalError::Raw(inner_err)) => inner_err
				.downcast_ref::<WorkflowError>()
				.map(|err| err.is_recoverable())
				.unwrap_or_default(),
			_ => false,
		}
	}
}

pub async fn sleep_until_ts(ts: i64) {
	let target_time = UNIX_EPOCH + Duration::from_millis(ts as u64);
	if let Ok(sleep_duration) = target_time.duration_since(SystemTime::now()) {
		time::sleep(sleep_duration).await;
	}
}

pub fn inject_fault() -> GlobalResult<()> {
	if rand::thread_rng().gen_range(0..100) < FAULT_RATE {
		bail!("This is a random panic!");
	}

	Ok(())
}

pub(crate) fn new_conn(
	shared_client: &chirp_client::SharedClientHandle,
	pools: &rivet_pools::Pools,
	cache: &rivet_cache::Cache,
	ray_id: Uuid,
	req_id: Uuid,
	name: &str,
) -> rivet_connection::Connection {
	let client = shared_client.clone().wrap(
		req_id,
		ray_id,
		vec![chirp_client::TraceEntry {
			context_name: name.into(),
			req_id: Some(req_id.into()),
			ts: rivet_util::timestamp::now(),
			run_context: match rivet_util::env::run_context() {
				rivet_util::env::RunContext::Service => chirp_client::RunContext::Service,
				rivet_util::env::RunContext::Test => chirp_client::RunContext::Test,
			} as i32,
		}],
	);

	rivet_connection::Connection::new(client, pools.clone(), cache.clone())
}

pub fn format_location(loc: &Location) -> String {
	let mut s = "{".to_string();

	let mut iter = loc.iter();

	if let Some(x) = iter.next() {
		s.push_str(&x.to_string());
	}

	for x in iter {
		s.push_str(", ");
		s.push_str(&x.to_string());
	}

	s.push_str("}");

	s
}
