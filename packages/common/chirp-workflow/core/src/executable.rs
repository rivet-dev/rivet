use std::{future::Future, pin::Pin};

use async_trait::async_trait;
use global_error::{GlobalError, GlobalResult};

use crate::{ctx::WorkflowCtx, error::WorkflowResult};

/// Signifies a retryable executable entity in a workflow. For example: activity, tuple of activities (join),
/// closure.
#[async_trait]
pub trait Executable: Send + Sized + Sync {
	type Output: Send;

	async fn execute(self, ctx: &mut WorkflowCtx) -> GlobalResult<Self::Output>;

	/// Move the context's cursor to where it should be after this executable is executed.
	fn shift_cursor(&self, ctx: &mut WorkflowCtx) -> WorkflowResult<()>;
}

pub type AsyncResult<'a, T> = Pin<Box<dyn Future<Output = GlobalResult<T>> + Send + 'a>>;

// Closure executable impl
#[async_trait]
impl<F, T> Executable for F
where
	F: for<'a> FnOnce(&'a mut WorkflowCtx) -> AsyncResult<'a, T> + Send + Sync,
	T: Send,
{
	type Output = T;

	async fn execute(self, ctx: &mut WorkflowCtx) -> GlobalResult<Self::Output> {
		let mut branch = ctx.branch().await.map_err(GlobalError::raw)?;

		// Move to next event
		self.shift_cursor(ctx).map_err(GlobalError::raw)?;

		let res = (self)(&mut branch).await?;

		// Validate no leftover events
		branch.cursor().check_clear().map_err(GlobalError::raw)?;

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

	async fn execute(self, ctx: &mut WorkflowCtx) -> GlobalResult<Self::Output> {
		if let Some(inner) = self {
			let mut branch = ctx.clone();

			// Move to next event
			inner.shift_cursor(ctx).map_err(GlobalError::raw)?;

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

			async fn execute(self, ctx: &mut WorkflowCtx) -> GlobalResult<Self::Output> {
				#[allow(non_snake_case)]
				let ($($args),*) = self;

				#[allow(non_snake_case)]
				let ($(mut $args),*) = ($(
					TupleHelper {
						branch: {
							let branch = ctx.clone();
							$args.shift_cursor(ctx).map_err(GlobalError::raw)?;

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
