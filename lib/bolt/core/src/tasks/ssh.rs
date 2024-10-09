use std::{io::Write, os::unix::fs::PermissionsExt, path::Path, sync::Arc};

use anyhow::*;
use duct::cmd;
use futures_util::StreamExt;
use tokio::task::block_in_place;

use crate::context::ProjectContext;

pub struct TempSshKey {
	tempfile: tempfile::NamedTempFile,
}

impl TempSshKey {
	pub async fn new(ctx: &ProjectContext, name: &str) -> Result<Self> {
		let private_key_openssh = ctx
			.read_secret(&["ssh", name, "private_key_openssh"])
			.await?;

		// Write SSH key
		let tempfile = block_in_place(|| {
			let mut tempfile = tempfile::NamedTempFile::new()?;
			tempfile
				.as_file()
				.set_permissions(std::fs::Permissions::from_mode(0o600))?;
			tempfile
				.as_file_mut()
				.write_all(private_key_openssh.as_bytes())?;
			Ok(tempfile)
		})?;

		Ok(Self { tempfile })
	}

	pub fn path(&self) -> &Path {
		self.tempfile.path()
	}
}

async fn ip_inner(
	_ctx: &ProjectContext,
	ip: &str,
	ssh_key: &TempSshKey,
	command: Option<&str>,
) -> Result<()> {
	block_in_place(|| {
		if let Some(command) = command {
			cmd!(
				"ssh",
				"-o",
				"StrictHostKeyChecking=no",
				"-t",
				"-l",
				"root",
				"-i",
				ssh_key.path(),
				ip,
				command
			)
			.run()
		} else {
			cmd!(
				"ssh",
				"-o",
				"StrictHostKeyChecking=no",
				"-t",
				"-l",
				"root",
				"-i",
				ssh_key.path(),
				"-L",
				"9090:10.0.0.84:8080",
				ip,
			)
			.run()
		}
	})?;

	Ok(())
}

pub async fn ip(ctx: &ProjectContext, ip: &str, command: Option<&str>) -> Result<()> {
	let ssh_key = Arc::new(TempSshKey::new(ctx, "server").await?);
	ip_inner(ctx, ip, &ssh_key, command).await
}

pub async fn ip_all(ctx: &ProjectContext, server_ips: &[&str], command: &str) -> Result<()> {
	let ssh_key = Arc::new(TempSshKey::new(ctx, "server").await?);

	futures_util::stream::iter(server_ips)
		.map(|server_ip| {
			let ctx = ctx.clone();
			let ssh_key = ssh_key.clone();
			async move {
				let res = ip_inner(&ctx, &server_ip, &ssh_key, Some(command)).await;
				println!("{res:?}");
			}
		})
		.buffer_unordered(32)
		.collect::<Vec<_>>()
		.await;

	Ok(())
}
