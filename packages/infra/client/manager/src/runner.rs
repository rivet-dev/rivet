use std::{
	os::unix::process::CommandExt,
	path::{Path, PathBuf},
	process::Stdio,
	result::Result::{Err, Ok},
	sync::Arc,
	time::Duration,
};

use anyhow::*;
use futures_util::{
	stream::{FuturesUnordered, SplitSink},
	FutureExt, SinkExt, StreamExt,
};
use nix::{
	errno::Errno,
	sys::{
		signal::{kill, Signal},
		wait::{waitpid, WaitStatus},
	},
	unistd::{fork, pipe, read, setsid, write, ForkResult, Pid},
};
use pegboard_config::runner_protocol;
use tokio::{fs, net::TcpStream, sync::Mutex};
use tokio_tungstenite::{tungstenite::protocol::Message, WebSocketStream};

use crate::utils;

/// How often to check that a PID is still running when observing actor state.
const PID_POLL_INTERVAL: Duration = Duration::from_millis(1000);
/// How long before killing a runner with a socket if it has not pinged.
const PING_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
enum ObservationState {
	Exited,
	Running,
	Dead,
}

// NOTE: Cloneable because this is just a handle
#[derive(Clone)]
pub struct Handle {
	pid: Pid,
	working_path: PathBuf,
	comms: Comms,
}

impl Handle {
	pub fn from_pid(comms: Comms, pid: Pid, working_path: PathBuf) -> Self {
		Handle {
			pid,
			working_path,
			comms,
		}
	}

	pub async fn attach_socket(&self, ws_stream: WebSocketStream<TcpStream>) -> Result<()> {
		match &self.comms {
			Comms::Basic => bail!("attempt to attach socket to basic runner"),
			Comms::Socket(tx) => {
				let mut guard = tx.lock().await;

				if guard.is_some() {
					tracing::warn!(pid=?self.pid, "runner received another socket");
				}

				let (ws_tx, mut ws_rx) = ws_stream.split();

				*guard = Some(ws_tx);

				// Spawn a new thread to handle incoming messages
				let self2 = self.clone();
				tokio::task::spawn(async move {
					let kill = loop {
						match tokio::time::timeout(PING_TIMEOUT, ws_rx.next()).await {
							Ok(msg) => match msg {
								Some(Ok(Message::Ping(_))) => {}
								Some(Ok(Message::Close(_))) | None => {
									tracing::debug!(pid=?self2.pid, "runner socket closed");
									break false;
								}
								Some(Ok(msg)) => {
									tracing::warn!(pid=?self2.pid, ?msg, "unexpected message in runner socket")
								}
								Some(Err(err)) => {
									tracing::error!(pid=?self2.pid, ?err, "runner socket error");
									break true;
								}
							},
							Err(_) => {
								tracing::error!(pid=?self2.pid, "socket timed out, killing runner");

								break true;
							}
						}
					};

					if kill {
						if let Err(err) = self2.signal(Signal::SIGKILL) {
							// TODO: This should hard error the manager?
							tracing::error!(pid=?self2.pid, %err, "failed to kill runner");
						}
					}
				});
			}
		}

		Ok(())
	}

	pub async fn send(&self, packet: &runner_protocol::ToRunner) -> Result<()> {
		match &self.comms {
			Comms::Basic => bail!("attempt to send socket message to basic runner"),
			Comms::Socket(socket) => {
				// Wait for socket to connect
				let mut attempts = 0;
				let mut guard = loop {
					{
						let guard = socket.lock().await;
						if guard.is_some() {
							break guard;
						}
					}

					attempts += 1;
					if attempts > 15 {
						bail!("timed out waiting for runner socket to connect");
					}

					tokio::time::sleep(std::time::Duration::from_millis(125)).await;
				};

				let socket = guard.as_mut().expect("should exist");
				let buf = serde_json::to_vec(packet)?;
				socket.send(Message::Binary(buf)).await?;
			}
		}

		Ok(())
	}

