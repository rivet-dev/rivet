use anyhow::Result;

use tokio::{
	sync::{OnceCell, watch},
	task::JoinHandle,
};

#[cfg(unix)]
use tokio::signal::unix::{Signal, SignalKind, signal};

#[cfg(windows)]
use tokio::signal::windows::ctrl_c as windows_ctrl_c;

const FORCE_CLOSE_THRESHOLD: usize = 3;

static HANDLER_CELL: OnceCell<(watch::Receiver<bool>, JoinHandle<()>)> = OnceCell::const_new();

/// Cross-platform termination signal wrapper that handles:
/// - Unix: SIGTERM and SIGINT
/// - Windows: Ctrl+C
struct TermSignalHandler {
	count: usize,
	tx: watch::Sender<bool>,

	#[cfg(unix)]
	sigterm: Signal,
	#[cfg(unix)]
	sigint: Signal,
	#[cfg(windows)]
	ctrl_c: tokio::signal::windows::CtrlC,
}

impl TermSignalHandler {
	/// Returns existing termination signal handler or initializes it.
	fn new() -> Result<Self> {
		tracing::debug!("initialized termination signal handler");

		Ok(Self {
			count: 0,
			tx: watch::channel(false).0,

			#[cfg(unix)]
			sigterm: signal(SignalKind::terminate())?,
			#[cfg(unix)]
			sigint: signal(SignalKind::interrupt())?,
			#[cfg(windows)]
			ctrl_c: windows_ctrl_c()?,
		})
	}

	async fn run(mut self) {
		loop {
			#[cfg(unix)]
			{
				tokio::select! {
					_ = self.sigterm.recv() => {}
					_ = self.sigint.recv() => {}
				}
			}

			#[cfg(windows)]
			{
				self.ctrl_c.recv().await;
			}

			self.count += 1;

			if self.count == 1 {
				tracing::info!("received SIGTERM");
			} else {
				tracing::warn!(count=%self.count, "received another SIGTERM");
			}

			if self.tx.send(self.count >= FORCE_CLOSE_THRESHOLD).is_err() {
				tracing::debug!("no sigterm subscribers");
			}
		}
	}
}

pub struct TermSignal(watch::Receiver<bool>);

impl TermSignal {
	pub async fn new() -> Self {
		let rx = HANDLER_CELL
			.get_or_init(|| {
				let term_signal = TermSignalHandler::new()
					.expect("failed initializing termination signal handler");
				let rx = term_signal.tx.subscribe();

				let join_handle = tokio::spawn(term_signal.run());

				std::future::ready((rx, join_handle))
			})
			.await
			.0
			.clone();

		TermSignal(rx)
	}

	/// Returns true if the user should abort any graceful attempt to shutdown and shutdown immediately.
	pub async fn recv(&mut self) -> bool {
		let _ = self.0.changed().await;
		*self.0.borrow()
	}

	pub fn stop() {
		if let Some((_, join_handle)) = HANDLER_CELL.get() {
			join_handle.abort();
		}
	}
}
