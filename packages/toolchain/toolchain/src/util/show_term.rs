use anyhow::*;
use std::process::{Child, Command};

#[cfg(target_os = "linux")]
struct Terminal {
	/// The name of the terminal emulator command
	name: &'static str,
	/// The flag to pass the command to the terminal emulator
	prompt_str: &'static [&'static str],
}

/// Terminals that don't work (note, more work might make them work):
///
/// - guake (runs the whole window, doesn't handle closing)
/// - upterm (doesn't have an arg to pass a command it)
/// - x-terminal-emulator
/// - tilda (doesn't show automatically)
/// - terminator (issues running the command)
/// - xfce4-terminal (issues running the command)
#[cfg(target_os = "linux")]
const TERMINALS: [Terminal; 7] = [
	Terminal {
		name: "kitty",
		prompt_str: &["-e"],
	},
	Terminal {
		name: "konsole",
		prompt_str: &["-e"],
	},
	Terminal {
		name: "gnome-terminal",
		prompt_str: &["--"],
	},
	Terminal {
		name: "st",
		prompt_str: &["-e"],
	},
	Terminal {
		name: "tilix",
		prompt_str: &["-e"],
	},
	Terminal {
		name: "urxvt",
		prompt_str: &["-e"],
	},
	Terminal {
		name: "xterm",
		prompt_str: &["-e"],
	},
];

pub async fn show_term(args: &[String]) -> Result<Child> {
	#[cfg(target_os = "windows")]
	let child: Child = Command::new("cmd.exe")
		.arg("/C")
		.arg("start")
		.arg("cmd.exe")
		.arg("/K")
		.args(args)
		.spawn()
		.expect("cmd.exe failed to start");

	#[cfg(target_os = "macos")]
	let child: Child = {
		// This script will run from home, so we need top change to the
		// project directory before running the command.
		let current_dir = std::env::current_dir()?
			.into_os_string()
			.into_string()
			.unwrap();

		// Create script tempfile
		let script_temp_file = tempfile::Builder::new()
			.prefix("rivet_term_")
			.suffix(".command")
			.tempfile()?;
		let script_path = script_temp_file.path().to_path_buf();
		script_temp_file.keep()?;

		// Write the script content to the script file
		let command_to_run = format!(
			"cd \"{}\" && {} && rm \"{}\"",
			shell_escape(&current_dir),
			args.iter()
				.map(|x| shell_escape(x))
				.collect::<Vec<_>>()
				.join(" "),
			shell_escape(&script_path.display().to_string()),
		);
		std::fs::write(&script_path, format!("#!/bin/bash\n{}", command_to_run))?;
		std::fs::set_permissions(
			&script_path,
			std::os::unix::fs::PermissionsExt::from_mode(0o755),
		)?;

		// Use `open` to run the script
		Command::new("open")
			.arg(&script_path)
			.spawn()
			.expect("Failed to open script")
	};

	#[cfg(target_os = "linux")]
	let child: Child = {
		let mut args = args.to_vec();

		// TODO(forest): For Linux, the code is trying to find an
		// available terminal emulator from a predefined list and
		// then run the command in it. However, the way to run a
		// command in a terminal emulator can vary between different
		// emulators. The -e flag used here might not work for all
		// of them.
		let mut command = None;

		for terminal in TERMINALS {
			if which::which(terminal.name).is_ok() {
				command = Some(terminal);
				break;
			}
		}

		match command {
			Some(terminal) => {
				// See if they have bash installed. If not, fallback to sh
				let shell = if which::which("bash").is_ok() {
					"bash"
				} else {
					"sh"
				};

				// Insert the flag --inside-terminal right after `sidekick`
				// in the args. The only args before it are the binary path
				// to the binary and `sidekick` itself, so it can go at the
				// 2nd index.
				args.insert(2, "--inside-terminal".to_string());

				// Add a "press any key to continue" message to the end of
				// the arguments to be run
				args.append(
					vec![
						"&&",
						"read",
						"-n",
						"1",
						"-s",
						"-r",
						"-p",
						"\"Press any key to continue\"",
					]
					.iter()
					.map(|x| x.to_string())
					.collect::<Vec<_>>()
					.as_mut(),
				);

				args = vec![args.join(" ")];

				Command::new(terminal.name)
					// This is the flag to run a command in the
					// terminal. Most will use -e, but some might use
					// something different.
					.args(terminal.prompt_str)
					// We pass everything to a shell manually so that we can
					// pass an entire string of the rest of the commands.
					// This is more consistent across terminals on linux.
					.arg(shell)
					.arg("-c")
					.args(&args)
					.spawn()
					.expect("Terminal emulator failed to start")
			}
			None => {
				panic!("No terminal emulator found");
			}
		}
	};

	Ok(child)
}

#[cfg(target_os = "macos")]
fn shell_escape(s: &str) -> String {
	if s.is_empty() {
		return String::from("''");
	}
	if !s.contains(|c: char| c.is_whitespace() || "[]{}()*;'\"\\|<>~&^$?!`".contains(c)) {
		return s.to_string();
	}
	let mut result = String::with_capacity(s.len() + 2);
	result.push('\'');
	for c in s.chars() {
		if c == '\'' {
			result.push_str("'\\''");
		} else {
			result.push(c);
		}
	}
	result.push('\'');
	result
}

#[cfg(target_os = "linux")]
#[cfg(test)]
mod tests {
	use std::fs;
	use std::path::Path;
	use std::process::Command;

	use super::TERMINALS;

	#[test]
	#[ignore]
	/// This test makes sure that the configuration to run a command in each
	/// terminal works. It shouldn't run in CI, since it would be difficult to
	/// configure. It can be run locally if each terminal in the const is
	/// installed.
	fn test_terminals() {
		for terminal in TERMINALS {
			let file_name = format!("{}.txt", terminal.name);

			let mut args = Vec::new();

			args.push(format!("touch {}", file_name));

			let output = Command::new(terminal.name)
				.args(terminal.prompt_str)
				.args(&args)
				.output()
				.expect("Failed to execute command");

			assert!(output.status.success(), "Command failed: {}", terminal.name);

			let file_path = Path::new(&file_name);
			assert!(file_path.exists(), "File does not exist: {}", file_name);

			// Clean up the file
			fs::remove_file(file_path).expect("Failed to remove file");
		}
	}
}
