use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};

use crate::{
	cloud::v2,
	commands::deploy::{
		byoc::{self, ByocPrepare},
		cluster::{self, ClusterUpload},
	},
	target::{CloudArgs, ClusterTarget},
};

#[derive(Parser)]
pub struct Opts {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Build and upload a cluster build without rolling it out, and print its
	/// id. Roll it out later with `rivet deploy --build <id>`.
	Prepare(PrepareOpts),
}

#[derive(Parser)]
pub struct PrepareOpts {
	#[command(flatten)]
	cloud: CloudArgs,
	/// Dockerfile to build.
	#[arg(long, default_value = "Dockerfile")]
	dockerfile: PathBuf,
	/// Docker build context.
	#[arg(long, default_value = ".")]
	build_context: PathBuf,
	/// Image name recorded on the build. Defaults to the cluster name.
	#[arg(long)]
	image: Option<String>,
	/// Image tag recorded on the build. Defaults to the current git short SHA,
	/// or a timestamp outside git.
	#[arg(long)]
	tag: Option<String>,
}

impl Opts {
	pub async fn execute(self) -> Result<()> {
		match self.command {
			Commands::Prepare(opts) => opts.execute().await,
		}
	}
}

impl PrepareOpts {
	async fn execute(self) -> Result<()> {
		if !self.dockerfile.exists() {
			bail!("Dockerfile not found: {}", self.dockerfile.display());
		}

		let (cloud, ClusterTarget { cluster, .. }) = self.cloud.connect_cluster().await?;
		if v2::is_byoc_cluster(&cloud, &cluster).await? {
			return byoc::upload(
				&cloud,
				ByocPrepare {
					cluster,
					dockerfile: self.dockerfile,
					build_context: self.build_context,
					image: self.image,
					tag: self.tag,
				},
			)
			.await;
		}
		cluster::upload(
			&cloud,
			ClusterUpload {
				cluster,
				dockerfile: self.dockerfile,
				build_context: self.build_context,
				image: self.image,
				tag: self.tag,
			},
		)
		.await
	}
}
