use anyhow::*;
use bolt_core::context::ProjectContext;
use clap::{Parser, ValueEnum};
use rivet_api::{apis::*, models};
use tabled::Tabled;
use uuid::Uuid;

#[derive(ValueEnum, Clone)]
pub enum DatacenterProvider {
	Linode,
}

impl From<DatacenterProvider> for models::AdminProvider {
	fn from(provider: DatacenterProvider) -> Self {
		match provider {
			DatacenterProvider::Linode => models::AdminProvider::Linode,
		}
	}
}

#[derive(ValueEnum, Clone)]
pub enum DatacenterBuildDeliveryMethod {
	TrafficServer,
	S3Direct,
}

impl From<DatacenterBuildDeliveryMethod> for models::AdminBuildDeliveryMethod {
	fn from(method: DatacenterBuildDeliveryMethod) -> Self {
		match method {
			DatacenterBuildDeliveryMethod::TrafficServer => {
				models::AdminBuildDeliveryMethod::TrafficServer
			}
			DatacenterBuildDeliveryMethod::S3Direct => models::AdminBuildDeliveryMethod::S3Direct,
		}
	}
}

#[derive(Parser)]
pub enum SubCommand {
	/// Creates a new datacenter
	Create {
		/// The name id of the cluster
		#[clap(long, short = 'c')]
		cluster: String,
		/// The name id of the datacenter
		#[clap(long, short = 'd')]
		name_id: String,
		/// The display name of the datacenter
		#[clap(long)]
		display_name: String,
		/// The provider of the datacenter
		#[clap(long)]
		provider: DatacenterProvider,
		/// The provider datacenter ID
		#[clap(long)]
		provider_datacenter_id: String,
		/// The build delivery method
		#[clap(long)]
		build_delivery_method: DatacenterBuildDeliveryMethod,
	},
	/// Lists all datacenters of a cluster
	List {
		/// The name id of the cluster
		#[clap(long, short = 'c')]
		cluster: String,
	},
	/// Taint a datacenter
	Taint {
		/// The name id of the cluster
		#[clap(long, short = 'c')]
		cluster: String,
		/// The name id of the datacenter
		#[clap(long, short = 'd')]
		name_id: String,
	},
}

#[derive(Tabled)]
struct DatacenterTableRow {
	name_id: String,
	datacenter_id: Uuid,
}

impl SubCommand {
	pub async fn execute(self, ctx: ProjectContext) -> Result<()> {
		match self {
			Self::Create {
				cluster: cluster_name_id,
				name_id,
				display_name,
				provider,
				provider_datacenter_id,
				build_delivery_method,
			} => {
				ensure!(
					ctx.ns().rivet.provisioning.is_some(),
					"Provisioning is not enabled on this cluster"
				);

				let clusters =
					admin_clusters_api::admin_clusters_list(&ctx.openapi_config_cloud().await?)
						.await?
						.clusters;

				let cluster = clusters.iter().find(|c| c.name_id == cluster_name_id);

				let cluster = match cluster {
					Some(cluster) => cluster,
					None => bail!("cluster with the name id {} not found", cluster_name_id),
				};

				admin_clusters_datacenters_api::admin_clusters_datacenters_create(
					&ctx.openapi_config_cloud().await?,
					&cluster.cluster_id.to_string(),
					models::AdminClustersDatacentersCreateRequest {
						name_id,
						display_name,
						provider: provider.into(),
						provider_datacenter_id,
						build_delivery_method: build_delivery_method.into(),
					},
				)
				.await?;

				rivet_term::status::success("Datacenter created", "");
			}
			Self::List {
				cluster: cluster_name_id,
			} => {
				let clusters =
					admin_clusters_api::admin_clusters_list(&ctx.openapi_config_cloud().await?)
						.await?
						.clusters;

				let cluster = clusters.iter().find(|c| c.name_id == cluster_name_id);

				let cluster = match cluster {
					Some(c) => c,
					None => bail!("cluster with the name id {} not found", cluster_name_id),
				};

				let datacenters = admin_clusters_datacenters_api::admin_clusters_datacenters_list(
					&ctx.openapi_config_cloud().await?,
					&cluster.cluster_id.to_string(),
				)
				.await?
				.datacenters;

				rivet_term::status::success("Datacenters", "");
				rivet_term::format::table(datacenters.iter().map(|d| DatacenterTableRow {
					name_id: d.name_id.clone(),
					datacenter_id: d.datacenter_id,
				}));
			}
			Self::Taint {
				cluster: cluster_name_id,
				name_id,
			} => {
				let clusters =
					admin_clusters_api::admin_clusters_list(&ctx.openapi_config_cloud().await?)
						.await?
						.clusters;
				let cluster = clusters.iter().find(|c| c.name_id == cluster_name_id);

				let cluster = match cluster {
					Some(c) => c,
					None => bail!("cluster with the name id {} not found", cluster_name_id),
				};

				let datacenters = admin_clusters_datacenters_api::admin_clusters_datacenters_list(
					&ctx.openapi_config_cloud().await?,
					&cluster.cluster_id.to_string(),
				)
				.await?
				.datacenters;

				let datacenter = datacenters.iter().find(|d| d.name_id == name_id);

				let datacenter = match datacenter {
					Some(d) => d,
					None => bail!("datacenter with the name id {} not found", name_id),
				};

				admin_clusters_datacenters_api::admin_clusters_datacenters_taint(
					&ctx.openapi_config_cloud().await?,
					&cluster.cluster_id.to_string(),
					&datacenter.datacenter_id.to_string(),
				)
				.await?;
			}
		}

		Ok(())
	}
}