	pub fn spawn_orphaned(
		comms: Comms,
		runner_binary_path: &Path,
		working_path: PathBuf,
		env: &[(&str, String)],
	) -> Result<Self> {
		// Prepare the arguments for the runner
		let runner_args = vec![working_path.to_str().context("bad path")?];

		// TODO: Do pipes have to be manually deleted here?
		// Pipe communication between processes
		let (pipe_read, pipe_write) = pipe()?;

		// NOTE: This is why we fork the process twice: https://stackoverflow.com/a/5386753
		match unsafe { fork() }.context("process first fork failed")? {
			ForkResult::Parent { child } => {
				// Close the writing end of the pipe in the parent
				nix::unistd::close(pipe_write)?;

				// Ensure that the child process spawned successfully
				match waitpid(child, None).context("waitpid failed")? {
					WaitStatus::Exited(_, 0) => {
						// Read the second child's PID from the pipe
						let mut buf = [0u8; 4];
						read(pipe_read, &mut buf)?;
						let orphan_pid = Pid::from_raw(i32::from_le_bytes(buf));

						Ok(Handle {
							pid: orphan_pid,
							working_path,
							comms,
						})
					}
					WaitStatus::Exited(_, status) => {
						bail!("Child process exited with status {}", status)
					}
					_ => bail!("Unexpected wait status for child process"),
				}
			}
			ForkResult::Child => {
				// Child process
				match unsafe { fork() } {
					Result::Ok(ForkResult::Parent { child }) => {
						// Write the second child's PID to the pipe
						let orphan_pid_bytes = child.as_raw().to_le_bytes();
						write(pipe_write, &orphan_pid_bytes)?;

						// Exit the intermediate child
						std::process::exit(0);
					}
					Result::Ok(ForkResult::Child) => {
						// Disassociate from the parent by creating a new session
						setsid().context("setsid failed")?;

						// Exit immediately on fail in order to not leak process
						let err = std::process::Command::new(&runner_binary_path)
							.args(&runner_args)
							.envs(env.iter().cloned())
							.stdin(Stdio::null())
							.stdout(Stdio::null())
							.stderr(Stdio::null())
							.exec();
						eprintln!("exec failed: {err:?}");
						std::process::exit(1);
					}
					Err(err) => {
						// Exit immediately in order to not leak child process.
						eprintln!("process second fork failed: {err:?}");
						std::process::exit(1);
					}
				}
			}
		}
	}

	pub async fn observe(&self) -> Result<Option<i32>> {
		let exit_code_path = self.working_path.join("exit-code");
		let proc_path = Path::new("/proc").join(self.pid.to_string());

		let mut futs = FuturesUnordered::new();

		// Watch for exit code file being written
		futs.push(
			async {
				utils::wait_for_write(&exit_code_path).await?;

				anyhow::Ok(ObservationState::Exited)
			}
			.boxed(),
		);

		// Polling interval to check that the pid still exists
		futs.push(
			async {
				tokio::time::sleep(PID_POLL_INTERVAL).await;

				if fs::metadata(&proc_path).await.is_ok() {
					anyhow::Ok(ObservationState::Running)
				} else {
					anyhow::Ok(ObservationState::Dead)
				}
			}
			.boxed(),
		);

		let state = loop {
			// Get next complete future
			if let Some(state) = futs.next().await {
				let state = state?;

				// If still running, add poll future back to list
				if let ObservationState::Running = state {
					futs.push(
						async {
							tokio::time::sleep(PID_POLL_INTERVAL).await;

							if fs::metadata(&proc_path).await.is_ok() {
								Ok(ObservationState::Running)
							} else {
								Ok(ObservationState::Dead)
							}
						}
						.boxed(),
					);
				} else {
					break state;
				}
			} else {
				bail!("observation failed, developer error");
			}
		};

		let exit_code = if let ObservationState::Exited = state {
			use std::result::Result::Ok;
			match fs::read_to_string(&exit_code_path).await {
				Ok(contents) => match contents.trim().parse::<i32>() {
					Ok(x) => Some(x),
					Err(err) => {
						tracing::error!(pid=?self.pid, ?err, "failed to parse exit code file");

						None
					}
				},
				Err(err) => {
					tracing::error!(pid=?self.pid, ?err, "failed to read exit code file");

					None
				}
			}
		} else {
			tracing::warn!(pid=?self.pid, "process died before exit code file was written");

			None
		};

		tracing::info!(pid=?self.pid, ?exit_code, "exited");

		Ok(exit_code)
	}

	pub fn signal(&self, signal: Signal) -> Result<()> {
		// https://pubs.opengroup.org/onlinepubs/9699919799/functions/kill.html
		if (signal as i32) < 1 {
			bail!("signals < 1 not allowed");
		}

		match kill(self.pid, signal) {
			Ok(_) => {}
			Err(Errno::ESRCH) => {
				tracing::warn!(pid=?self.pid, "pid not found for signalling")
			}
			Err(err) => return Err(err.into()),
		}

		Ok(())
	}
}

impl Handle {
	pub fn pid(&self) -> &Pid {
		&self.pid
	}

	pub fn has_socket(&self) -> bool {
		matches!(self.comms, Comms::Socket(_))
	}
}

#[derive(Clone)]
pub enum Comms {
	Basic,
	Socket(Arc<Mutex<Option<SplitSink<WebSocketStream<TcpStream>, Message>>>>),
}

impl Comms {
	pub fn socket() -> Self {
		Comms::Socket(Arc::new(Mutex::new(None)))
	}
}
