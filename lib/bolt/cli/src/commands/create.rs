use anyhow::*;
use bolt_core::{context::ProjectContext, tasks};
use clap::Parser;

#[derive(Parser)]
pub struct CreateOpts {
	#[clap(subcommand)]
	command: SubCommand,

	#[clap(long)]
	create_pkg: bool,
	/// The additional root in which to create the service. Leave blank for top-most.
	#[clap(long)]
	root: Option<String>,
}

impl CreateOpts {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		let CreateOpts {
			command,
			create_pkg,
			root,
		} = self;

		command.execute(ctx, create_pkg, root).await
	}
}

#[derive(Parser)]
enum SubCommand {
	Api {
		#[clap(index = 1)]
		api_name: String,
	},
	Bucket {
		#[clap(index = 1)]
		pkg_name: String,
		#[clap(index = 2)]
		service_name: String,
	},
	#[clap(alias = "db")]
	Database {
		#[clap(index = 1)]
		pkg_name: String,
		#[clap(index = 2)]
		service_name: String,
	},
	#[clap(alias = "op")]
	Operation {
		#[clap(index = 1)]
		pkg_name: String,
		#[clap(index = 2)]
		service_name: String,
	},
	Standalone {
		#[clap(index = 1)]
		pkg_name: String,
		#[clap(index = 2)]
		service_name: String,
	},
	Worker {
		#[clap(index = 1)]
		pkg_name: String,
		#[clap(index = 2)]
		service_name: String,
	},
}

impl SubCommand {
	pub async fn execute(
		self,
		mut ctx: ProjectContext,
		create_pkg: bool,
		root: Option<String>,
	) -> Result<()> {
		let (pkg_name, template_type, service_name) = match self {
			Self::Api { api_name } => (
				String::new(),
				tasks::template::TemplateType::Api,
				api_name.to_string(),
			),
			Self::Bucket {
				pkg_name,
				service_name,
			} => (
				pkg_name.to_string(),
				tasks::template::TemplateType::Bucket,
				service_name.to_string(),
			),
			Self::Database {
				pkg_name,
				service_name,
			} => (
				pkg_name.to_string(),
				tasks::template::TemplateType::Database,
				service_name.to_string(),
			),
			Self::Operation {
				pkg_name,
				service_name,
			} => (
				pkg_name.to_string(),
				tasks::template::TemplateType::Operation,
				service_name.to_string(),
			),
			Self::Standalone {
				pkg_name,
				service_name,
			} => (
				pkg_name.to_string(),
				tasks::template::TemplateType::Standalone,
				service_name.to_string(),
			),
			Self::Worker {
				pkg_name,
				service_name,
			} => (
				pkg_name.to_string(),
				tasks::template::TemplateType::Worker,
				service_name.to_string(),
			),
		};

		assert!(
			!pkg_name.contains('_'),
			"package name should not contain underscores, use dashes"
		);

		assert!(
			!service_name.contains('_'),
			"service name should not contain underscores, use dashes"
		);

		tasks::template::generate(
			&mut ctx,
			tasks::template::TemplateOpts {
				root,
				create_pkg,
				pkg_name,
				template_type,
				service_name,
			},
		)
		.await?;

		Ok(())
	}
}
