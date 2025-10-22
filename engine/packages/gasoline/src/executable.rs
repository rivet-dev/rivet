use std::{future::Future, pin::Pin};

use anyhow::Result;
use async_trait::async_trait;
use futures_util::future::join_all;

use crate::{ctx::WorkflowCtx, error::WorkflowResult};

/// Signifies a retryable executable entity in a workflow. For example: activity, tuple of activities (join).
#[async_trait]
pub trait Executable: Send + Sized + Sync {
	type Output: Send;

	async fn execute(self, ctx: &mut WorkflowCtx) -> Result<Self::Output>;

	/// Move the context's cursor to where it should be after this executable is executed.
	fn shift_cursor(&self, ctx: &mut WorkflowCtx) -> WorkflowResult<()>;
}

pub type AsyncResult<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

// Closure executable impl
#[async_trait]
impl<F, T> Executable for F
where
	F: for<'a> FnOnce(&'a mut WorkflowCtx) -> AsyncResult<'a, T> + Send + Sync,
	T: Send,
{
	type Output = T;

	#[tracing::instrument(skip_all)]
	async fn execute(self, ctx: &mut WorkflowCtx) -> Result<Self::Output> {
		let mut branch = ctx.branch().await?;

		// Move to next event
		self.shift_cursor(ctx)?;

		let res = (self)(&mut branch).await?;

		// Validate no leftover events
		branch.cursor().check_clear()?;

		Ok(res)
	}

	fn shift_cursor(&self, ctx: &mut WorkflowCtx) -> WorkflowResult<()> {
		ctx.cursor_mut().inc();
		Ok(())
	}
}

// Option executable impl
#[async_trait]
impl<T: Executable> Executable for Option<T> {
	type Output = Option<T::Output>;

	#[tracing::instrument(skip_all)]
	async fn execute(self, ctx: &mut WorkflowCtx) -> Result<Self::Output> {
		if let Some(inner) = self {
			let mut branch = ctx.clone();

			// Move to next event
			inner.shift_cursor(ctx)?;

			let res = inner.execute(&mut branch).await?;

			Ok(Some(res))
		} else {
			Ok(None)
		}
	}

	fn shift_cursor(&self, ctx: &mut WorkflowCtx) -> WorkflowResult<()> {
		ctx.cursor_mut().inc();
		Ok(())
	}
}

// Implements `Executable` for any tuple size
macro_rules! impl_tuple {
	($($args:ident),*) => {
		#[async_trait::async_trait]
		impl<$($args : Executable),*> Executable for ($($args),*) {
			type Output = ($(<$args as Executable>::Output),*);

			#[tracing::instrument(skip_all)]
			async fn execute(self, ctx: &mut WorkflowCtx) -> Result<Self::Output> {
				#[allow(non_snake_case)]
				let ($($args),*) = self;

				#[allow(non_snake_case)]
				let ($(mut $args),*) = ($(
					TupleHelper {
						branch: {
							let mut branch = ctx.clone();
							branch.set_parallelized();
							$args.shift_cursor(ctx)?;
							branch
						},
						exec: $args,
					}
				),*);

				// We don't use a try_join here because we do not want to short circuit any executables
				#[allow(non_snake_case)]
				let ($($args),*) = tokio::join!(
					$($args.exec.execute(&mut $args.branch)),*
				);

				// Handle errors here instead
				Ok(($($args?),*))
			}

			fn shift_cursor(&self, ctx: &mut WorkflowCtx) -> WorkflowResult<()> {
				#[allow(non_snake_case)]
				let ($($args),*) = self;

				$(
					$args.shift_cursor(ctx)?;
				)*

				Ok(())
			}
		}
	}
}

impl_tuple!(A, B);
impl_tuple!(A, B, C);
impl_tuple!(A, B, C, D);
impl_tuple!(A, B, C, D, E);

// Fixes ownership issues in macro
struct TupleHelper<T: Executable> {
	branch: WorkflowCtx,
	exec: T,
}

// Vec executable impl
#[async_trait]
impl<T: Executable> Executable for Vec<T> {
	type Output = Vec<T::Output>;

	#[tracing::instrument(skip_all)]
	async fn execute(self, ctx: &mut WorkflowCtx) -> Result<Self::Output> {
		if self.is_empty() {
			return Ok(Vec::new());
		}

		// Prepare branches and shift cursors
		let mut helpers = Vec::with_capacity(self.len());
		for exec in self {
			let mut branch = ctx.clone();
			branch.set_parallelized();
			exec.shift_cursor(ctx)?;
			helpers.push(TupleHelper { branch, exec });
		}

		// Execute all items in parallel
		let mut futures = Vec::with_capacity(helpers.len());
		for helper in helpers {
			// Move fields out of helper to avoid lifetime issues
			let TupleHelper { mut branch, exec } = helper;

			futures.push(async move { exec.execute(&mut branch).await });
		}

		// Wait for all to complete (no short-circuiting)
		let results = join_all(futures).await;

		// Handle errors after all have completed
		let mut outputs = Vec::with_capacity(results.len());
		for result in results {
			outputs.push(result?);
		}

		Ok(outputs)
	}

	fn shift_cursor(&self, ctx: &mut WorkflowCtx) -> WorkflowResult<()> {
		for exec in self {
			exec.shift_cursor(ctx)?;
		}
		Ok(())
	}
}
