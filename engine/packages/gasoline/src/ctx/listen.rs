use std::ops::Deref;

use crate::{
	ctx::WorkflowCtx,
	db::SignalData,
	error::{WorkflowError, WorkflowResult},
	history::location::Location,
	metrics,
};

/// Indirection struct to prevent invalid implementations of listen traits.
pub struct ListenCtx<'a> {
	ctx: &'a WorkflowCtx,
	location: &'a Location,
	// Used by certain db drivers to know when to update internal indexes for signal wake conditions
	last_attempt: bool,
	// HACK: Prevent `ListenCtx::listen_any` from being called more than once
	used: bool,
}

impl<'a> ListenCtx<'a> {
	pub(crate) fn new(ctx: &'a WorkflowCtx, location: &'a Location) -> Self {
		ListenCtx {
			ctx,
			location,
			last_attempt: false,
			used: false,
		}
	}

	pub(crate) fn reset(&mut self, last_attempt: bool) {
		self.used = false;
		self.last_attempt = last_attempt;
	}

	/// Checks for a signal to this workflow with any of the given signal names.
	/// - Will error if called more than once.
	#[tracing::instrument(skip_all, fields(?signal_names))]
	pub async fn listen_any(
		&mut self,
		signal_names: &[&'static str],
		limit: usize,
	) -> WorkflowResult<Vec<SignalData>> {
		if self.used {
			return Err(WorkflowError::ListenCtxUsed);
		} else {
			self.used = true;
		}

		// Fetch new pending signals
		let signals = self
			.ctx
			.db()
			.pull_next_signals(
				self.ctx.workflow_id(),
				self.ctx.name(),
				signal_names,
				self.location,
				self.ctx.version(),
				self.ctx.loop_location(),
				limit,
				self.last_attempt,
			)
			.await?;

		if signals.is_empty() {
			return Err(WorkflowError::NoSignalFound(Box::from(signal_names)));
		}

		let now = rivet_util::timestamp::now();
		for signal in &signals {
			let recv_lag = (now as f64 - signal.create_ts as f64) / 1000.0;
			metrics::SIGNAL_RECV_LAG
				.with_label_values(&[self.ctx.name(), signal.signal_name.as_str()])
				.observe(recv_lag);

			if recv_lag > 3.0 {
				// We print an error here so the trace of this workflow does not get dropped
				tracing::error!(
					?recv_lag,
					signal_id=%signal.signal_id,
					signal_name=%signal.signal_name,
					"long signal recv time",
				);
			}

			tracing::debug!(
				signal_id=%signal.signal_id,
				signal_name=%signal.signal_name,
				"signal received",
			);
		}

		Ok(signals)
	}
}

impl<'a> Deref for ListenCtx<'a> {
	type Target = rivet_pools::Pools;

	fn deref(&self) -> &Self::Target {
		self.ctx.pools()
	}
}
