use std::{fs, path::Path, sync::mpsc, time::Duration};

use anyhow::*;
use utils::var;

mod container;
mod log_shipper;
mod throttle;
mod utils;

/// Maximum length of a single log line
const MAX_LINE_BYTES: usize = 1024;
/// Maximum number of bytes to buffer before dropping logs
const MAX_BUFFER_BYTES: usize = 1024 * 1024;
// 7 day logs retention
const LOGS_RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

fn main() -> Result<()> {
	let actor_path = std::env::args()
		.skip(1)
		.next()
		.context("`actor_path` arg required")?;
	let actor_path = Path::new(&actor_path);

	rivet_logs::Logs::new(actor_path.join("logs"), LOGS_RETENTION).start_sync()?;

	// Write PID to file
	fs::write(
		actor_path.join("pid"),
		std::process::id().to_string().as_bytes(),
	)?;

	let root_user_enabled = var("ROOT_USER_ENABLED")? == "1";
	let vector_socket_addr: Option<String> = var("VECTOR_SOCKET_ADDR")
		.ok()
		.map(|x| x.parse())
		.transpose()
		.context("failed to parse vector socket addr")?;
	let actor_id = var("ACTOR_ID")?;

	let (shutdown_tx, shutdown_rx) = mpsc::sync_channel(1);

	// Start log shipper
	let (msg_tx, log_shipper_thread) = if let Some(vector_socket_addr) = vector_socket_addr {
		let (msg_tx, msg_rx) =
			mpsc::sync_channel::<log_shipper::ReceivedMessage>(MAX_BUFFER_BYTES / MAX_LINE_BYTES);
		let log_shipper = log_shipper::LogShipper {
			shutdown_rx,
			msg_rx,
			vector_socket_addr,
			actor_id,
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
