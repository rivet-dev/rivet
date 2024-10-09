use global_error::{GlobalError, GlobalResult};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
	activity::{Activity, ActivityInput},
	builder::workflow as builder,
	ctx::{workflow::Loop, WorkflowCtx},
	executable::{AsyncResult, Executable},
	listen::{CustomListener, Listen},
	message::Message,
	signal::Signal,
	utils::time::{DurationToMillis, TsToMillis},
	workflow::{Workflow, WorkflowInput},
};

/// Used to set the version of the inner ctx, execute something, then set it back.
macro_rules! wrap {
	($self:expr, $step:expr, $code:tt) => {{
		// Error for version mismatch
		$self
			.inner
			.compare_version($step, $self.version)
			.map_err(GlobalError::raw)?;

		let old_version = $self.inner.version();

		$self.inner.set_version($self.version);
		let res = $code;
		$self.inner.set_version(old_version);

		res
	}};
}

/// Wraps around an existing `WorkflowCtx` and applies the given version when running any workflow steps.
pub struct VersionedWorkflowCtx<'a> {
	inner: &'a mut WorkflowCtx,
	version: usize,
}

impl<'a> VersionedWorkflowCtx<'a> {
	pub(crate) fn new(inner: &'a mut WorkflowCtx, version: usize) -> Self {
		VersionedWorkflowCtx { inner, version }
	}

	pub fn into_inner(self) -> &'a mut WorkflowCtx {
		self.inner
	}

	/// Creates a workflow ctx reference with a given version.
	pub fn v(&'a mut self, version: usize) -> VersionedWorkflowCtx<'a> {
		VersionedWorkflowCtx {
			inner: self.inner,
			version,
		}
	}

	/// Like `branch` but it does not add another layer of depth. Meant to be implemented and not used
	/// directly in workflows.
	pub fn step(&mut self) -> WorkflowCtx {
		let mut branch = self.inner.step();

		branch.set_version(self.version);

		branch
	}

	/// Creates a sub workflow builder.
	pub fn workflow<I>(&mut self, input: I) -> builder::sub_workflow::SubWorkflowBuilder<I>
	where
		I: WorkflowInput,
		<I as WorkflowInput>::Workflow: Workflow<Input = I>,
	{
		builder::sub_workflow::SubWorkflowBuilder::new(self.inner, self.version, input)
	}

	/// Run activity. Will replay on failure.
	pub async fn activity<I>(
		&mut self,
		input: I,
	) -> GlobalResult<<<I as ActivityInput>::Activity as Activity>::Output>
	where
		I: ActivityInput,
		<I as ActivityInput>::Activity: Activity<Input = I>,
	{
		wrap!(self, "activity", { self.inner.activity(input).await })
	}

	/// Joins multiple executable actions (activities, closures) and awaits them simultaneously. This does not
	/// short circuit in the event of an error to make sure activity side effects are recorded.
	pub async fn join<T: Executable>(&mut self, exec: T) -> GlobalResult<T::Output> {
		wrap!(self, "join", { exec.execute(self.inner).await })
	}

	/// Creates a signal builder.
	pub fn signal<T: Signal + Serialize>(&mut self, body: T) -> builder::signal::SignalBuilder<T> {
		builder::signal::SignalBuilder::new(self.inner, self.version, body)
	}

	/// Listens for a signal for a short time before setting the workflow to sleep. Once the signal is
	/// received, the workflow will be woken up and continue.
	pub async fn listen<T: Listen>(&mut self) -> GlobalResult<T> {
		wrap!(self, "listen", { self.inner.listen::<T>().await })
	}

	/// Execute a custom listener.
	pub async fn custom_listener<T: CustomListener>(
		&mut self,
		listener: &T,
	) -> GlobalResult<<T as CustomListener>::Output> {
		wrap!(self, "listen", {
			self.inner.custom_listener(listener).await
		})
	}

	/// Creates a signal builder.
	pub fn msg<M>(&mut self, body: M) -> builder::message::MessageBuilder<M>
	where
		M: Message,
	{
		builder::message::MessageBuilder::new(self.inner, self.version, body)
	}

	/// Runs workflow steps in a loop. **Ensure that there are no side effects caused by the code in this
	/// callback**. If you need side causes or side effects, use a native rust loop.
	pub async fn repeat<F, T>(&mut self, cb: F) -> GlobalResult<T>
	where
		F: for<'b> FnMut(&'b mut WorkflowCtx) -> AsyncResult<'b, Loop<T>>,
		T: Serialize + DeserializeOwned,
	{
		wrap!(self, "loop", { self.inner.repeat(cb).await })
	}

	pub async fn sleep(&mut self, duration: impl DurationToMillis) -> GlobalResult<()> {
		wrap!(self, "sleep", { self.inner.sleep(duration).await })
	}

	pub async fn sleep_until(&mut self, time: impl TsToMillis) -> GlobalResult<()> {
		wrap!(self, "sleep", { self.inner.sleep_until(time).await })
	}

	pub async fn listen_with_timeout<T: Listen>(
		&mut self,
		duration: impl DurationToMillis,
	) -> GlobalResult<Option<T>> {
		wrap!(self, "listen with timeout", {
			self.inner.listen_with_timeout::<T>(duration).await
		})
	}
}

// impl<'a> Deref for VersionedWorkflowCtx<'a> {
// 	type Target = WorkflowCtx;

// 	fn deref(&self) -> &Self::Target {
// 		self.inner
// 	}
// }
