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
