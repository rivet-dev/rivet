use std::{
	fs,
	net::SocketAddr,
	os::fd::AsRawFd,
	path::{Path, PathBuf},
	sync::mpsc,
};

use anyhow::*;
use utils::{var, Stakeholder};

mod container;
mod log_shipper;
mod throttle;
mod utils;

/// Maximum length of a single log line
const MAX_LINE_BYTES: usize = 1024;

/// Maximum number of bytes to buffer before dropping logs
const MAX_BUFFER_BYTES: usize = 1024 * 1024;

fn main() -> Result<()> {
	let actor_path = std::env::args()
		.skip(1)
		.next()
		.context("`actor_path` arg required")?;
	let actor_path = Path::new(&actor_path);

	redirect_logs(actor_path.join("log"))?;

	// Write PID to file
	fs::write(
		actor_path.join("pid"),
		std::process::id().to_string().as_bytes(),
	)?;

	let root_user_enabled = var("ROOT_USER_ENABLED")? == "1";
	let vector_socket_addr: Option<SocketAddr> = var("VECTOR_SOCKET_ADDR")
		.ok()
		.map(|x| x.parse())
		.transpose()
		.context("failed to parse vector socket addr")?;
	let stakeholder = match var("STAKEHOLDER").ok() {
		Some(x) if x == "dynamic_server" => Stakeholder::DynamicServer {
			server_id: var("SERVER_ID")?,
		},
		Some(x) => bail!("invalid actor stakeholder: {x}"),
		None => bail!("no actor stakeholder specified"),
	};

	let (shutdown_tx, shutdown_rx) = mpsc::sync_channel(1);

	// Start log shipper
	let (msg_tx, log_shipper_thread) = if let Some(vector_socket_addr) = vector_socket_addr {
		let (msg_tx, msg_rx) =
			mpsc::sync_channel::<log_shipper::ReceivedMessage>(MAX_BUFFER_BYTES / MAX_LINE_BYTES);
		let log_shipper = log_shipper::LogShipper {
			shutdown_rx,
			msg_rx,
			vector_socket_addr,
			stakeholder,
		};
		let log_shipper_thread = log_shipper.spawn();
		(Some(msg_tx), Some(log_shipper_thread))
	} else {
		(None, None)
	};

	// Run the container
	let exit_code = match container::run(msg_tx.clone(), &actor_path, root_user_enabled) {
		Result::Ok(exit_code) => exit_code,
		Err(err) => {
			eprintln!("run container failed: {err:?}");
			container::send_message(
				&msg_tx,
				None,
				log_shipper::StreamType::StdErr,
				format!("Aborting"),
			);

			1
		}
	};

	// Shutdown all threads
	match shutdown_tx.send(()) {
		Result::Ok(_) => {
			println!("Sent shutdown signal");
		}
		Err(err) => {
			eprintln!("Failed to send shutdown signal: {err:?}");
		}
	}

	// Wait for log shipper to finish
	drop(msg_tx);
	if let Some(log_shipper_thread) = log_shipper_thread {
		match log_shipper_thread.join() {
			Result::Ok(_) => {}
			Err(err) => {
				eprintln!("log shipper failed: {err:?}")
			}
		}
	}

	fs::write(
		actor_path.join("exit-code"),
		exit_code.to_string().as_bytes(),
	)?;

	std::process::exit(exit_code)
}

fn redirect_logs(log_file_path: PathBuf) -> Result<()> {
	println!("Redirecting all logs to {}", log_file_path.display());
	let log_file = fs::OpenOptions::new()
		.write(true)
		.create(true)
		.append(true)
		.open(log_file_path)?;
	let log_fd = log_file.as_raw_fd();

	nix::unistd::dup2(log_fd, nix::libc::STDOUT_FILENO)?;
	nix::unistd::dup2(log_fd, nix::libc::STDERR_FILENO)?;

	Ok(())
}
