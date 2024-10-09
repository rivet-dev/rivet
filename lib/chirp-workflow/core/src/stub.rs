//! Stubs provide the ability to create the blueprint for a workflow step without a workflow context, so its
//! execution can be deferred to later

// TODO: Add signal, workflow, message, stubs
// TODO: Versions for stubs

use async_trait::async_trait;
use global_error::GlobalResult;

use crate::{
	activity::{Activity, ActivityInput},
	ctx::WorkflowCtx,
	executable::{AsyncResult, Executable},
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
}

#[async_trait]
impl<I> Executable for ActivityStub<I>
where
	I: ActivityInput + Send + Sync,
	<I as ActivityInput>::Activity: Activity<Input = I>,
{
	type Output = <I::Activity as Activity>::Output;

	async fn execute(self, ctx: &mut WorkflowCtx) -> GlobalResult<Self::Output> {
		ctx.activity(self.inner).await
	}
}

// Wraps activity inputs for trait impl
pub fn activity<I>(input: I) -> ActivityStub<I>
where
	I: ActivityInput,
	<I as ActivityInput>::Activity: Activity<Input = I>,
{
	ActivityStub { inner: input }
}

pub struct VersionStub {
	#[allow(dead_code)]
	version: usize,
}

impl VersionStub {
	pub fn activity<I>(_input: I) -> ActivityStub<I>
	where
		I: ActivityInput,
		<I as ActivityInput>::Activity: Activity<Input = I>,
	{
		todo!();
	}
}

pub fn v<I>(version: usize) -> VersionStub {
	VersionStub { version }
}
