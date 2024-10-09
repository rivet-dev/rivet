use crate::{
	ctx::WorkflowCtx,
	db::SignalData,
	error::{WorkflowError, WorkflowResult},
	history::location::Location,
};

/// Indirection struct to prevent invalid implementations of listen traits.
pub struct ListenCtx<'a> {
	ctx: &'a mut WorkflowCtx,
	location: &'a Location,
	// HACK: Prevent `ListenCtx::listen_any` from being called more than once
	used: bool,
}

impl<'a> ListenCtx<'a> {
	pub(crate) fn new(ctx: &'a mut WorkflowCtx, location: &'a Location) -> Self {
		ListenCtx {
			ctx,
			location,
			used: false,
		}
	}

	/// Checks for a signal to this workflow with any of the given signal names.
	/// - Will error if called more than once.
	pub async fn listen_any(
		&mut self,
		signal_names: &[&'static str],
	) -> WorkflowResult<SignalData> {
		if self.used {
			return Err(WorkflowError::ListenCtxUsed);
		} else {
			self.used = true;
		}

		// Fetch new pending signal
		let signal = self
			.ctx
			.db()
			.pull_next_signal(
				self.ctx.workflow_id(),
				signal_names,
				&self.location,
				self.ctx.version(),
				self.ctx.loop_location(),
			)
			.await?;

		let Some(signal) = signal else {
			return Err(WorkflowError::NoSignalFound(Box::from(signal_names)));
		};

		let recv_lag = (rivet_util::timestamp::now() as f64 - signal.create_ts as f64) / 1000.;
		crate::metrics::SIGNAL_RECV_LAG
			.with_label_values(&[&self.ctx.name(), &signal.signal_name])
			.observe(recv_lag);

		tracing::info!(
			workflow_name=%self.ctx.name(),
			workflow_id=%self.ctx.workflow_id(),
			signal_id=%signal.signal_id,
			signal_name=%signal.signal_name,
			"signal received",
		);

		Ok(signal)
	}
}
