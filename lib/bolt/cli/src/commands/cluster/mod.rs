use anyhow::*;
use bolt_core::context::ProjectContext;
use clap::{Parser, ValueEnum};
use rivet_api::{apis::*, models};
use uuid::Uuid;

mod datacenter;
mod server;

#[derive(ValueEnum, Clone)]
pub enum Provider {
	Linode,
}

impl From<Provider> for models::AdminClustersProvider {
	fn from(provider: Provider) -> Self {
		match provider {
			Provider::Linode => models::AdminClustersProvider::Linode,
		}
	}
}

#[derive(ValueEnum, Clone)]
pub enum BuildDeliveryMethod {
	TrafficServer,
	S3Direct,
}

impl From<BuildDeliveryMethod> for models::AdminClustersBuildDeliveryMethod {
	fn from(method: BuildDeliveryMethod) -> Self {
		match method {
			BuildDeliveryMethod::TrafficServer => {
				models::AdminClustersBuildDeliveryMethod::TrafficServer
			}
			BuildDeliveryMethod::S3Direct => models::AdminClustersBuildDeliveryMethod::S3Direct,
		}
	}
}

#[derive(ValueEnum, Clone)]
pub enum PoolType {
	Job,
	Gg,
	Ats,
	Pegboard,
	Pb,
}

impl From<PoolType> for models::AdminClustersPoolType {
	fn from(pool_type: PoolType) -> Self {
		match pool_type {
			PoolType::Job => models::AdminClustersPoolType::Job,
			PoolType::Gg => models::AdminClustersPoolType::Gg,
			PoolType::Ats => models::AdminClustersPoolType::Ats,
			PoolType::Pegboard | PoolType::Pb => models::AdminClustersPoolType::Pegboard,
		}
	}
}

#[derive(Parser)]
pub enum SubCommand {
	/// Creates a new cluster
	Create {
		/// The name of the cluster
		#[clap(long, short = 'c')]
		name_id: String,
		/// The ID of the owner team
		#[clap(long)]
		owner_team_id: String,
	},
	/// Lists all clusters
	List,
	/// Datacenter handler
	#[clap(alias = "dc")]
	Datacenter {
		#[clap(subcommand)]
		command: datacenter::SubCommand,
	},
	/// Server handler
	#[clap(alias = "s")]
	Server {
		#[clap(subcommand)]
		command: server::SubCommand,
	},
}

impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		match self {
			Self::Create {
				name_id,
				owner_team_id,
			} => {
				ensure!(
					ctx.ns().rivet.provisioning.is_some(),
					"Provisioning is not enabled on this cluster"
				);

				admin_clusters_api::admin_clusters_create(
					&ctx.openapi_config_cloud().await?,
					models::AdminClustersCreateClusterRequest {
						name_id,
						owner_team_id: Some(Uuid::parse_str(&owner_team_id).unwrap()),
					},
				)
				.await?;

				rivet_term::status::success("Cluster created", "");
			}
			Self::List => {
				let clusters =
					admin_clusters_api::admin_clusters_list(&ctx.openapi_config_cloud().await?)
						.await
						.unwrap()
						.clusters;

				rivet_term::status::success("Clusters", clusters.len().to_string());

				if !clusters.is_empty() {
					render::clusters(clusters);
				}
			}
			Self::Datacenter { command } => {
				command.execute(ctx).await?;
			}
			Self::Server { command } => {
				command.execute(ctx).await?;
			}
		}

		Ok(())
	}
}

pub fn unwrap_cluster_name_id(
	ctx: &ProjectContext,
	cluster_name_id: Option<String>,
) -> Result<String> {
	if let Some(cluster_name_id) = cluster_name_id {
		Ok(cluster_name_id)
	} else {
		Ok(ctx
			.ns()
			.rivet
			.provisioning
			.as_ref()
			.context("no `rivet.provisioning` in ns config")?
			.cluster
			.as_ref()
			.context("no `rivet.provisioning.cluster` in ns config")?
			.name_id
			.clone())
	}
}

mod render {
	use rivet_api::models;
	use tabled::Tabled;
	use uuid::Uuid;

	#[derive(Tabled)]
	pub struct ClusterTableRow {
		pub name_id: String,
		pub cluster_id: Uuid,
		#[tabled(display_with = "display_option")]
		pub owner_team_id: Option<Uuid>,
	}

	pub fn clusters(clusters: Vec<models::AdminClustersCluster>) {
		let rows = clusters.iter().map(|c| ClusterTableRow {
			name_id: c.name_id.clone(),
			cluster_id: c.cluster_id,
			owner_team_id: c.owner_team_id,
		});

		rivet_term::format::table(rows);
	}

	pub(crate) fn display_option<T: std::fmt::Display>(item: &Option<T>) -> String {
		match item {
			Some(s) => s.to_string(),
			None => String::new(),
		}
	}
}
