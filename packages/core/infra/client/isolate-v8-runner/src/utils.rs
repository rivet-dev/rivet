pub mod tokio {
	use anyhow::*;
	use deno_core::unsync::MaskFutureAsSend;

	// Copied from deno-runtime tokio_util.rs
	fn create_basic_runtime() -> Result<::tokio::runtime::Runtime> {
		let event_interval = 61;
		let global_queue_interval = 31;
		let max_io_events_per_tick = 1024;

		::tokio::runtime::Builder::new_current_thread()
			.enable_io()
			.enable_time()
			.event_interval(event_interval)
			.global_queue_interval(global_queue_interval)
			.max_io_events_per_tick(max_io_events_per_tick)
			// This limits the number of threads for blocking operations (like for
			// synchronous fs ops) or CPU bound tasks like when we run dprint in
			// parallel for deno fmt.
			// The default value is 512, which is an unhelpfully large thread pool. We
			// don't ever want to have more than a couple dozen threads.
			.max_blocking_threads(32)
			.build()
			.map_err(Into::into)
	}

	// Copied from deno-runtime tokio_util.rs
	#[inline(always)]
	pub fn create_and_run_current_thread<F, R>(future: F) -> Result<R>
	where
		F: std::future::Future<Output = R> + 'static,
		R: Send + 'static,
	{
		let rt = create_basic_runtime()?;

		// Since this is the main future, we want to box it in debug mode because it tends to be fairly
		// large and the compiler won't optimize repeated copies. We also make this runtime factory
		// function #[inline(always)] to avoid holding the unboxed, unused future on the stack.

		#[cfg(debug_assertions)]
		// SAFETY: this is guaranteed to be running on a current-thread executor
		let future = Box::pin(unsafe { MaskFutureAsSend::new(future) });

		#[cfg(not(debug_assertions))]
		// SAFETY: this is guaranteed to be running on a current-thread executor
		let future = unsafe { MaskFutureAsSend::new(future) };

		let join_handle = rt.spawn(future);

		let r = rt.block_on(join_handle)?.into_inner();
		// Forcefully shutdown the runtime - we're done executing JS code at this
		// point, but there might be outstanding blocking tasks that were created and
		// latered "unrefed". They won't terminate on their own, so we're forcing
		// termination of Tokio runtime at this point.
		rt.shutdown_background();

		Ok(r)
	}
}
