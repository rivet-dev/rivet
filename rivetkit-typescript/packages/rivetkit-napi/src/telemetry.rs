//! Node-side telemetry glue. Export itself lives in `rivetkit_core::telemetry::export`.

/// Forwards the OpenTelemetry SDK's own diagnostics to the JavaScript logger.
///
/// The SDK reports dropped spans and export failures through Rust `tracing`,
/// which prints to stdout in a different format from the actor's Pino logs.
/// This layer hands those events to a JS callback instead, so an operator sees
/// them alongside everything else the actor logs.
pub(crate) mod sdk_log_bridge {
	use napi::bindgen_prelude::*;
	use napi::threadsafe_function::{ErrorStrategy, ThreadSafeCallContext, ThreadsafeFunction};
	// Forced-sync: read from inside a tracing layer callback, which is a sync
	// context and never spans an await.
	use parking_lot::RwLock;
	use tracing::field::{Field, Visit};
	use tracing_subscriber::Layer;
	use tracing_subscriber::layer::Context;

	/// One SDK diagnostic, flattened for the JavaScript side.
	pub(crate) struct SdkLogEvent {
		pub(crate) name: String,
		pub(crate) message: String,
	}

	/// The most recently installed sink. It is replaceable rather than set
	/// once, because a Node worker thread that installed it can exit, after
	/// which its callback silently drops every event. The next registry to
	/// start, on whichever thread, takes over.
	static SINK: RwLock<Option<ThreadsafeFunction<SdkLogEvent, ErrorStrategy::Fatal>>> =
		RwLock::new(None);

	/// Installs the JavaScript sink, replacing any earlier one.
	///
	/// The threadsafe function is unreferenced. A referenced one counts as live
	/// work on the Node event loop, so a process that had registered the sink
	/// would never exit on its own. Warnings still cross while the application
	/// is running; the sink just stops being a reason to keep running.
	pub(crate) fn install(env: Env, callback: JsFunction) -> Result<()> {
		let mut tsfn =
			callback.create_threadsafe_function(0, |ctx: ThreadSafeCallContext<SdkLogEvent>| {
				let mut object = ctx.env.create_object()?;
				object.set("name", ctx.value.name)?;
				object.set("message", ctx.value.message)?;
				Ok(vec![object.into_unknown()])
			})?;
		tsfn.unref(&env)?;
		*SINK.write() = Some(tsfn);
		Ok(())
	}

	#[derive(Default)]
	struct FieldCollector {
		name: String,
		message: String,
	}

	impl Visit for FieldCollector {
		fn record_str(&mut self, field: &Field, value: &str) {
			match field.name() {
				"name" => self.name = value.to_owned(),
				"message" => self.message = value.to_owned(),
				_ => {}
			}
		}

		fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
			let rendered = format!("{value:?}");
			match field.name() {
				"name" => self.name = rendered,
				"message" => self.message = rendered,
				_ => {}
			}
		}
	}

	pub(crate) struct SdkLogLayer;

	impl<S: tracing::Subscriber> Layer<S> for SdkLogLayer {
		fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
			let sink = SINK.read();
			let Some(sink) = sink.as_ref() else {
				return;
			};
			let mut fields = FieldCollector::default();
			event.record(&mut fields);
			sink.call(
				SdkLogEvent {
					name: fields.name,
					message: fields.message,
				},
				napi::threadsafe_function::ThreadsafeFunctionCallMode::NonBlocking,
			);
		}
	}
}
