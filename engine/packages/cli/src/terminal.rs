//! Interactive terminal input: confirmations and hidden prompts.

use std::{
	io::{BufRead, BufReader, IsTerminal, Write},
	time::Duration,
};

use anyhow::{Context, Result, bail};

/// How long to wait after a hidden prompt returns for the rest of a paste to
/// reach the terminal.
const PASTE_SETTLE: Duration = Duration::from_millis(50);
/// The exit code a shell reports for a process stopped by Ctrl-C.
const INTERRUPTED_EXIT_CODE: i32 = 130;

/// Asks `question` on the terminal and returns whether the answer was yes.
/// Fails when there is no terminal to ask on, telling the user to pass
/// `--yes` to `action` without confirming. The terminal is stdin, or
/// `/dev/tty` when stdin carries piped input such as a secret's value.
pub fn confirm(question: &str, action: &str) -> Result<bool> {
	let Some(mut input) = answer_input() else {
		bail!("no terminal to confirm on; pass --yes to {action} without confirming");
	};
	let mut stderr = std::io::stderr();
	write!(stderr, "{question} [y/N] ")?;
	stderr.flush()?;
	let mut answer = String::new();
	input
		.read_line(&mut answer)
		.context("read the confirmation from the terminal")?;
	let answer = answer.trim().to_ascii_lowercase();
	Ok(answer == "y" || answer == "yes")
}

fn answer_input() -> Option<Box<dyn BufRead>> {
	if !std::io::stderr().is_terminal() {
		return None;
	}
	let stdin = std::io::stdin();
	if stdin.is_terminal() {
		return Some(Box::new(stdin.lock()));
	}
	controlling_terminal().map(|tty| Box::new(BufReader::new(tty)) as Box<dyn BufRead>)
}

#[cfg(unix)]
fn controlling_terminal() -> Option<std::fs::File> {
	let tty = std::fs::File::open("/dev/tty").ok()?;
	tty.is_terminal().then_some(tty)
}

#[cfg(not(unix))]
fn controlling_terminal() -> Option<std::fs::File> {
	None
}

/// Prompts for a value without echoing it.
///
/// Ctrl-C restores the terminal's attributes, which the prompt changed to hide
/// the input, and exits with code 130. A paste of several lines is refused:
/// the prompt stops at the first line ending, and the rest would otherwise stay
/// in the terminal's input for the shell to run once the CLI exits, so it is
/// discarded.
pub async fn prompt_hidden(prompt: String, multi_line_hint: &str) -> Result<String> {
	let saved = tty::Saved::capture();
	let read = async move {
		tokio::task::spawn_blocking(move || rpassword::prompt_password(prompt)).await
	};
	let value = tokio::select! {
		// Poll the signal first so its handler is installed before the prompt
		// starts reading and could see Ctrl-C.
		biased;
		_ = tokio::signal::ctrl_c() => interrupted(&saved),
		result = read => match result.context("terminal prompt task failed")? {
			Ok(value) => value,
			Err(error) if error.kind() == std::io::ErrorKind::Interrupted => interrupted(&saved),
			Err(error) => return Err(error).context("read the value from the terminal"),
		},
	};

	// The Ctrl-C handler stays installed for the rest of the process, so keep
	// Ctrl-C stopping the CLI while it talks to the Cloud API.
	tokio::spawn(async {
		if tokio::signal::ctrl_c().await.is_ok() {
			std::process::exit(INTERRUPTED_EXIT_CODE);
		}
	});

	tokio::time::sleep(PASTE_SETTLE).await;
	if saved.discard_pending_input() {
		bail!(
			"the pasted value spans more than one line, which the prompt cannot take, so it was discarded; {multi_line_hint}"
		);
	}
	Ok(value)
}

fn interrupted(saved: &tty::Saved) -> ! {
	saved.restore();
	eprintln!();
	std::process::exit(INTERRUPTED_EXIT_CODE)
}

#[cfg(unix)]
mod tty {
	use std::{fs::File, mem::MaybeUninit, os::fd::AsRawFd};

	/// The controlling terminal's attributes from before a prompt changed them.
	pub struct Saved(Option<(File, libc::termios)>);

	impl Saved {
		pub fn capture() -> Self {
			let Ok(tty) = File::options().read(true).write(true).open("/dev/tty") else {
				return Saved(None);
			};
			let mut attrs = MaybeUninit::<libc::termios>::uninit();
			// SAFETY: `tty` is an open file descriptor and `attrs` is valid for
			// writes of one `termios`.
			if unsafe { libc::tcgetattr(tty.as_raw_fd(), attrs.as_mut_ptr()) } != 0 {
				return Saved(None);
			}
			// SAFETY: `tcgetattr` returned 0, so it initialized `attrs`.
			Saved(Some((tty, unsafe { attrs.assume_init() })))
		}

		pub fn restore(&self) {
			if let Some((tty, attrs)) = &self.0 {
				// SAFETY: `tty` is open and `attrs` came from `tcgetattr`.
				unsafe {
					libc::tcsetattr(tty.as_raw_fd(), libc::TCSANOW, attrs);
				}
			}
		}

		/// Discards input waiting on the terminal and reports whether there
		/// was any.
		pub fn discard_pending_input(&self) -> bool {
			let Some((tty, _)) = &self.0 else {
				return false;
			};
			let mut poll = libc::pollfd {
				fd: tty.as_raw_fd(),
				events: libc::POLLIN,
				revents: 0,
			};
			// SAFETY: `poll` points at one valid `pollfd`, and a zero timeout
			// does not block.
			let ready = unsafe { libc::poll(&mut poll, 1, 0) };
			if ready <= 0 || poll.revents & libc::POLLIN == 0 {
				return false;
			}
			// SAFETY: `tty` is an open terminal file descriptor.
			unsafe {
				libc::tcflush(tty.as_raw_fd(), libc::TCIFLUSH);
			}
			true
		}
	}
}

#[cfg(not(unix))]
mod tty {
	/// Terminal attributes are only saved on Unix; elsewhere this does nothing.
	pub struct Saved;

	impl Saved {
		pub fn capture() -> Self {
			Saved
		}

		pub fn restore(&self) {}

		pub fn discard_pending_input(&self) -> bool {
			false
		}
	}
}
