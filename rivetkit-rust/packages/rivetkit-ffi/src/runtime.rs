//! The process-global runtime: tokio, the wire, and the host callbacks.

use std::sync::{Arc, OnceLock};

use anyhow::{Result, anyhow, bail};
use parking_lot::Mutex;
use rivetkit_host_adapter::wire::messages::InitOptions;
use rivetkit_host_adapter::wire::{Event, Status, Transport, Wire};
use tracing::field::{Field, Visit};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Non-null form of [`crate::abi::EventCallback`].
pub type EventFn = extern "C" fn(u32, u64, u64, u64, *const u8, usize, *const u8, usize, u64);

/// Non-null form of [`crate::abi::CompleteCallback`].
pub type CompleteFn = extern "C" fn(u64, i32, *const u8, usize, *const u8, usize, u64);

pub struct Runtime {
	/// Kept alive for the life of the process.
	_tokio: tokio::runtime::Runtime,
	pub wire: Arc<Wire>,
	on_event: EventFn,
	on_complete: CompleteFn,
}

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static INIT: Mutex<()> = Mutex::new(());

/// Calls the host's two function pointers. Calling an `extern "C" fn` pointer
/// with pointers into live Rust buffers is safe Rust.
struct FnTransport {
	on_event: EventFn,
	on_complete: CompleteFn,
}

impl Transport for FnTransport {
	fn event(&self, event: Event<'_>) {
		(self.on_event)(
			event.kind as u32,
			event.actor_type,
			event.actor,
			event.id,
			event.header.as_ptr(),
			event.header.len(),
			event.body.as_ptr(),
			event.body.len(),
			event.lease,
		);
	}

	fn complete(&self, token: u64, status: Status, header: &[u8], body: &[u8], lease: u64) {
		(self.on_complete)(
			token,
			status as i32,
			header.as_ptr(),
			header.len(),
			body.as_ptr(),
			body.len(),
			lease,
		);
	}
}

/// Starts the runtime once. Later calls must pass the same callbacks.
pub fn init(on_event: EventFn, on_complete: CompleteFn, options: InitOptions) -> Result<()> {
	// Serialize check-then-set so two racing calls cannot both "succeed" with
	// different callbacks.
	let _init = INIT.lock();
	if let Some(rt) = RUNTIME.get() {
		if rt.on_event as usize != on_event as usize
			|| rt.on_complete as usize != on_complete as usize
		{
			bail!(
				"rivetkit-ffi is already initialized with different callbacks \
				 (was the SDK loaded twice, for example by a reloaded class loader?)"
			);
		}
		return Ok(());
	}
	let tokio = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.thread_name("rivetkit")
		.build()?;
	let wire = Wire::new(
		Box::new(FnTransport {
			on_event,
			on_complete,
		}),
		tokio.handle().clone(),
	);
	let _ = RUNTIME.set(Runtime {
		_tokio: tokio,
		wire,
		on_event,
		on_complete,
	});
	install_log_bridge(options.log_filter.as_deref());
	Ok(())
}

pub fn get() -> Result<&'static Runtime> {
	RUNTIME
		.get()
		.ok_or_else(|| anyhow!("rivet_init has not been called"))
}

/// Forwards `tracing` records at or above the filter to the host as `LOG` events.
/// Does nothing if the process already has a global subscriber.
fn install_log_bridge(filter: Option<&str>) {
	let filter = tracing_subscriber::EnvFilter::try_new(filter.unwrap_or("warn"))
		.unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
	let _ = tracing_subscriber::registry()
		.with(filter)
		.with(HostLog)
		.try_init();
}

struct HostLog;

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for HostLog {
	fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
		let Some(rt) = RUNTIME.get() else { return };
		let mut message = Message(String::new());
		event.record(&mut message);
		let metadata = event.metadata();
		let level = match *metadata.level() {
			tracing::Level::ERROR => "error",
			tracing::Level::WARN => "warn",
			tracing::Level::INFO => "info",
			tracing::Level::DEBUG => "debug",
			tracing::Level::TRACE => "trace",
		};
		rt.wire.log(level, metadata.target(), message.0);
	}
}

/// The `message` field first, then other fields as `key=value`.
struct Message(String);

impl Visit for Message {
	fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
		use std::fmt::Write;
		if field.name() == "message" {
			let rest = std::mem::take(&mut self.0);
			let _ = write!(self.0, "{value:?}");
			if !rest.is_empty() {
				let _ = write!(self.0, " {rest}");
			}
		} else {
			if !self.0.is_empty() {
				self.0.push(' ');
			}
			let _ = write!(self.0, "{}={value:?}", field.name());
		}
	}

	fn record_str(&mut self, field: &Field, value: &str) {
		if field.name() == "message" {
			self.record_debug(field, &format_args!("{value}"));
		} else {
			self.record_debug(field, &value);
		}
	}
}
