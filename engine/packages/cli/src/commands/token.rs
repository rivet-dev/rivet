use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

use crate::{
	DEFAULT_NAMESPACE,
	cloud::{create_token, ensure_namespace, get_namespace},
	target::{CloudArgs, ProjectTarget},
};

#[derive(Parser)]
pub struct Opts {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Create a namespace-scoped token.
	Create(CreateOpts),
}

impl Opts {
	pub async fn execute(self) -> Result<()> {
		match self.command {
			Commands::Create(opts) => opts.execute().await,
		}
	}
}

#[derive(Clone, Copy, ValueEnum)]
enum TokenKind {
	/// Server-side runner/runtime token, prefixed `sk_`.
	Secret,
	/// Client-safe publishable token, prefixed `pk_`.
	Public,
	/// Connection token, prefixed `ck_`. A strict subset of a publishable
	/// token: resolves actors and opens connections, but cannot manage them.
	Connection,
}

impl TokenKind {
	/// The Cloud API path segment under `.../tokens/` for this kind.
	fn endpoint(self) -> &'static str {
		match self {
			TokenKind::Secret => "secret",
			TokenKind::Public => "publishable",
			TokenKind::Connection => "connection",
		}
	}
}

#[derive(Parser)]
pub struct CreateOpts {
	#[command(flatten)]
	cloud: CloudArgs,
	/// Namespace to mint the token for.
	#[arg(long, default_value = DEFAULT_NAMESPACE)]
	namespace: String,
	/// Kind of token to create.
	#[arg(long, value_enum)]
	kind: TokenKind,
	/// Create the namespace if it does not already exist. By default the
	/// namespace must exist.
	#[arg(long)]
	create_namespace: bool,
}

impl CreateOpts {
	pub async fn execute(self) -> Result<()> {
		let (
			cloud,
			_,
			ProjectTarget {
				org: organization,
				project,
			},
		) = self.cloud.connect_project().await?;

		let namespace = if self.create_namespace {
			ensure_namespace(&cloud, &project, &organization, &self.namespace).await?
		} else {
			get_namespace(&cloud, &project, &organization, &self.namespace).await?
		};

		let token = create_token(
			&cloud,
			&project,
			&organization,
			&namespace.name,
			self.kind.endpoint(),
		)
		.await?;

		// The token is the command's result; print only it to stdout so scripts
		// can capture it directly. Everything else logs to stderr via tracing.
		println!("{token}");

		Ok(())
	}
}
