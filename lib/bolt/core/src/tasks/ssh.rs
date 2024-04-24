use std::{io::Write, os::unix::fs::PermissionsExt};
use std::{path::Path, sync::Arc};

use anyhow::*;
use duct::cmd;
use futures_util::StreamExt;
use tokio::task::block_in_place;

use crate::{context::ProjectContext, tasks};

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

pub async fn ip(
	_ctx: &ProjectContext,
	ip: &str,
	ssh_key: &TempSshKey,
	command: Option<&str>,
) -> Result<()> {
	block_in_place(|| {
		if let Some(command) = command {
			cmd!("ssh", "-t", "-i", ssh_key.path(), ip, command).run()
		} else {
			cmd!("ssh", "-t", "-i", ssh_key.path(), ip).run()
		}
	})?;

	Ok(())
}

pub async fn id(ctx: &ProjectContext, server_id: &str, command: Option<&str>) -> Result<()> {
	let server_ips = tasks::api::get_cluster_server_ips(&ctx, Some(server_id), None).await?;
	let server_ip = server_ips
		.first()
		.context(format!("failed to find server with server id {server_id}"))?;

	// TODO: Choose correct SSH key
	let ssh_key = TempSshKey::new(&ctx, "server").await?;
	ip(ctx, &server_ip, &ssh_key, command).await?;

	Ok(())
}

pub async fn pool(ctx: &ProjectContext, pool: &str, command: Option<&str>) -> Result<()> {
	let server_ips = tasks::api::get_cluster_server_ips(&ctx, None, Some(pool)).await?;
	let server_ip = server_ips
		.first()
		.context(format!("failed to find server with pool {pool}"))?;

	let ssh_key = TempSshKey::new(&ctx, "server").await?;
	ip(ctx, &server_ip, &ssh_key, command).await?;

	Ok(())
}

pub async fn pool_all(ctx: &ProjectContext, pool: &str, command: &str) -> Result<()> {
	let server_ips = tasks::api::get_cluster_server_ips(&ctx, None, Some(pool)).await?;
	let ssh_key = Arc::new(TempSshKey::new(&ctx, "server").await?);

	futures_util::stream::iter(server_ips)
		.map(|server_ip| {
			let ctx = ctx.clone();
			let ssh_key = ssh_key.clone();
			async move {
				let res = ip(&ctx, &server_ip, &ssh_key, Some(command)).await;
				println!("{res:?}");
			}
		})
		.buffer_unordered(32)
		.collect::<Vec<_>>()
		.await;

	Ok(())
}
