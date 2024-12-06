use std::process::ExitCode;

/// This error type will exit without printing anything.
///
/// This should be used for errors where the error was already printed and the program should exit
/// gracefully.
pub struct GracefulExit;

impl std::fmt::Debug for GracefulExit {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("GracefulExit").finish()
	}
}

impl std::fmt::Display for GracefulExit {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "GracefulExit occurred")
	}
}

impl std::error::Error for GracefulExit {}

/// This error type will exit without printing anything.
///
/// This indicates the program was exited with a Ctrl-C event.
pub struct CtrlC;

impl std::fmt::Debug for CtrlC {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CtrlC").finish()
	}
}

impl std::fmt::Display for CtrlC {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "CtrlC occurred")
	}
}

impl std::error::Error for CtrlC {}

/// This error type will exit with a message, but will not report the error to Rivet.
///
/// This should be used for errors where the user input is incorrect.
pub struct UserError {
	pub message: String,
}

impl UserError {
	pub fn new(msg: impl ToString) -> Self {
		UserError {
			message: msg.to_string(),
		}
	}
}

impl std::fmt::Debug for UserError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("UserError")
			.field("message", &self.message)
			.finish()
	}
}

impl std::fmt::Display for UserError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.message)
	}
}

impl std::error::Error for UserError {}

/// This error type will exit with the given exit code without printing anything.
///
/// This should only be used for passthrough commands.
pub struct PassthroughExitCode {
	exit_code: ExitCode,
}

impl PassthroughExitCode {
	pub fn new(exit_code: ExitCode) -> Self {
		Self { exit_code }
	}

	pub fn exit_code(&self) -> ExitCode {
		self.exit_code
	}
}

impl std::fmt::Debug for PassthroughExitCode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("GracefulExit")
			.field("exit_code", &self.exit_code)
			.finish()
	}
}

impl std::fmt::Display for PassthroughExitCode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "GracefulExit occurred")
	}
}

impl std::error::Error for PassthroughExitCode {}
