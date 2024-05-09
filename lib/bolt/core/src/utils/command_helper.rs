use std::{
	fmt::Display,
	io::Write,
	process::{Command, Stdio},
};

use anyhow::*;
use async_trait::async_trait;
use tokio::task;

#[derive(thiserror::Error, Clone, Debug)]
enum CommandError {
	#[error("write input: {0}")]
	WriteInput(String),

	#[error("exec: {0}")]
	Exec(String),

	#[error("status: {0}")]
	Status(std::process::ExitStatus),
}

#[async_trait]
pub trait CommandHelper: Sized {
	async fn exec_with_err<'a, E>(
		mut self,
		stdin: Option<Vec<u8>>,
		err: E,
		quiet: bool,
		without_stdio: bool,
	) -> Result<()>
	where
		E: Display + Send + Sync + 'static;

	async fn exec_string_with_err<E>(mut self, err: E, quiet: bool) -> Result<String>
	where
		E: Display + Send + Sync + 'static;

	async fn exec_string_with_stderr(mut self, quiet: bool) -> Result<String>;

	async fn exec_with_stderr(mut self, quiet: bool) -> Result<()>;

	async fn exec_value_with_err<E, T>(mut self, err: E, quiet: bool) -> Result<T>
	where
		E: Display + Send + Sync + 'static,
		T: serde::de::DeserializeOwned + Send + Sync + 'static;

	async fn exec(mut self) -> Result<()> {
		self.exec_with_err(None, "Command failed", false, false)
			.await
	}

	async fn exec_quiet(mut self, quiet: bool, without_stdio: bool) -> Result<()> {
		self.exec_with_err(None, "Command failed", quiet, without_stdio)
			.await
	}

	async fn exec_stdin(mut self, stdin: Vec<u8>) -> Result<()> {
		self.exec_with_err(Some(stdin), "Command failed", false, false)
			.await
	}

	async fn exec_string(mut self) -> Result<String> {
		self.exec_string_with_err("Command failed", false).await
	}

	async fn exec_value<T>(mut self) -> Result<T>
	where
		T: serde::de::DeserializeOwned + Send + Sync + 'static,
	{
		self.exec_value_with_err::<_, T>("Command failed", false)
			.await
	}
}

#[async_trait]
impl CommandHelper for Command {
	async fn exec_with_err<'a, E>(
		mut self,
		stdin: Option<Vec<u8>>,
		err: E,
		quiet: bool,
		without_stdio: bool,
	) -> Result<()>
	where
		E: Display + Send + Sync + 'static,
	{
		if !quiet {
			eprintln!("  $ {:?}", self);
		}

		task::spawn_blocking(move || {
			// Silent output if needed
			let child = if without_stdio {
				self.stdout(Stdio::piped()).stderr(Stdio::piped())
			} else {
				&mut self
			};

			// Setup stdin
			let child = if let Some(stdin) = stdin {
				// Execute command
				let mut child = child
					.stdin(Stdio::piped())
					.spawn()
					.map_err(|err| CommandError::Exec(err.to_string()))?;

				// Write stdin
				child
					.stdin
					.as_mut()
					.context("stdin")?
					.write_all(stdin.as_slice())
					.map_err(|err| CommandError::WriteInput(err.to_string()))?;

				child
			} else {
				// Execute command
				child
					.stdin(Stdio::inherit())
					.spawn()
					.map_err(|err| CommandError::Exec(err.to_string()))?
			};

			// Wait for output
			let output = child
				.wait_with_output()
				.map_err(|err| CommandError::Exec(err.to_string()))?;

			// Validate success
			if !output.status.success() {
				if !quiet {
					eprintln!("{}", err);
				}
				return Err(CommandError::Status(output.status).into());
			}

			Ok(())
		})
		.await?
	}

	async fn exec_string_with_err<E>(mut self, err: E, quiet: bool) -> Result<String>
	where
		E: Display + Send + Sync + 'static,
	{
		if !quiet {
			eprintln!("  $ {:?}", self);
		}

		task::spawn_blocking(move || {
			// Execute command
			let output = self
				.output()
				.map_err(|err| CommandError::Exec(err.to_string()))?;

			// Validate success
			if !output.status.success() {
				if !quiet {
					eprintln!("{}", err);
				}
				return Err(CommandError::Status(output.status).into());
			}

			let res = String::from_utf8(output.stdout)?;

			Ok(res)
		})
		.await?
	}

	async fn exec_string_with_stderr(mut self, quiet: bool) -> Result<String> {
		if !quiet {
			eprintln!("  $ {:?}", self);
		}

		task::spawn_blocking(move || {
			// Execute command
			let output = self
				.output()
				.map_err(|err| CommandError::Exec(err.to_string()))?;

			// Validate success
			if !output.status.success() {
				eprintln!("{}", String::from_utf8(output.stderr)?);

				return Err(CommandError::Status(output.status).into());
			}

			let res = String::from_utf8(output.stdout)?;

			Ok(res)
		})
		.await?
	}

	async fn exec_with_stderr(mut self, quiet: bool) -> Result<()> {
		if !quiet {
			eprintln!("  $ {:?}", self);
		}

		task::spawn_blocking(move || {
			// Execute command
			let output = self
				.output()
				.map_err(|err| CommandError::Exec(err.to_string()))?;

			// Validate success
			if !output.status.success() {
				eprintln!("{}", String::from_utf8(output.stderr)?);

				return Err(CommandError::Status(output.status).into());
			}

			Ok(())
		})
		.await?
	}

	async fn exec_value_with_err<E, T>(mut self, err: E, quiet: bool) -> Result<T>
	where
		E: Display + Send + Sync + 'static,
		T: serde::de::DeserializeOwned + Send + Sync + 'static,
	{
		if !quiet {
			eprintln!("  $ {:?}", self);
		}

		task::spawn_blocking(move || {
			// Execute command
			let output = self
				.output()
				.map_err(|err| CommandError::Exec(err.to_string()))?;

			// Validate success
			if !output.status.success() {
				if !quiet {
					eprintln!("{}", err);
				}
				return Err(CommandError::Status(output.status).into());
			}

			// Parse output
			let res = serde_json::from_slice::<T>(&output.stdout[..])?;

			Ok(res)
		})
		.await?
	}
}
