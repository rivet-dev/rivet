//! Short first attempts for small object-store requests, applied inside the production
//! HTTP connector that `InstrumentedHttpConnector` installs under `object_store`'s retry
//! loop.
//!
//! A request sent on a stalled connection otherwise waits for the client's 30 s timeout
//! before `object_store` retries it, while a fresh attempt usually answers in milliseconds.
//! So the first attempt of a small GET or HEAD, and of a small conditional PUT when its
//! deadline is configured, gets a short deadline for its response headers. When the deadline passes, the attempt is cut off and returned to
//! `object_store` as a retryable transport error; its own retry loop then sends the next
//! attempt with the normal client timeouts. No retry is added here.
//!
//! The deadline covers only the time until response headers arrive, which for a PUT
//! includes sending its body, so a PUT's deadline grows with its size. Response bodies,
//! multipart parts, overwriting uploads and listings keep the client's normal timeouts.
//!
//! Each cut-off is one extra billed request, so a per-client budget allows at most one
//! cut-off per ten eligible requests, plus a small burst. When the budget is spent, an
//! attempt past its deadline keeps waiting instead of being cut off, which is what the
//! client did before this existed.
//!
//! A conditional PUT that was cut off may still have been applied. Its retry carries the
//! same precondition, so at most one of the two attempts applies, but a retry that then
//! fails its precondition cannot tell whether the earlier attempt applied it. That outcome
//! is reported as an unknown error instead of a precondition failure, so the caller reads
//! the object back as it does for any other unknown write outcome.

use object_store::client::{HttpError, HttpErrorKind, HttpRequest};
use std::{
	future::Future,
	sync::atomic::{AtomicU64, Ordering},
	time::Duration,
};

/// The first-attempt deadlines of one client, from the object-storage configuration
/// (`read_first_attempt_ms`, `write_first_attempt_ms` and
/// `write_first_attempt_bytes_per_second`, which document their defaults).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FirstAttemptDeadlines {
	/// Response-header deadline of a small GET or HEAD, or `None` when disabled.
	pub read: Option<Duration>,
	/// Response-header deadline of a small conditional PUT before its upload time, or
	/// `None` when disabled.
	pub write: Option<Duration>,
	/// The upload rate a conditional PUT's first attempt may fall to before it is cut off.
	pub write_bytes_per_second: u64,
}

impl FirstAttemptDeadlines {
	/// Classifies one HTTP request and returns its first-attempt deadline, or `None` when
	/// it keeps the client's normal timeouts. `billed_method` is the request's billed
	/// class, so listings, which are GETs, are excluded.
	pub(crate) fn deadline(
		&self,
		request: &HttpRequest,
		billed_method: &str,
	) -> Option<(Eligible, Duration)> {
		match billed_method {
			"get" | "head" => self.read.map(|deadline| (Eligible::Read, deadline)),
			"put" if is_conditional_put(request) => {
				let base = self.write?;
				let bytes = request.body().content_length();
				if bytes > MAX_CONDITIONAL_PUT_BYTES || self.write_bytes_per_second == 0 {
					return None;
				}
				let upload =
					Duration::from_secs_f64(bytes as f64 / self.write_bytes_per_second as f64);
				Some((Eligible::ConditionalPut, base + upload))
			}
			_ => None,
		}
	}
}

/// The largest conditional PUT body that gets a short first attempt. Leases, heartbeats and
/// most manifests are far smaller; a larger body would be resent whole after a cut-off.
pub(crate) const MAX_CONDITIONAL_PUT_BYTES: usize = 1024 * 1024;

/// The budget in thousandths of a cut-off: each eligible first attempt earns
/// `BUDGET_EARNED_PER_ATTEMPT`, so cut-offs stay under one in ten eligible requests, and
/// at most `BUDGET_CAP` is saved for a burst.
const BUDGET_COST_PER_CUT_OFF: u64 = 1_000;
const BUDGET_EARNED_PER_ATTEMPT: u64 = 100;
const BUDGET_CAP: u64 = 20 * BUDGET_COST_PER_CUT_OFF;

/// What kind of small request a first attempt is, which decides how a cut-off is
/// reported to `object_store`'s retry loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Eligible {
	/// A GET or HEAD, which the retry loop retries after a timeout.
	Read,
	/// A conditional PUT. The retry loop retries a timeout only for idempotent requests, so
	/// a cut-off is reported as a request error, which it always retries, and its retry's
	/// precondition failure is reported as unknown.
	ConditionalPut,
}

/// The first-attempt deadlines and cut-off budget of one HTTP client.
#[derive(Debug)]
pub(crate) struct ShortFirstAttempts {
	pub(crate) deadlines: FirstAttemptDeadlines,
	budget: AtomicU64,
}

