use std::{future::Future, pin::Pin};

use async_trait::async_trait;
use global_error::{GlobalError, GlobalResult};

use crate::ctx::WorkflowCtx;

/// Signifies a retryable executable entity in a workflow. For example: activity, tuple of activities (join),
/// closure.
#[async_trait]
pub trait Executable: Send + Sized {
	type Output: Send;

	async fn execute(self, ctx: &mut WorkflowCtx) -> GlobalResult<Self::Output>;
}

pub type AsyncResult<'a, T> = Pin<Box<dyn Future<Output = GlobalResult<T>> + Send + 'a>>;

// Closure executable impl
#[async_trait]
impl<F, T> Executable for F
where
	F: for<'a> FnOnce(&'a mut WorkflowCtx) -> AsyncResult<'a, T> + Send,
	T: Send,
{
	type Output = T;

	async fn execute(self, ctx: &mut WorkflowCtx) -> GlobalResult<Self::Output> {
		let mut branch = ctx.branch().await.map_err(GlobalError::raw)?;

		let res = (self)(&mut branch).await?;

		// Validate no leftover events
		branch.cursor().check_clear().map_err(GlobalError::raw)?;

		// Move to next event
		ctx.cursor_mut().update(branch.cursor().root());

		Ok(res)
	}
}

// Option executable impl
#[async_trait]
impl<T: Executable> Executable for Option<T> {
	type Output = Option<T::Output>;

	async fn execute(self, ctx: &mut WorkflowCtx) -> GlobalResult<Self::Output> {
		let mut branch = ctx.step();

		if let Some(inner) = self {
			inner.execute(&mut branch).await.map(Some)
		} else {
			Ok(None)
		}
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
						branch: ctx.step(),
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
