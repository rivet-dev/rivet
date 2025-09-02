use std::{os::fd::AsFd, path::PathBuf};

use anyhow::*;
use chrono::{Datelike, Duration, TimeDelta, TimeZone, Utc};
use tokio::fs;

pub struct Logs {
	path: PathBuf,
	retention: Duration,

	last_rotation: chrono::DateTime<Utc>,
	next_rotation: chrono::DateTime<Utc>,
}

impl Logs {
	pub fn new(path: PathBuf, retention: std::time::Duration) -> Self {
		Logs {
			path,
			retention: chrono::Duration::from_std(retention).expect("invalid retention duration"),

			last_rotation: Utc.timestamp_opt(0, 0).unwrap(),
			next_rotation: Utc.timestamp_opt(0, 0).unwrap(),
		}
	}
}

impl Logs {
	pub async fn start(mut self) -> Result<tokio::task::JoinHandle<()>> {
		// Create logs dir if it does not exist
		fs::create_dir_all(&self.path).await?;

		self.rotate().await?;

		Ok(tokio::spawn(self.run()))
	}

	async fn run(mut self) {
		loop {
			let now = Utc::now();

			// Sleep for a long duration until close to the day transition
			if self.next_rotation - now > Duration::seconds(5) {
				tokio::time::sleep(
					(self.next_rotation - now - Duration::seconds(5))
						.max(TimeDelta::default())
						.to_std()
						.expect("bad duration"),
				)
				.await;
			} else {
				// Rotate on the day (ordinal)
				if now.ordinal() != self.last_rotation.ordinal() {
					if let Err(err) = self.rotate().await {
						tracing::error!(?err, "failed logs rotation");
					}
				} else {
					// Sleep in short steps
					tokio::time::sleep(std::time::Duration::from_millis(250)).await;
				}
			}
		}
	}

	async fn rotate(&mut self) -> Result<()> {
		self.last_rotation = Utc::now();
		self.next_rotation = Utc.from_utc_datetime(
			&(self
				.last_rotation
				.date_naive()
				.and_hms_opt(0, 0, 0)
				.context("invalid date")?
				+ Duration::days(1)),
		);

		let file_name = format!("log-{}", self.last_rotation.format("%m-%d-%y"));
		let path = self.path.join(file_name);

		tracing::info!("Redirecting all logs to {}", path.display());
		let log_file = fs::OpenOptions::new()
			.write(true)
			.create(true)
			.append(true)
			.open(path)
			.await?;
		let log_fd = log_file.as_fd();

		nix::unistd::dup2_stdout(log_fd)?;
		nix::unistd::dup2_stderr(log_fd)?;

		self.prune().await
	}

	/// Remove files from `self.path` that are older than `self.retention`.
	async fn prune(&self) -> Result<()> {
		let mut entries = fs::read_dir(&self.path).await?;
		let mut pruned = 0;

		while let Some(entry) = entries.next_entry().await? {
			let metadata = entry.metadata().await?;
			let modified = chrono::DateTime::<Utc>::from(metadata.modified()?);

			if modified < Utc::now() - self.retention {
				pruned += 1;
				fs::remove_file(entry.path()).await?;
			}
		}

		if pruned != 0 {
			tracing::debug!("pruned {pruned} logs files");
		}

		Ok(())
	}
}

impl Logs {
	pub fn start_sync(mut self) -> Result<std::thread::JoinHandle<()>> {
		// Create logs dir if it does not exist
		std::fs::create_dir_all(&self.path)?;

		self.rotate_sync()?;

		Ok(std::thread::spawn(|| self.run_sync()))
	}

	fn run_sync(mut self) {
		loop {
			let now = Utc::now();

			// Sleep for a long duration until close to the transition
			if self.next_rotation - now > Duration::seconds(5) {
				std::thread::sleep(
					(self.next_rotation - now - Duration::seconds(5))
						.to_std()
						.expect("bad duration"),
				);
			} else {
				// Rotate on the day (ordinal)
				if now.ordinal() != self.last_rotation.ordinal() {
					if let Err(err) = self.rotate_sync() {
						tracing::error!(?err, "failed logs rotation");
					}
				} else {
					// Sleep in short steps
					std::thread::sleep(std::time::Duration::from_millis(250));
				}
			}
		}
	}

	fn rotate_sync(&mut self) -> Result<()> {
		self.last_rotation = Utc::now();
		self.next_rotation = Utc.from_utc_datetime(
			&(self
				.last_rotation
				.date_naive()
				.and_hms_opt(0, 0, 0)
				.context("invalid date")?
				+ Duration::days(1)),
		);

		let file_name = format!("log-{}", self.last_rotation.format("%m-%d-%y"));
		let path = self.path.join(file_name);

		tracing::info!("Redirecting all logs to {}", path.display());
		let log_file = std::fs::OpenOptions::new()
			.write(true)
			.create(true)
			.append(true)
			.open(path)?;
		let log_fd = log_file.as_fd();

		nix::unistd::dup2_stdout(log_fd)?;
		nix::unistd::dup2_stderr(log_fd)?;

		self.prune_sync()
	}

	/// Remove files from `self.path` that are older than `self.retention`.
	fn prune_sync(&self) -> Result<()> {
		let mut entries = std::fs::read_dir(&self.path)?;
		let mut pruned = 0;

		while let Some(entry) = entries.next() {
			let entry = entry?;
			let metadata = entry.metadata()?;
			let modified = chrono::DateTime::<Utc>::from(metadata.modified()?);

			if modified < Utc::now() - self.retention {
				pruned += 1;
				std::fs::remove_file(entry.path())?;
			}
		}

		if pruned != 0 {
			tracing::debug!("pruned {pruned} logs files");
		}

		Ok(())
	}
}
