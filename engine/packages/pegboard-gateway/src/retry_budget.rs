//! The gateway is the only retry owner for a refused request. Storage and UPS refuse work they
//! cannot admit and return at once as `overloaded`; the gateway sends the request again after the refusal's hint,
//! spending one token from a process-wide budget that every received request refills by a
//! fraction of a token. When storage sheds load for longer than the budget covers, refusals reach
//! the client as `guard.service_unavailable` with the hint, and the client owns the next attempt.
//!
//! A request whose destination worker could not be resolved, as after that worker died, was
//! never sent either. Its retry routes from the start with a fresh topology lookup, which may
//! name another worker or the tablet's new maintainer. Recovery from a dead worker takes longer
//! than a burst of shed load, so those retries back off exponentially until the request deadline
//! or the budget runs out instead of stopping after a fixed number of attempts.
//! Snapshot-only storage reads interrupted by transport can also retry here: routing has not
//! sent the user request to the actor, and repeating the lookup has no mutating effects.
use anyhow::Result;
use axum::{body::Body, response::Response};
use hyper::header::{self, HeaderValue};
use rivet_engine_context::EngineContext;
use rivet_env::Env;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

/// Retry hint for a refused UPS stream open, which carries none of its own.
const UPS_OVERLOADED_RETRY_AFTER_MS: u64 = 100;

/// First backoff step; each later retry of the same request doubles it.
const BACKOFF_INITIAL_MS: u64 = 25;

/// Longest backoff step between retries of one request.
const BACKOFF_MAX_MS: u64 = 1_000;

/// Retries one request may spend on overload refusals before the refusal goes to the client.
const MAX_RETRIES_PER_REQUEST: u32 = 4;

/// Budget arithmetic runs in thousandths of a token so a 10% deposit is exact.
const MILLI: u64 = 1_000;

/// A routing failure the edge may retry before the user request reaches the actor: either
/// storage/UPS refused the work or a snapshot-only lookup lost its reply. An uncertain mutation,
/// explicit storage failure, fence, or corruption is final for this routing attempt.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Refusal {
	/// Bounded cause, used as a metric label.
	pub reason: &'static str,
	pub retry_after_ms: u64,
	pub kind: RefusalKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefusalKind {
	/// Storage or UPS shed the work. A retry waits at least the hint, within a per-request cap.
	Overloaded,
	/// A snapshot-only lookup lost its reply. Uses the same bounded attempt policy as overload.
	ReadInterrupted,
	/// Topology could not resolve the destination worker, so nothing was sent. A retry routes
	/// again from a fresh lookup after a jittered exponential backoff, bounded by the deadline
	/// and the budget. The hint is only for the client once those run out.
	RouteUnresolved,
}

/// Classifies `error` as a retryable refusal.
/// Only definite refusals and transport-interrupted snapshot reads qualify. An uncertain
/// mutation never does.
pub(crate) fn refusal(error: &anyhow::Error) -> Option<Refusal> {
	if let Some(retry_after_ms) = depot_client::retry_after_ms(error) {
		if error
			.downcast_ref::<rivet_error::RivetError>()
			.is_some_and(|error| error.group() == "depot" && error.code() == "read_interrupted")
		{
			return Some(Refusal {
				reason: "depot_read_interrupted",
				retry_after_ms,
				kind: RefusalKind::ReadInterrupted,
			});
		}
		// A storage command whose maintainer's endpoint could not be resolved was not sent.
		if depot_client::route_unresolved(error) {
			return Some(Refusal {
				reason: "depot_endpoint_unresolved",
				retry_after_ms,
				kind: RefusalKind::RouteUnresolved,
			});
		}
		return Some(Refusal {
			reason: "depot_overloaded",
			retry_after_ms,
			kind: RefusalKind::Overloaded,
		});
	}
	let structured = error.downcast_ref::<rivet_error::RivetError>()?;
	if structured.group() == "ups" && structured.code() == "overloaded" {
		return Some(Refusal {
			reason: "ups_overloaded",
			retry_after_ms: UPS_OVERLOADED_RETRY_AFTER_MS,
			kind: RefusalKind::Overloaded,
		});
	}
	// Nothing was sent: the worker's route is being replaced, as after a worker died.
	if structured.group() == "ups" && structured.code() == "endpoint_unresolved" {
		return Some(Refusal {
			reason: "ups_endpoint_unresolved",
			retry_after_ms: UPS_OVERLOADED_RETRY_AFTER_MS,
			kind: RefusalKind::RouteUnresolved,
		});
	}
	None
}

/// A refusal the gateway did not retry, as the client sees it.
#[derive(Debug, rivet_error::RivetError, serde::Serialize)]
#[error(
	"guard",
	"service_unavailable",
	"Service unavailable.",
	"Service unavailable ({reason}). Retry after {retry_after_ms} ms."
)]
struct ServiceUnavailable {
	reason: &'static str,
	retry_after_ms: u64,
}

/// The client-facing error for a refusal the gateway did not retry.
pub(crate) fn refusal_error(refusal: Refusal) -> anyhow::Error {
	ServiceUnavailable {
		reason: refusal.reason,
		retry_after_ms: refusal.retry_after_ms,
	}
	.build()
}