impl ShortFirstAttempts {
	pub(crate) fn new(deadlines: FirstAttemptDeadlines) -> Self {
		Self {
			deadlines,
			budget: AtomicU64::new(BUDGET_CAP),
		}
	}
}

/// How a first attempt under a short deadline ended.
pub(crate) enum FirstAttempt<T> {
	/// The attempt answered, before its deadline or after it with the budget spent.
	Answered(T),
	/// The attempt was cut off at its deadline and must be retried.
	CutOff,
}

impl ShortFirstAttempts {
	/// Runs one first attempt under `deadline`. Past the deadline, the attempt is cut off if
	/// the budget allows one more cut-off, and otherwise it is awaited to completion.
	pub(crate) async fn run<F: Future>(
		&self,
		method: &'static str,
		deadline: Duration,
		attempt: F,
	) -> FirstAttempt<F::Output> {
		self.earn();
		let mut attempt = std::pin::pin!(attempt);

		#[allow(
			clippy::disallowed_methods,
			reason = "the production object-store HTTP transport adapter times its attempts on tokio time, as the upstream retry loop it runs under does; simulations never install it"
		)]
		let early = tokio::time::timeout(deadline, attempt.as_mut()).await;
		match early {
			Ok(output) => {
				record(method, "answered");
				FirstAttempt::Answered(output)
			}
			Err(_) if self.spend() => {
				record(method, "cut_off");
				FirstAttempt::CutOff
			}
			Err(_) => {
				record(method, "waited");
				FirstAttempt::Answered(attempt.await)
			}
		}
	}

	fn earn(&self) {
		let _ = self
			.budget
			.try_update(Ordering::Relaxed, Ordering::Relaxed, |budget| {
				Some(
					budget
						.saturating_add(BUDGET_EARNED_PER_ATTEMPT)
						.min(BUDGET_CAP),
				)
			});
	}

	/// Takes one cut-off from the budget, or returns false if it holds less than one.
	fn spend(&self) -> bool {
		self.budget
			.try_update(Ordering::Relaxed, Ordering::Relaxed, |budget| {
				budget.checked_sub(BUDGET_COST_PER_CUT_OFF)
			})
			.is_ok()
	}
}

fn record(method: &str, result: &str) {
	crate::metrics::OBJECT_STORE_FIRST_ATTEMPTS
		.with_label_values(&[method, result])
		.inc();
}

/// Whether `request` is a single-request conditional PUT: S3 and Azure send their
/// precondition as `If-Match` or `If-None-Match`, and GCS as `x-goog-if-generation-match`.
/// Azure block operations (`comp=` in the query) are parts of a multipart upload and keep
/// their normal timeouts.
pub(crate) fn is_conditional_put(request: &HttpRequest) -> bool {
	if request.method() != http::Method::PUT {
		return false;
	}
	let block_operation = request
		.uri()
		.query()
		.is_some_and(|query| query.split('&').any(|pair| pair.starts_with("comp=")));
	if block_operation {
		return false;
	}
	let headers = request.headers();
	headers.contains_key(http::header::IF_MATCH)
		|| headers.contains_key(http::header::IF_NONE_MATCH)
		|| headers.contains_key("x-goog-if-generation-match")
}

/// The error a cut-off attempt returns to `object_store`'s retry loop, of a kind it retries
/// for `eligible`.
pub(crate) fn cut_off_error(eligible: Eligible, deadline: Duration) -> HttpError {
	let kind = match eligible {
		Eligible::Read => HttpErrorKind::Timeout,
		// The attempt may have reached the store. Retrying it is still safe because the retry
		// carries the same precondition, and `ambiguous_precondition_error` reports a retry
		// whose precondition then fails as unknown.
		Eligible::ConditionalPut => HttpErrorKind::Request,
	};
	HttpError::new(kind, FirstAttemptCutOff { deadline })
}

/// Whether a later attempt's response `status` is a precondition failure that an earlier,
/// cut-off attempt of the same conditional PUT may have caused.
pub(crate) fn is_ambiguous_precondition(status: u16) -> bool {
	matches!(status, 409 | 412)
}

/// The error for a conditional PUT whose earlier attempt was cut off and whose retry then
/// failed its precondition. `object_store` returns it as a generic, non-retryable error,
/// which callers treat as an unknown outcome and read back.
pub(crate) fn ambiguous_precondition_error(status: u16) -> HttpError {
	HttpError::new(HttpErrorKind::Unknown, AmbiguousPrecondition { status })
}

#[derive(Debug, thiserror::Error)]
#[error("first attempt cut off after {deadline:?} without response headers")]
struct FirstAttemptCutOff {
	deadline: Duration,
}

#[derive(Debug, thiserror::Error)]
#[error(
	"conditional put failed its precondition ({status}) after an earlier attempt was cut off, so that attempt may have applied it; read the object back"
)]
struct AmbiguousPrecondition {
	status: u16,
}


