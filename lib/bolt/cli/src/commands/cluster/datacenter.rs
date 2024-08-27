use anyhow::*;
use bolt_core::context::ProjectContext;
use clap::{Parser, ValueEnum};
use rivet_api::{apis::*, models};

#[derive(ValueEnum, Clone)]
pub enum DatacenterProvider {
	Linode,
}

impl From<DatacenterProvider> for models::AdminClustersProvider {
	fn from(provider: DatacenterProvider) -> Self {
		match provider {
			DatacenterProvider::Linode => models::AdminClustersProvider::Linode,
		}
	}
}

#[derive(ValueEnum, Clone)]
pub enum DatacenterBuildDeliveryMethod {
	TrafficServer,
	S3Direct,
}

impl From<DatacenterBuildDeliveryMethod> for models::AdminClustersBuildDeliveryMethod {
	fn from(method: DatacenterBuildDeliveryMethod) -> Self {
		match method {
			DatacenterBuildDeliveryMethod::TrafficServer => {
				models::AdminClustersBuildDeliveryMethod::TrafficServer
			}
			DatacenterBuildDeliveryMethod::S3Direct => {
				models::AdminClustersBuildDeliveryMethod::S3Direct
			}
		}
	}
}

#[derive(ValueEnum, Clone)]
pub enum DatacenterPoolType {
	Job,
	Gg,
	Ats,
}

impl From<DatacenterPoolType> for models::AdminClustersPoolType {
	fn from(pool_type: DatacenterPoolType) -> Self {
		match pool_type {
			DatacenterPoolType::Job => models::AdminClustersPoolType::Job,
			DatacenterPoolType::Gg => models::AdminClustersPoolType::Gg,
			DatacenterPoolType::Ats => models::AdminClustersPoolType::Ats,
		}
	}
}

#[derive(Parser)]
pub enum SubCommand {
	/// Creates a new datacenter
	Create {
		/// The name id of the cluster
		#[clap(index = 1)]
		cluster: String,
		/// The name id of the datacenter
		#[clap(index = 2)]
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
		/// Whether or not prebakes are enabled
		#[clap(long)]
		prebakes_enabled: bool,
	},
	/// Lists all datacenters of a cluster
	List {
		/// The name id of the cluster
		#[clap(index = 1)]
		cluster: String,
	},
	/// Update a datacenter's pools
	Update {
		/// The name id of the cluster
		#[clap(index = 1)]
		cluster: String,
		/// The name id of the datacenter
		#[clap(index = 2)]
		name_id: String,
		/// The pool type
		#[clap(index = 3)]
		pool: DatacenterPoolType,
		/// The hardware types
		#[clap(long)]
		hardware: Vec<String>,
		/// The desired count
		#[clap(long)]
		desired_count: Option<i32>,
		/// The min count
		#[clap(long)]
		min_count: Option<i32>,
		/// The max count
		#[clap(long)]
		max_count: Option<i32>,
		/// The drain timeout
		#[clap(long)]
		drain_timeout: Option<i64>,
		/// Whether or not prebakes are enabled
		#[clap(long)]
		prebakes_enabled: Option<bool>,
	},
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
				prebakes_enabled,
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
					models::AdminClustersCreateDatacenterRequest {
						name_id,
						display_name,
						provider: provider.into(),
						provider_datacenter_id,
						build_delivery_method: build_delivery_method.into(),
						prebakes_enabled,
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

				rivet_term::status::success("Datacenters", datacenters.len().to_string());
				if !datacenters.is_empty() {
					render::datacenters(datacenters);
				}
			}
			Self::Update {
				cluster: cluster_name_id,
				name_id,
				pool,
				hardware,
				desired_count,
				min_count,
				max_count,
				drain_timeout,
				prebakes_enabled,
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

				admin_clusters_datacenters_api::admin_clusters_datacenters_update(
					&ctx.openapi_config_cloud().await?,
					&cluster.cluster_id.to_string(),
					&datacenter.datacenter_id.to_string(),
					models::AdminClustersUpdateDatacenterRequest {
						pools: vec![models::AdminClustersPoolUpdate {
							desired_count,
							drain_timeout,
							hardware: hardware
								.iter()
								.map(|hardware| models::AdminClustersHardware {
									provider_hardware: hardware.clone(),
								})
								.collect(),
							min_count,
							max_count,
							pool_type: pool.into(),
						}],
						prebakes_enabled,
					},
				)
				.await?;

				rivet_term::status::success("Datacenter updated", "");
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
	struct DcTableRow {
		#[tabled(display_with = "display_option")]
		pub name_id: Option<String>,
		#[tabled(display_with = "display_option")]
		pub datacenter_id: Option<Uuid>,
		#[tabled(display_with = "display_provider")]
		pub provider: Option<models::AdminClustersProvider>,
		#[tabled(inline)]
		pub pool: PoolTableRow,
		#[tabled(display_with = "display_option")]
		pub prebakes_enabled: Option<bool>,
	}

	#[derive(Tabled, Default)]
	struct PoolTableRow {
		#[tabled(display_with = "display_pool_type")]
		pub pool_type: Option<models::AdminClustersPoolType>,
		#[tabled(display_with = "display_option")]
		pub min_count: Option<i32>,
		#[tabled(display_with = "display_option")]
		pub desired_count: Option<i32>,
		#[tabled(display_with = "display_option")]
		pub max_count: Option<i32>,
	}

	pub fn datacenters(mut datacenters: Vec<models::AdminClustersDatacenter>) {
		let rows = datacenters.iter_mut().flat_map(|d| {
			d.pools.sort_by_key(|pool| pool.pool_type);

			std::iter::once(DcTableRow {
				name_id: Some(d.name_id.clone()),
				datacenter_id: Some(d.datacenter_id),
				provider: Some(d.provider),
				prebakes_enabled: Some(d.prebakes_enabled),
				..Default::default()
			})
			.chain(d.pools.iter().cloned().map(|pool| DcTableRow {
				pool: PoolTableRow {
					pool_type: Some(pool.pool_type),
					min_count: Some(pool.min_count),
					desired_count: Some(pool.desired_count),
					max_count: Some(pool.max_count),
				},
				..Default::default()
			}))
		});

		rivet_term::format::table(rows);
	}

	fn display_provider(item: &Option<models::AdminClustersProvider>) -> String {
		match item {
			Some(models::AdminClustersProvider::Linode) => "Linode".to_string(),
			None => String::new(),
		}
	}

	fn display_pool_type(item: &Option<models::AdminClustersPoolType>) -> String {
		match item {
			Some(models::AdminClustersPoolType::Job) => "Job".to_string(),
			Some(models::AdminClustersPoolType::Gg) => "GG".to_string(),
			Some(models::AdminClustersPoolType::Ats) => "ATS".to_string(),
			None => String::new(),
		}
	}
}