/// The client-facing HTTP reply for a refusal the gateway did not retry: the canonical error
/// with a `Retry-After` hint.
pub(crate) fn refusal_response(refusal: Refusal) -> Response {
	let retry_after_secs = refusal.retry_after_ms.div_ceil(1_000).max(1);
	let mut response =
		rivet_guard_core::errors::http_response(&refusal_error(refusal)).map(Body::from);
	response
		.headers_mut()
		.insert(header::RETRY_AFTER, HeaderValue::from(retry_after_secs));
	response
}

pub struct RetryBudget {
	milli_tokens: AtomicU64,
	max_milli_tokens: u64,
	deposit_milli_tokens: u64,
	/// Set while the budget is spent, so the warning fires once per exhaustion.
	exhausted: AtomicBool,
}

impl RetryBudget {
	pub fn from_config(ctx: &EngineContext) -> Self {
		let config = ctx.config().pegboard();
		Self::new(
			config.gateway_retry_budget_percent(),
			config.gateway_retry_budget_max_tokens(),
		)
	}

	/// A budget that starts full, refilled by `percent` hundredths of a token per request and
	/// holding at most `max_tokens`.
	pub fn new(percent: u32, max_tokens: u32) -> Self {
		let max_milli_tokens = u64::from(max_tokens) * MILLI;
		Self {
			milli_tokens: AtomicU64::new(max_milli_tokens),
			max_milli_tokens,
			deposit_milli_tokens: u64::from(percent) * MILLI / 100,
			exhausted: AtomicBool::new(false),
		}
	}

	/// Credits one received request.
	pub fn deposit(&self) {
		// The update never rejects a value, so it cannot fail.
		let _ = self
			.milli_tokens
			.try_update(Ordering::AcqRel, Ordering::Acquire, |tokens| {
				Some(
					tokens
						.saturating_add(self.deposit_milli_tokens)
						.min(self.max_milli_tokens),
				)
			});
	}

	/// Spends one token. Returns `false` when the budget is spent.
	fn try_withdraw(&self) -> bool {
		let withdrawn =
			self.milli_tokens
				.try_update(Ordering::AcqRel, Ordering::Acquire, |tokens| {
					tokens.checked_sub(MILLI)
				});
		match withdrawn {
			Ok(_) => {
				self.exhausted.store(false, Ordering::Release);
				true
			}
			Err(_) => {
				if !self.exhausted.swap(true, Ordering::AcqRel) {
					tracing::warn!(
						"gateway retry budget is spent; refused requests now fail to the client. storage is shedding load, so add capacity if this persists"
					);
				}
				false
			}
		}
	}

	/// Starts the retries of one request that must finish by `deadline_boot_ms`.
	pub fn retries(&self, deadline_boot_ms: u64) -> Retries<'_> {
		Retries {
			budget: self,
			deadline_boot_ms,
			retries: 0,
		}
	}

	/// Waits out `refusal` for the request's `attempt`th retry, when the per-request cap, the
	/// budget and the request deadline allow it. Returns `false` when the refusal should go to
	/// the client instead.
	async fn wait_to_retry(
		&self,
		env: &dyn Env,
		refusal: Refusal,
		attempt: u32,
		deadline_boot_ms: u64,
	) -> Result<bool> {
		let denied = |cause: &'static str| {
			tracing::debug!(
				reason = refusal.reason,
				cause,
				"refused gateway request goes to the client"
			);
			Ok(false)
		};
		let floor_ms = match refusal.kind {
			RefusalKind::Overloaded | RefusalKind::ReadInterrupted => {
				if attempt >= MAX_RETRIES_PER_REQUEST {
					return denied("attempts");
				}
				refusal.retry_after_ms
			}
			RefusalKind::RouteUnresolved => 0,
		};

		// Route retries are uncapped, so the shift stops growing once it passes the maximum.
		let backoff_ms = (BACKOFF_INITIAL_MS << attempt.min(16)).min(BACKOFF_MAX_MS);
		let jittered_ms = env
			.rng("gateway.retry")
			.gen_range(backoff_ms / 2..backoff_ms + 1);
		let wait_ms = jittered_ms.max(floor_ms);
		let clock = env.clock();
		let now_boot_ms = clock.observe()?.boot_ms;
		if now_boot_ms.saturating_add(wait_ms) >= deadline_boot_ms {
			return denied("deadline");
		}
		if !self.try_withdraw() {
			return denied("budget");
		}

		rivet_env::sleep(clock.as_ref(), Duration::from_millis(wait_ms)).await?;
		Ok(true)
	}
}

/// The retries of one request. Each attempt starts from the beginning, so it resolves its route
/// afresh.
pub struct Retries<'a> {
	budget: &'a RetryBudget,
	deadline_boot_ms: u64,
	retries: u32,
}

impl Retries<'_> {
	/// Decides whether the attempt that failed with `error` runs again. Returns `Ok` after
	/// waiting out the backoff when it may, and otherwise the error for the client: anything
	/// that is not a refusal at once, and a refusal once the deadline or budget runs out.
	pub async fn retry(&mut self, env: &dyn Env, error: anyhow::Error) -> Result<()> {
		let Some(refused) = refusal(&error) else {
			return Err(error);
		};
		if !self
			.budget
			.wait_to_retry(env, refused, self.retries, self.deadline_boot_ms)
			.await?
		{
			return Err(error);
		}
		self.retries += 1;
		Ok(())
	}
}


