use anyhow::Result;

#[cfg(unix)]
use tokio::signal::unix::{Signal, SignalKind, signal};

#[cfg(windows)]
use tokio::signal::windows::ctrl_c as windows_ctrl_c;

/// Cross-platform termination signal wrapper that handles:
/// - Unix: SIGTERM and SIGINT
/// - Windows: Ctrl+C
pub struct TermSignal {
	#[cfg(unix)]
	sigterm: Signal,
	#[cfg(unix)]
	sigint: Signal,
	#[cfg(windows)]
	ctrl_c: tokio::signal::windows::CtrlC,
}

impl TermSignal {
	/// Creates a new termination signal handler
	pub fn new() -> Result<Self> {
		Ok(Self {
			#[cfg(unix)]
			sigterm: signal(SignalKind::terminate())?,
			#[cfg(unix)]
			sigint: signal(SignalKind::interrupt())?,
			#[cfg(windows)]
			ctrl_c: windows_ctrl_c()?,
		})
	}

	/// Waits for the next termination signal
	pub async fn recv(&mut self) -> Option<()> {
		#[cfg(unix)]
		{
			tokio::select! {
				result = self.sigterm.recv() => result,
				result = self.sigint.recv() => result,
			}
		}

		#[cfg(windows)]
		{
			self.ctrl_c.recv().await
		}
	}
}
