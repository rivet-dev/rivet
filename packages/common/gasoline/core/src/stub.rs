//! Stubs provide the ability to create the blueprint for a workflow step without a workflow context, so its
//! execution can be deferred to later

// TODO: Add signal, workflow, message, stubs

use std::marker::PhantomData;

use anyhow::Result;
use async_trait::async_trait;

use crate::{
	activity::{Activity, ActivityInput},
	ctx::WorkflowCtx,
	error::WorkflowResult,
	executable::{AsyncResult, Executable},
	history::{event::EventId, removed::Removed},
};

// Must wrap all closures being used as executables in this function due to
// https://github.com/rust-lang/rust/issues/70263
pub fn closure<F, T: Send>(f: F) -> F
where
	F: for<'a> FnOnce(&'a mut WorkflowCtx) -> AsyncResult<'a, T> + Send,
{
	f
}

pub struct ActivityStub<I>
where
	I: ActivityInput,
	<I as ActivityInput>::Activity: Activity<Input = I>,
{
	inner: I,
	version: Option<usize>,
}

#[async_trait]
impl<I> Executable for ActivityStub<I>
where
	I: ActivityInput + Send + Sync,
	<I as ActivityInput>::Activity: Activity<Input = I>,
{
	type Output = <I::Activity as Activity>::Output;

	#[tracing::instrument(skip_all)]
	async fn execute(self, ctx: &mut WorkflowCtx) -> Result<Self::Output> {
		if let Some(version) = self.version {
			ctx.v(version).activity(self.inner).await
		} else {
			ctx.activity(self.inner).await
		}
	}

	fn shift_cursor(&self, ctx: &mut WorkflowCtx) -> WorkflowResult<()> {
		let event_id = EventId::new(I::Activity::NAME, &self.inner);
		let history_res = ctx
			.cursor()
			.compare_activity(self.version.unwrap_or(ctx.version()), &event_id)?;
		let location = ctx.cursor().current_location_for(&history_res);

		ctx.cursor_mut().update(&location);

		Ok(())
	}
}

// Wraps activity inputs for trait impl
pub fn activity<I>(input: I) -> ActivityStub<I>
where
	I: ActivityInput,
	<I as ActivityInput>::Activity: Activity<Input = I>,
{
	ActivityStub {
		inner: input,
		version: None,
	}
}

pub struct VersionStub {
	version: usize,
}

impl VersionStub {
	pub fn activity<I>(self, input: I) -> ActivityStub<I>
	where
		I: ActivityInput,
		<I as ActivityInput>::Activity: Activity<Input = I>,
	{
		ActivityStub {
			inner: input,
			version: Some(self.version),
		}
	}
}

pub fn v(version: usize) -> VersionStub {
	VersionStub { version }
}

pub struct RemovedStub<T: Removed>(PhantomData<T>);

#[async_trait]
impl<T: Removed + Send + Sync> Executable for RemovedStub<T> {
	type Output = ();

	#[tracing::instrument(skip_all)]
	async fn execute(self, ctx: &mut WorkflowCtx) -> Result<Self::Output> {
		ctx.removed::<T>().await
	}

	fn shift_cursor(&self, ctx: &mut WorkflowCtx) -> WorkflowResult<()> {
		ctx.cursor_mut().inc();
		Ok(())
	}
}

pub fn removed<T: Removed>() -> RemovedStub<T> {
	RemovedStub(PhantomData)
}
