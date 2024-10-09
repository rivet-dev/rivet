use anyhow::*;
use bolt_core::{context::ProjectContext, tasks::ssh};
use clap::Parser;
use rivet_api::{apis::*, models};
use uuid::Uuid;

use super::PoolType;

#[derive(Parser)]
pub enum SubCommand {
	/// Lists all datacenters of a cluster
	List {
		/// The name id of the cluster
		#[clap(index = 1)]
		cluster: String,
		#[clap(long, short = 's')]
		server_id: Option<Uuid>,
		#[clap(long, short = 'p')]
		pool: Option<PoolType>,
		#[clap(long, short = 'd')]
		datacenter: Option<String>,
		#[clap(long)]
		ip: Option<String>,
	},
	/// Taint servers in a cluster
	Taint {
		/// The name id of the cluster
		#[clap(index = 1)]
		cluster: String,
		#[clap(long, short = 's')]
		server_id: Option<Uuid>,
		#[clap(long, short = 'p')]
		pool: Option<PoolType>,
		#[clap(long, short = 'd')]
		datacenter: Option<String>,
		#[clap(long)]
		ip: Option<String>,
		/// Confirms that you want to execute this command on all servers
		#[clap(long)]
		all: bool,
	},
	/// Destroy servers in a cluster
	Destroy {
		/// The name id of the cluster
		#[clap(index = 1)]
		cluster: String,
		#[clap(long, short = 's')]
		server_id: Option<Uuid>,
		#[clap(long, short = 'p', value_enum)]
		pool: Option<PoolType>,
		#[clap(long, short = 'd')]
		datacenter: Option<String>,
		#[clap(long)]
		ip: Option<String>,
		/// Confirms that you want to execute this command on all servers
		#[clap(long)]
		all: bool,
	},
	/// Lists lost servers in a cluster
	ListLost {
		/// The name id of the cluster
		#[clap(index = 1)]
		cluster: String,
		#[clap(long, short = 's')]
		server_id: Option<Uuid>,
		#[clap(long, short = 'p', value_enum)]
		pool: Option<PoolType>,
		#[clap(long, short = 'd')]
		datacenter: Option<String>,
		#[clap(long)]
		ip: Option<String>,
	},
	/// Prunes lost servers in a cluster. use `list-lost` to see servers first.
	Prune {
		/// The name id of the cluster
		#[clap(index = 1)]
		cluster: String,
		#[clap(long, short = 's')]
		server_id: Option<Uuid>,
		#[clap(long, short = 'p', value_enum)]
		pool: Option<PoolType>,
		#[clap(long, short = 'd')]
		datacenter: Option<String>,
		#[clap(long)]
		ip: Option<String>,
		/// Confirms that you want to execute this command on all lost servers
		#[clap(long)]
		all: bool,
	},
	/// SSH in to a server in the cluster
	Ssh {
		/// The name id of the cluster
		#[clap(index = 1)]
		cluster: String,
		#[clap(long, short = 's')]
		server_id: Option<Uuid>,
		#[clap(long, short = 'p', value_enum)]
		pool: Option<PoolType>,
		#[clap(long, short = 'd')]
		datacenter: Option<String>,
		#[clap(long)]
		ip: Option<String>,
		/// Confirms that you want to execute this command on all servers
		#[clap(long)]
		all: bool,

		#[clap(long, short = 'c')]
		command: Option<String>,
	},
}

