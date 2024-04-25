use anyhow::*;
use bolt_core::context::ProjectContext;
use clap::Parser;
use rivet_api::{apis::*, models};
use tabled::Tabled;
use uuid::Uuid;

mod datacenter;

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
	Datacenter {
		#[clap(subcommand)]
		command: datacenter::SubCommand,
	},
}

#[derive(Tabled)]
struct ClusterTableRow {
	name_id: String,
	cluster_id: Uuid,
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
					models::AdminClustersCreateRequest {
						name_id: name_id,
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

				rivet_term::status::success("Clusters", "");
				rivet_term::format::table(clusters.iter().map(|c| ClusterTableRow {
					name_id: c.name_id.clone(),
					cluster_id: c.cluster_id,
				}));
			}

			Self::Datacenter { command } => {
				command.execute(ctx).await?;
			}
		}

		Ok(())
	}
}
