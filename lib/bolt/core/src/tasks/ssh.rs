use anyhow::*;
use duct::cmd;
use std::path::Path;
use std::{io::Write, os::unix::fs::PermissionsExt};
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
	let ssh_key = TempSshKey::new(&ctx, "salt_minion").await?;
	ip(ctx, &server.public_ipv4, &ssh_key, command).await?;

	Ok(())
}

pub async fn pool(ctx: &ProjectContext, pool: &str, command: Option<&str>) -> Result<()> {
	let (key_name, server_ip) = match pool {
		"salt_master" => {
			let tf_master_cluster = terraform::output::read_master_cluster(&ctx).await;
			("salt_master", tf_master_cluster.salt_master_host.value)
		}
		"nebula_lighthouse" => {
			let tf_master_cluster = terraform::output::read_master_cluster(&ctx).await;
			(
				"nebula_lighthouse",
				tf_master_cluster.nebula_lighthouse_host.value,
			)
		}
		_ => {
			// Choose IP
			let tf_pools = terraform::output::read_pools(&ctx).await;
			let server = tf_pools
				.servers
				.value
				.into_iter()
				.map(|x| x.1)
				.find(|x| x.pool_id == pool)
				.expect("failed to find server pool");

			("salt_minion", server.public_ipv4)
		}
	};

	let ssh_key = TempSshKey::new(&ctx, key_name).await?;
	ip(ctx, &server_ip, &ssh_key, command).await?;

	Ok(())
}