// TODO: Move API calls and rendering to bolt core
impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		match self {
			Self::List {
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
				let server_id = server_id.map(|x| x.to_string());
				let servers = admin_clusters_servers_api::admin_clusters_servers_list(
					&cloud_config,
					&cluster.cluster_id.to_string(),
					server_id.as_deref(),
					datacenter.as_deref(),
					pool.map(Into::into),
					ip.as_deref(),
				)
				.await?
				.servers;

				rivet_term::status::success("Servers", servers.len().to_string());
				render::servers(servers);
			}
			Self::Taint {
				cluster: cluster_name_id,
				server_id,
				pool,
				datacenter,
				ip,
				all,
			} => {
				if server_id.is_none()
					&& pool.is_none()
					&& datacenter.is_none()
					&& ip.is_none() && !all
				{
					bail!("must use --all if command has no filters");
				}

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
				let server_id = server_id.map(|x| x.to_string());
				admin_clusters_servers_api::admin_clusters_servers_taint(
					&cloud_config,
					&cluster.cluster_id.to_string(),
					server_id.as_deref(),
					datacenter.as_deref(),
					pool.map(Into::into),
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
				all,
			} => {
				if server_id.is_none()
					&& pool.is_none()
					&& datacenter.is_none()
					&& ip.is_none() && !all
				{
					bail!("must use --all if command has no filters");
				}

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
				let server_id = server_id.map(|x| x.to_string());
				admin_clusters_servers_api::admin_clusters_servers_destroy(
					&cloud_config,
					&cluster.cluster_id.to_string(),
					server_id.as_deref(),
					datacenter.as_deref(),
					pool.map(Into::into),
					ip.as_deref(),
				)
				.await?;
			}
			Self::ListLost {
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

				let server_id = server_id.map(|x| x.to_string());
				let servers = admin_clusters_servers_api::admin_clusters_servers_list_lost(
					&cloud_config,
					&cluster.cluster_id.to_string(),
					server_id.as_deref(),
					datacenter.as_deref(),
					pool.map(Into::into),
					ip.as_deref(),
				)
				.await?
				.servers;

				rivet_term::status::success("Lost Servers", servers.len().to_string());
				if !servers.is_empty() {
					render::servers(servers);
				}
			}
			Self::Prune {
				cluster: cluster_name_id,
				server_id,
				pool,
				datacenter,
				ip,
				all,
			} => {
				if server_id.is_none()
					&& pool.is_none()
					&& datacenter.is_none()
					&& ip.is_none() && !all
				{
					bail!("must use --all if command has no filters");
				}

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

				// Prune servers
				let server_id = server_id.map(|x| x.to_string());
				admin_clusters_servers_api::admin_clusters_servers_prune(
					&cloud_config,
					&cluster.cluster_id.to_string(),
					server_id.as_deref(),
					datacenter.as_deref(),
					pool.map(Into::into),
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
				all,
			} => {
				if server_id.is_none()
					&& pool.is_none()
					&& datacenter.is_none()
					&& ip.is_none() && !all
				{
					bail!("must use --all if command has no filters");
				}

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
				let server_id = server_id.map(|x| x.to_string());
				let mut servers = admin_clusters_servers_api::admin_clusters_servers_list(
					&cloud_config,
					&cluster.cluster_id.to_string(),
					server_id.as_deref(),
					datacenter.as_deref(),
					pool.map(Into::into),
					ip.as_deref(),
				)
				.await?;
				servers.servers.sort_by_key(|s| s.server_id);
				let server_ips = servers
					.servers
					.iter()
					.filter_map(|x| x.public_ip.as_ref())
					.map(|x| x.as_str())
					.collect::<Vec<_>>();

				// SSH in to servers
				if let Some(command) = command {
					ssh::ip_all(&ctx, &server_ips, &command).await?;
				} else {
					let ip = server_ips.first().context("no matching servers")?;
					ssh::ip(&ctx, ip, command.as_deref()).await?;
				}
			}
		}

		Ok(())
	}
}

mod render {
	use rivet_api::models;
	use tabled::Tabled;
	use uuid::Uuid;

	use super::super::render::display_option;

	#[derive(Tabled, Default)]
	struct ServerTableRow {
		pub server_id: Uuid,
		pub datacenter_id: Uuid,
		#[tabled(display_with = "display_pool_type")]
		pub pool_type: models::AdminClustersPoolType,
		#[tabled(display_with = "display_option")]
		pub public_ip: Option<String>,
	}

	pub fn servers(mut servers: Vec<models::AdminClustersServer>) {
		servers.sort_by_key(|s| (s.datacenter_id, s.pool_type, s.public_ip.clone()));

		let rows = servers.iter().map(|s| ServerTableRow {
			server_id: s.server_id,
			datacenter_id: s.datacenter_id,
			pool_type: s.pool_type,
			public_ip: s.public_ip.clone(),
		});

		rivet_term::format::table(rows);
	}

	fn display_pool_type(item: &models::AdminClustersPoolType) -> String {
		match item {
			models::AdminClustersPoolType::Job => "Job".to_string(),
			models::AdminClustersPoolType::Gg => "GG".to_string(),
			models::AdminClustersPoolType::Ats => "ATS".to_string(),
			models::AdminClustersPoolType::Pegboard => "Pegboard".to_string(),
		}
	}
}
