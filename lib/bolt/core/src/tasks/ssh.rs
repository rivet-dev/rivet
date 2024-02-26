use anyhow::*;
use duct::cmd;
use futures_util::StreamExt;
use std::{io::Write, os::unix::fs::PermissionsExt};
use std::{path::Path, sync::Arc};
use tokio::task::block_in_place;

use crate::{context::ProjectContext, dep::terraform};

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

pub async fn name(ctx: &ProjectContext, name: &str, command: Option<&str>) -> Result<()> {
	let tf_pools = terraform::output::read_pools(ctx).await;
	let server = tf_pools
		.servers
		.get(name)
		.context("failed to find server with name")?;

	// TODO: Choose correct SSH key
	let ssh_key = TempSshKey::new(&ctx, "server").await?;
	ip(ctx, &server.public_ipv4, &ssh_key, command).await?;

	Ok(())
}

pub async fn pool(
	ctx: &ProjectContext,
	pool: &str,
	region: Option<&str>,
	command: Option<&str>,
) -> Result<()> {
	// Choose IP
	let tf_pools = terraform::output::read_pools(&ctx).await;
	let server = tf_pools
		.servers
		.value
		.into_iter()
		.map(|x| x.1)
		.find(|x| x.pool_id == pool && region.map(|r| &x.region_id == r).unwrap_or(true))
		.expect("failed to find server pool");

	let ssh_key = TempSshKey::new(&ctx, "server").await?;
	ip(ctx, &server.public_ipv4, &ssh_key, command).await?;

	Ok(())
}

pub async fn pool_all(
	ctx: &ProjectContext,
	pool: &str,
	region: Option<&str>,
	command: &str,
) -> Result<()> {
	let ssh_key = Arc::new(TempSshKey::new(&ctx, "server").await?);

	let tf_pools = terraform::output::read_pools(&ctx).await;

	futures_util::stream::iter(
		tf_pools
			.servers
			.value
			.into_iter()
			.filter(|x| x.1.pool_id == pool && region.map(|r| &x.1.region_id == r).unwrap_or(true)),
	)
	.map(|(name, server)| {
		let ctx = ctx.clone();
		let ssh_key = ssh_key.clone();
		async move {
			let res = ip(&ctx, &server.public_ipv4, &ssh_key, Some(command)).await;
			println!("{name}: {res:?}");
		}
	})
	.buffer_unordered(32)
	.collect::<Vec<_>>()
	.await;

	Ok(())
}
