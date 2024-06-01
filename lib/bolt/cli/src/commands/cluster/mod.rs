use anyhow::*;
use bolt_core::{context::ProjectContext, tasks::ssh};
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
	/// Taint servers in a cluster
	Taint {
		/// The name id of the cluster
		#[clap(index = 1)]
		cluster: String,
		#[clap(long)]
		server_id: Option<String>,
		#[clap(long, short = 'p')]
		pool: Option<String>,
		#[clap(long, short = 'd')]
		datacenter: Option<String>,
		#[clap(long)]
		ip: Option<String>,
	},
	/// Destroy servers in a cluster
	Destroy {
		/// The name id of the cluster
		#[clap(index = 1)]
		cluster: String,
		#[clap(long)]
		server_id: Option<String>,
		#[clap(long, short = 'p')]
		pool: Option<String>,
		#[clap(long, short = 'd')]
		datacenter: Option<String>,
		#[clap(long)]
		ip: Option<String>,
	},
	/// SSH in to a server in the cluster
	Ssh {
		/// The name id of the cluster
		#[clap(index = 1)]
		cluster: String,
		#[clap(long)]
		server_id: Option<String>,
		#[clap(long, short = 'p')]
		pool: Option<String>,
		#[clap(long, short = 'd')]
		datacenter: Option<String>,
		#[clap(long)]
		ip: Option<String>,

		#[clap(long, short = 'c')]
		command: Option<String>,
	},
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

				rivet_term::status::success("Clusters", "");
				rivet_term::format::table(clusters.iter().map(|c| ClusterTableRow {
					name_id: c.name_id.clone(),
					cluster_id: c.cluster_id,
				}));
			}

			Self::Taint {
				cluster: cluster_name_id,
				server_id,
				pool,
				datacenter,
				ip,
			} => {
				let cloud_config = ctx.openapi_config_cloud().await?;

				// Look up cluster
				let clusters = admin_clusters_api::admin_clusters_list(&cloud_config)
					.await?
					.clusters;
				let cluster = clusters.iter().find(|c| c.name_id == cluster_name_id);
				let cluster = match cluster {
					Some(c) => c,
					None => bail!("cluster with the name id {} not found", cluster_name_id),
				};

				// Taint servers
				let pool_type = pool
					.map(|p| match p.as_str() {
						"job" => Ok(models::AdminClustersPoolType::Job),
						"gg" => Ok(models::AdminClustersPoolType::Gg),
						"ats" => Ok(models::AdminClustersPoolType::Ats),
						_ => Err(anyhow!("invalid pool type")),
					})
					.transpose()?;
				admin_clusters_servers_api::admin_clusters_servers_taint(
					&cloud_config,
					&cluster.cluster_id.to_string(),
					server_id.as_deref(),
					datacenter.as_deref(),
					pool_type,
					ip.as_deref(),
				)
				.await?;
			}
			Self::Destroy {
				cluster: cluster_name_id,
				server_id,
				pool,
				datacenter,
				ip,
			} => {
				let cloud_config = ctx.openapi_config_cloud().await?;

				// Look up cluster
				let clusters = admin_clusters_api::admin_clusters_list(&cloud_config)
					.await?
					.clusters;
				let cluster = clusters.iter().find(|c| c.name_id == cluster_name_id);
				let cluster = match cluster {
					Some(c) => c,
					None => bail!("cluster with the name id {} not found", cluster_name_id),
				};

				// Destroy servers
				let pool_type = pool
					.map(|p| match p.as_str() {
						"job" => Ok(models::AdminClustersPoolType::Job),
						"gg" => Ok(models::AdminClustersPoolType::Gg),
						"ats" => Ok(models::AdminClustersPoolType::Ats),
						_ => Err(anyhow!("invalid pool type")),
					})
					.transpose()?;
				admin_clusters_servers_api::admin_clusters_servers_destroy(
					&cloud_config,
					&cluster.cluster_id.to_string(),
					server_id.as_deref(),
					datacenter.as_deref(),
					pool_type,
					ip.as_deref(),
				)
				.await?;
			}
			Self::Ssh {
				cluster: cluster_name_id,
				command,
				server_id,
				pool,
				datacenter,
				ip,
			} => {
				let cloud_config = ctx.openapi_config_cloud().await?;

				// Look up cluster
				let clusters = admin_clusters_api::admin_clusters_list(&cloud_config)
					.await?
					.clusters;
				let cluster = clusters.iter().find(|c| c.name_id == cluster_name_id);
				let cluster = match cluster {
					Some(c) => c,
					None => bail!("cluster with the name id {} not found", cluster_name_id),
				};

				// Look up server IPs
				let pool_type = pool
					.map(|p| match p.as_str() {
						"job" => Ok(models::AdminClustersPoolType::Job),
						"gg" => Ok(models::AdminClustersPoolType::Gg),
						"ats" => Ok(models::AdminClustersPoolType::Ats),
						_ => Err(anyhow!("invalid pool type")),
					})
					.transpose()?;
				let mut servers = admin_clusters_servers_api::admin_clusters_servers_list(
					&cloud_config,
					&cluster.cluster_id.to_string(),
					server_id.as_deref(),
					datacenter.as_deref(),
					pool_type,
					ip.as_deref(),
				)
				.await?;
				servers.servers.sort_by_key(|s| s.server_id);
				let server_ips = servers
					.servers
					.iter()
					.map(|x| x.public_ip.as_str())
					.collect::<Vec<_>>();

				// SSH in to servers
				if let Some(command) = command {
					ssh::ip_all(&ctx, &server_ips, &command).await?;
				} else {
					let ip = server_ips.first().context("no matching servers")?;
					ssh::ip(&ctx, ip, command.as_deref()).await?;
				}
			}
			Self::Datacenter { command } => {
				command.execute(ctx).await?;
			}
		}

		Ok(())
	}
}
