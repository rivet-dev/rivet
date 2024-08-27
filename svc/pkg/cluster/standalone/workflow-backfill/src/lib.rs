use std::{
	convert::TryInto,
	net::{IpAddr, Ipv4Addr},
};

use chirp_workflow::prelude::*;
use rivet_operation::prelude::{proto::backend, Message};
use serde::Serialize;
use serde_json::json;

#[tracing::instrument(skip_all)]
pub async fn run_from_env() -> GlobalResult<()> {
	let pools = rivet_pools::from_env("cluster-workflow-backfill").await?;
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("cluster-workflow-backfill");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = StandaloneCtx::new(
		chirp_workflow::compat::db_from_pools(&pools).await?,
		rivet_connection::Connection::new(client, pools, cache),
		"cluster-workflow-backfill",
	)
	.await?;

	let crdb = ctx.crdb().await?;
	let mut tx = crdb.begin().await?;

	// Delete invalid dns record rows
	sql_execute!(
		[ctx, @tx &mut tx]
		"
		DELETE FROM db_cluster.servers_cloudflare
		WHERE dns_record_id IS NULL OR secondary_dns_record_id IS NULL
		",
	)
	.await?;

	let mut bctx = BackfillCtx::new();

	#[derive(sqlx::FromRow)]
	struct ClusterRow {
		cluster_id: Uuid,
		name_id: String,
		owner_team_id: Option<Uuid>,
	}

	let cluster_rows = sql_fetch_all!(
		[ctx, ClusterRow, @tx &mut tx]
		"
		SELECT
			cluster_id,
			name_id,
			owner_team_id
		FROM db_cluster.clusters
		",
	)
	.await?;

	for cluster in cluster_rows {
		bctx.workflow("cluster", |wf| {
			wf.tags(json!({
				"cluster_id": cluster.cluster_id,
			}))?;
			wf.input(cluster::workflows::cluster::Input {
				cluster_id: cluster.cluster_id,
				name_id: cluster.name_id.clone(),
				owner_team_id: cluster.owner_team_id,
			})?;
			wf.finalize();

			#[derive(Serialize, Hash)]
			struct InsertDbInput {
				cluster_id: Uuid,
				name_id: String,
				owner_team_id: Option<Uuid>,
			}

			wf.activity(
				"insert_db",
				InsertDbInput {
					cluster_id: cluster.cluster_id,
					name_id: cluster.name_id.clone(),
					owner_team_id: cluster.owner_team_id,
				},
				serde_json::Value::Null,
			)?;

			wf.message(
				"cluster_create_complete",
				json!({
					"cluster_id": cluster.cluster_id,
				}),
				cluster::workflows::cluster::CreateComplete {},
			)?;

			Ok(())
		})?;
	}

	#[derive(sqlx::FromRow, Clone)]
	struct DcRow {
		datacenter_id: Uuid,
		cluster_id: Uuid,
		name_id: String,
		display_name: String,
		provider: i64,
		provider_datacenter_id: String,
		provider_api_token: Option<String>,
		pools: Vec<u8>,
		build_delivery_method: i64,
	}
	#[derive(sqlx::FromRow)]
	struct DcTlsRow {
		datacenter_id: Uuid,
		gg_cert_pem: Option<String>,
		gg_private_key_pem: Option<String>,
		job_cert_pem: Option<String>,
		job_private_key_pem: Option<String>,
		expire_ts: i64,
	}

	let dc_rows = sql_fetch_all!(
		[ctx, DcRow, @tx &mut tx]
		"
		SELECT
			datacenter_id,
			cluster_id,
			name_id,
			display_name,
			provider_datacenter_id,
			provider_api_token,
			create_ts,
			provider,
			pools,
			build_delivery_method
		FROM db_cluster.datacenters
		",
	)
	.await?;
	let dc_tls_rows = sql_fetch_all!(
		[ctx, DcTlsRow, @tx &mut tx]
		"
		SELECT
			datacenter_id,
			gg_cert_pem,
			gg_private_key_pem,
			job_cert_pem,
			job_private_key_pem,
			expire_ts
		FROM db_cluster.datacenter_tls
		",
	)
	.await?;

	for dc in dc_rows.clone() {
		let tls = unwrap!(dc_tls_rows
			.iter()
			.find(|dc_tls| dc_tls.datacenter_id == dc.datacenter_id));

		bctx.workflow("cluster_datacenter", |wf| {
			wf.tags(json!({
				"datacenter_id": dc.datacenter_id,
			}))?;
			wf.input(json!({
				"cluster_id": dc.cluster_id,
				"datacenter_id": dc.datacenter_id,
				"name_id": dc.name_id.clone(),
				"display_name": dc.display_name.clone(),

				"provider": TryInto::<cluster::types::Provider>::try_into(dc.provider)?,
				"provider_datacenter_id": dc.provider_datacenter_id.clone(),
				"provider_api_token": dc.provider_api_token.clone(),

				"pools": ({
					let proto = backend::cluster::Pools::decode(dc.pools.as_slice())?.pools;

					proto
						.into_iter()
						.map(TryInto::<cluster::types::Pool>::try_into)
						.collect::<GlobalResult<Vec<_>>>()?
				}),

				"build_delivery_method": unwrap!(
					cluster::types::BuildDeliveryMethod::from_repr(dc.build_delivery_method.try_into()?)
				),
				"prebakes_enabled": false,
			}))?;
			wf.finalize();

			#[derive(Serialize, Hash)]
			struct InsertDbInput {
				cluster_id: Uuid,
				datacenter_id: Uuid,
				name_id: String,
				display_name: String,

				provider: cluster::types::Provider,
				provider_datacenter_id: String,
				provider_api_token: Option<String>,

				pools: Vec<cluster::types::Pool>,

				build_delivery_method: cluster::types::BuildDeliveryMethod,
				prebakes_enabled: bool,
			}

			wf.activity(
				"insert_db",
				InsertDbInput {
					cluster_id: dc.cluster_id,
					datacenter_id: dc.datacenter_id,
					name_id: dc.name_id.clone(),
					display_name: dc.display_name.clone(),

					provider: dc.provider.try_into()?,
					provider_datacenter_id: dc.provider_datacenter_id.clone(),
					provider_api_token: dc.provider_api_token.clone(),

					pools: {
						let proto = backend::cluster::Pools::decode(dc.pools.as_slice())?.pools;

						proto
							.into_iter()
							.map(TryInto::try_into)
							.collect::<GlobalResult<Vec<_>>>()?
					},

					build_delivery_method: unwrap!(cluster::types::BuildDeliveryMethod::from_repr(
						dc.build_delivery_method.try_into()?,
					)),
					prebakes_enabled: false,
				},
				serde_json::Value::Null,
			)?;

			// Tls issue
			if let (
				Some(gg_cert_pem),
				Some(gg_private_key_pem),
				Some(job_cert_pem),
				Some(job_private_key_pem),
			) = (
				&tls.gg_cert_pem,
				&tls.gg_private_key_pem,
				&tls.job_cert_pem,
				&tls.job_private_key_pem,
			) {
				wf.sub_workflow(|swf| {
					let base_zone_id = unwrap!(
						util::env::cloudflare::zone::main::id(),
						"dns not configured"
					);
					let job_zone_id =
						unwrap!(util::env::cloudflare::zone::job::id(), "dns not configured");
					let domain_main = unwrap!(util::env::domain_main(), "dns not enabled");
					let domain_job = unwrap!(util::env::domain_job(), "dns not enabled");

					#[derive(Serialize, Hash)]
					struct OrderInput {
						renew: bool,
						zone_id: String,
						common_name: String,
						subject_alternative_names: Vec<String>,
					}

					swf.activity(
						"order",
						OrderInput {
							renew: false,
							zone_id: base_zone_id.to_string(),
							common_name: domain_main.to_string(),
							subject_alternative_names: vec![format!(
								"*.{}.{domain_main}",
								dc.datacenter_id
							)],
						},
						json!({
							"cert": gg_cert_pem,
							"private_key": gg_private_key_pem,
							"expire_ts": tls.expire_ts,
						}),
					)?;

					swf.activity(
						"order",
						OrderInput {
							renew: false,
							zone_id: job_zone_id.to_string(),
							common_name: domain_job.to_string(),
							subject_alternative_names: vec![
								format!("*.lobby.{}.{domain_job}", dc.datacenter_id),
								format!("*.{}.{domain_job}", dc.datacenter_id),
							],
						},
						json!({
							"cert": job_cert_pem,
							"private_key": job_private_key_pem,
							"expire_ts": tls.expire_ts,
						}),
					)?;

					#[derive(Serialize, Hash)]
					struct InsertDbInput {
						datacenter_id: Uuid,
						gg_cert: String,
						gg_private_key: String,
						job_cert: String,
						job_private_key: String,
						expire_ts: i64,
					}

					swf.activity(
						"insert_db",
						InsertDbInput {
							datacenter_id: dc.datacenter_id,
							gg_cert: gg_cert_pem.clone(),
							gg_private_key: gg_private_key_pem.clone(),
							job_cert: job_cert_pem.clone(),
							job_private_key: job_private_key_pem.clone(),
							expire_ts: tls.expire_ts,
						},
						serde_json::Value::Null,
					)?;

					Ok(())
				})?;
			}

			wf.message(
				"cluster_datacenter_create_complete",
				json!({
					"datacenter_id": dc.datacenter_id,
				}),
				cluster::workflows::datacenter::CreateComplete {},
			)?;

			// Scale
			wf.sub_workflow(|swf| {
				#[derive(Serialize, Hash)]
				struct CalculateDiffInput {
					datacenter_id: Uuid,
				}

				swf.activity(
					"calculate_diff",
					CalculateDiffInput {
						datacenter_id: dc.datacenter_id,
					},
					json!({
						"actions": [],
					}),
				)?;

				Ok(())
			})?;

			Ok(())
		})?;
	}

	#[derive(Debug, sqlx::FromRow)]
	struct ServerRow {
		server_id: Uuid,
		datacenter_id: Uuid,
		pool_type: i64,
		provider_server_id: Option<String>,
		provider_hardware: Option<String>,
		vlan_ip: Option<IpAddr>,
		public_ip: Option<IpAddr>,
		is_provisioned: bool,
		is_installed: bool,
		is_draining: bool,
		is_tainted: bool,
	}
	#[derive(Debug, sqlx::FromRow)]
	struct ServerLinodeRow {
		server_id: Uuid,
		ssh_key_id: i64,
		linode_id: Option<i64>,
		firewall_id: Option<i64>,
	}
	#[derive(sqlx::FromRow)]
	struct ServerCfRow {
		server_id: Uuid,
		dns_record_id: Option<String>,
		secondary_dns_record_id: Option<String>,
	}

	let server_rows = sql_fetch_all!(
		[ctx, ServerRow, @tx &mut tx]
		"
		SELECT
			server_id,
			datacenter_id,
			pool_type,
			provider_server_id,
			provider_hardware,
			vlan_ip,
			public_ip,
			(provision_complete_ts IS NOT NULL) AS is_provisioned,
			(install_complete_ts IS NOT NULL) AS is_installed,
			(drain_ts IS NOT NULL) AS is_draining,
			(taint_ts IS NOT NULL) AS is_tainted
		FROM db_cluster.servers
		WHERE cloud_destroy_ts IS NULL
		",
	)
	.await?;
	let server_linode_rows = sql_fetch_all!(
		[ctx, ServerLinodeRow, @tx &mut tx]
		"
		SELECT
			server_id,
			ssh_key_id,
			linode_id,
			firewall_id
		FROM db_cluster.servers_linode
		WHERE destroy_ts IS NULL
		",
	)
	.await?;
	let server_cf_rows = sql_fetch_all!(
		[ctx, ServerCfRow, @tx &mut tx]
		"
		SELECT
			server_id,
			dns_record_id,
			secondary_dns_record_id
		FROM db_cluster.servers_cloudflare
		",
	)
	.await?;

	for server in server_rows {
		let dc = unwrap!(dc_rows
			.iter()
			.find(|dc| dc.datacenter_id == server.datacenter_id));
		let pools = {
			let proto = backend::cluster::Pools::decode(dc.pools.as_slice())?.pools;

			proto
				.into_iter()
				.map(TryInto::<cluster::types::Pool>::try_into)
				.collect::<GlobalResult<Vec<_>>>()?
		};
		let pool_type = server.pool_type.try_into()?;
		let pool = unwrap!(pools.iter().find(|p| p.pool_type == pool_type));

		let linode = server_linode_rows
			.iter()
			.find(|s| s.server_id == server.server_id);
		let cf = server_cf_rows
			.iter()
			.find(|s| s.server_id == server.server_id);

		let install_token = create_token(&ctx).await?;

		let firewall_preset = match pool_type {
			cluster::types::PoolType::Job => linode::types::FirewallPreset::Job,
			cluster::types::PoolType::Gg => linode::types::FirewallPreset::Gg,
			cluster::types::PoolType::Ats => linode::types::FirewallPreset::Ats,
		};

		let linode_server_workflow_id = bctx.workflow("linode_server", |wf| {
			wf.tags(json!({
				"server_id": server.server_id,
			}))?;
			wf.input(json!({
				"server_id": server.server_id,
				"provider_datacenter_id": dc.provider_datacenter_id,
				"custom_image": Option::<String>::None,
				"hardware": unwrap!(pool.hardware.first()).provider_hardware,
				"api_token": dc.provider_api_token.clone(),
				"firewall_preset": firewall_preset.clone(),
				"vlan_ip": server.vlan_ip,
				"tags": [],
			}))?;
			wf.finalize();

			let Some(linode) = linode else { return Ok(()) };

			let Some(linode_id) = linode.linode_id else {
				return Ok(());
			};
			let linode_id = linode_id as u64;

			#[derive(Serialize, Hash)]
			struct CreateSshKeyInput {
				server_id: Uuid,
				api_token: Option<String>,
				is_test: bool,
			}

			wf.activity(
				"create_ssh_key",
				CreateSshKeyInput {
					server_id: server.server_id,
					api_token: dc.provider_api_token.clone(),
					is_test: false,
				},
				json!({
					"ssh_key_id": linode.ssh_key_id,
					// Not the actual public key, but not required
					"public_key": "",
				}),
			)?;

			let ns = util::env::namespace();
			let tags = vec![
				// HACK: Linode requires tags to be > 3 characters. We extend the namespace to make sure it
				// meets the minimum length requirement.
				format!("rivet-{ns}"),
				format!("{ns}-{}", dc.provider_datacenter_id),
				format!("{ns}-{}", firewall_preset),
				format!("{ns}-{}-{}", dc.provider_datacenter_id, firewall_preset),
			];

			#[derive(Serialize, Hash)]
			struct CreateInstanceInput {
				api_token: Option<String>,
				ssh_public_key: String,
				name: String,
				datacenter: String,
				hardware: String,
				tags: Vec<String>,
			}

			wf.activity(
				"create_instance",
				CreateInstanceInput {
					api_token: dc.provider_api_token.clone(),
					// Not the actual public key, but not required
					ssh_public_key: "".to_string(),
					name: format!("{ns}-{}", server.server_id),
					datacenter: dc.provider_datacenter_id.clone(),
					hardware: unwrap!(pool.hardware.first()).provider_hardware.clone(),
					tags: tags.clone(),
				},
				json!({
					"linode_id": linode_id,
					// Not the actual server disk size, but not required
					"server_disk_size": 0,
				}),
			)?;

			#[derive(Serialize, Hash)]
			struct WaitInstanceReadyInput {
				api_token: Option<String>,
				linode_id: u64,
			}

			wf.activity(
				"wait_instance_ready",
				WaitInstanceReadyInput {
					api_token: dc.provider_api_token.clone(),
					linode_id,
				},
				serde_json::Value::Null,
			)?;

			#[derive(Serialize, Hash)]
			struct CreateBootDiskInput {
				api_token: Option<String>,
				image: String,
				ssh_public_key: String,
				linode_id: u64,
				disk_size: u64,
			}

			wf.activity(
				"create_boot_disk",
				CreateBootDiskInput {
					api_token: dc.provider_api_token.clone(),
					image: "linode/debian11".to_string(),
					// Not the actual public key, but not required
					ssh_public_key: "".to_string(),
					linode_id,
					// Not the actual server disk size, but not required
					disk_size: 0,
				},
				// Not the actual boot id, but not required
				0,
			)?;

			#[derive(Serialize, Hash)]
			struct WaitDiskReadyInput {
				api_token: Option<String>,
				linode_id: u64,
				disk_id: u64,
			}

			wf.activity(
				"wait_disk_ready",
				WaitDiskReadyInput {
					api_token: dc.provider_api_token.clone(),
					linode_id,
					// Not the actual boot id, but not required
					disk_id: 0,
				},
				serde_json::Value::Null,
			)?;

			#[derive(Serialize, Hash)]
			struct CreateSwapDiskInput {
				api_token: Option<String>,
				linode_id: u64,
			}

			wf.activity(
				"create_swap_disk",
				CreateSwapDiskInput {
					api_token: dc.provider_api_token.clone(),
					linode_id,
				},
				// Not the actual boot id, but not required
				0,
			)?;

			#[derive(Serialize, Hash)]
			struct CreateInstanceConfigInput {
				api_token: Option<String>,
				vlan_ip: Option<Ipv4Addr>,
				linode_id: u64,
				boot_disk_id: u64,
				swap_disk_id: u64,
			}

			wf.activity(
				"create_instance_config",
				CreateInstanceConfigInput {
					api_token: dc.provider_api_token.clone(),
					vlan_ip: server
						.vlan_ip
						.map(|vlan_ip| {
							if let IpAddr::V4(vlan_ip) = vlan_ip {
								GlobalResult::Ok(vlan_ip)
							} else {
								bail!("unexpected ipv6");
							}
						})
						.transpose()?,
					linode_id,
					// Not the actual boot id, but not required
					boot_disk_id: 0,
					// Not the actual swap id, but not required
					swap_disk_id: 0,
				},
				serde_json::Value::Null,
			)?;

			let Some(firewall_id) = linode.firewall_id else {
				return Ok(());
			};

			#[derive(Debug, Serialize, Deserialize, Hash)]
			struct CreateFirewallInput {
				server_id: Uuid,
				api_token: Option<String>,
				firewall_preset: linode::types::FirewallPreset,
				tags: Vec<String>,
				linode_id: u64,
			}

			wf.activity(
				"create_firewall",
				CreateFirewallInput {
					server_id: server.server_id,
					api_token: dc.provider_api_token.clone(),
					firewall_preset: firewall_preset.clone(),
					tags,
					linode_id,
				},
				firewall_id,
			)?;

			#[derive(Debug, Serialize, Deserialize, Hash)]
			struct BootInstanceInput {
				api_token: Option<String>,
				linode_id: u64,
			}

			wf.activity(
				"boot_instance",
				BootInstanceInput {
					api_token: dc.provider_api_token.clone(),
					linode_id,
				},
				serde_json::Value::Null,
			)?;

			let public_ip = if let Some(public_ip) = server.public_ip {
				if let IpAddr::V4(public_ip) = public_ip {
					public_ip
				} else {
					bail!("unexpected ipv6");
				}
			} else {
				return Ok(());
			};

			#[derive(Serialize, Hash)]
			struct GetPublicIpInput {
				api_token: Option<String>,
				linode_id: u64,
			}

			wf.activity(
				"get_public_ip",
				GetPublicIpInput {
					api_token: dc.provider_api_token.clone(),
					linode_id,
				},
				public_ip,
			)?;

			wf.signal(
				"linode_server_provision_complete",
				json!({
					"linode_id": linode_id,
					"public_ip": public_ip,
					// Not actual boot disk id, but not required
					"boot_disk_id": 0,
				}),
			)?;

			Ok(())
		})?;

		bctx.workflow("cluster_server", |wf| {
			wf.tags(json!({
				"server_id": server.server_id,
			}))?;
			wf.input(json!({
				"datacenter_id": server.datacenter_id,
				"server_id": server.server_id,
				"pool_type": pool_type.clone(),
				"tags": [],
			}))?;
			wf.finalize();

			#[derive(Serialize, Hash)]
			struct GetDcInput {
				datacenter_id: Uuid,
			}

			wf.activity(
				"get_dc",
				GetDcInput {
					datacenter_id: server.datacenter_id,
				},
				cluster::types::Datacenter {
					cluster_id: dc.cluster_id,
					datacenter_id: dc.datacenter_id,
					name_id: dc.name_id.clone(),
					display_name: dc.display_name.clone(),

					provider: dc.provider.try_into()?,
					provider_datacenter_id: dc.provider_datacenter_id.clone(),
					provider_api_token: dc.provider_api_token.clone(),

					pools: {
						let proto = backend::cluster::Pools::decode(dc.pools.as_slice())?.pools;

						proto
							.into_iter()
							.map(TryInto::try_into)
							.collect::<GlobalResult<Vec<_>>>()?
					},

					build_delivery_method: unwrap!(cluster::types::BuildDeliveryMethod::from_repr(
						dc.build_delivery_method.try_into()?,
					)),
					prebakes_enabled: false,
					create_ts: util::timestamp::now(),
				},
			)?;

			let vlan_ip = if let Some(vlan_ip) = server.vlan_ip {
				if let IpAddr::V4(vlan_ip) = vlan_ip {
					vlan_ip
				} else {
					bail!("unexpected ipv6");
				}
			} else {
				return Ok(());
			};

			#[derive(Serialize, Hash)]
			struct GetVlanIpInput {
				datacenter_id: Uuid,
				server_id: Uuid,
				pool_type: cluster::types::PoolType,
			}

			wf.activity(
				"get_vlan_ip",
				GetVlanIpInput {
					datacenter_id: server.datacenter_id,
					server_id: server.server_id,
					pool_type: pool_type.clone(),
				},
				vlan_ip,
			)?;

			wf.dispatch_sub_workflow(linode_server_workflow_id)?;

			if !server.is_provisioned {
				return Ok(());
			}

			let public_ip = if let IpAddr::V4(public_ip) = unwrap!(server.public_ip) {
				public_ip
			} else {
				bail!("unexpected ipv6");
			};

			wf.listen(
				"linode_server_provision_complete",
				json!({
					"linode_id": unwrap!(unwrap!(linode, "no linode row").linode_id, "no linode id"),
					"public_ip": public_ip,
					"boot_disk_id": 0,
				}),
			)?;

			#[derive(Serialize, Hash)]
			struct UpdateDbInput {
				server_id: Uuid,
				pool_type: cluster::types::PoolType,
				cluster_id: Uuid,
				datacenter_id: Uuid,
				provider_datacenter_id: String,
				datacenter_name_id: String,
				provider_server_id: String,
				provider_hardware: String,
				public_ip: Ipv4Addr,
				already_installed: bool,
			}

			wf.activity(
				"update_db",
				UpdateDbInput {
					server_id: server.server_id,
					pool_type: pool_type.clone(),
					cluster_id: dc.cluster_id,
					datacenter_id: server.datacenter_id,
					provider_datacenter_id: dc.provider_datacenter_id.clone(),
					datacenter_name_id: dc.name_id.clone(),
					provider_server_id: unwrap!(server.provider_server_id.clone()),
					provider_hardware: unwrap!(server.provider_hardware.clone()),
					public_ip,
					already_installed: false,
				},
				serde_json::Value::Null,
			)?;

			if !server.is_installed {
				return Ok(());
			}

			// Install
			wf.sub_workflow(|swf| {
				#[derive(Serialize, Hash)]
				struct CreateTokenInput {}

				swf.activity("create_token", CreateTokenInput {}, install_token.clone())?;

				#[derive(Serialize, Hash)]
				struct InstallOverSshInput {
					datacenter_id: Uuid,
					public_ip: Ipv4Addr,
					pool_type: cluster::types::PoolType,
					initialize_immediately: bool,
					server_token: String,
				}

				swf.activity(
					"install_over_ssh",
					InstallOverSshInput {
						datacenter_id: server.datacenter_id,
						public_ip,
						pool_type: pool_type.clone(),
						initialize_immediately: true,
						server_token: install_token.clone(),
					},
					serde_json::Value::Null,
				)?;

				#[derive(Serialize, Hash)]
				struct UpdateDbInput {
					datacenter_id: Uuid,
					server_id: Uuid,
					pool_type: cluster::types::PoolType,
				}

				swf.activity(
					"update_db",
					UpdateDbInput {
						datacenter_id: server.datacenter_id,
						server_id: server.server_id,
						pool_type: pool_type.clone(),
					},
					serde_json::Value::Null,
				)?;

				Ok(())
			})?;

			wf.signal(
				"cluster_datacenter_scale",
				cluster::workflows::datacenter::Scale {},
			)?;

			if let cluster::types::PoolType::Gg = pool_type {
				if let Some(cf) = cf {
					// Dns create
					wf.sub_workflow(|swf| {
						#[derive(Serialize, Hash)]
						struct GetServerInfoInput {
							server_id: Uuid,
						}

						swf.activity(
							"get_server_info",
							GetServerInfoInput {
								server_id: server.server_id,
							},
							json!({
								"datacenter_id": server.datacenter_id,
								"public_ip": public_ip,
							}),
						)?;

						let zone_id =
							unwrap!(util::env::cloudflare::zone::job::id(), "dns not configured");
						let domain_job = unwrap!(util::env::domain_job());

						#[derive(Serialize, Hash)]
						struct CreateDnsRecordInput {
							record_name: String,
							public_ip: Ipv4Addr,
							zone_id: String,
						}

						if let Some(dns_record_id) = &cf.dns_record_id {
							swf.activity(
								"create_dns_record",
								CreateDnsRecordInput {
									record_name: format!(
										"*.lobby.{}.{domain_job}",
										server.datacenter_id
									),
									public_ip,
									zone_id: zone_id.to_string(),
								},
								dns_record_id.clone(),
							)?;
						}

						if let Some(secondary_dns_record_id) = &cf.secondary_dns_record_id {
							swf.activity(
								"create_dns_record",
								CreateDnsRecordInput {
									record_name: format!(
										"lobby.{}.{domain_job}",
										server.datacenter_id
									),
									public_ip,
									zone_id: zone_id.to_string(),
								},
								secondary_dns_record_id.clone(),
							)?;
						}

						if cf.dns_record_id.is_some() && cf.secondary_dns_record_id.is_some() {
							#[derive(Serialize, Hash)]
							struct InsertDbInput {
								server_id: Uuid,
								primary_dns_record_id: String,
								secondary_dns_record_id: String,
							}

							swf.activity(
								"insert_db",
								InsertDbInput {
									server_id: server.server_id,
									primary_dns_record_id: unwrap!(cf.dns_record_id.clone()),
									secondary_dns_record_id: unwrap!(cf
										.secondary_dns_record_id
										.clone()),
								},
								serde_json::Value::Null,
							)?;
						}

						Ok(())
					})?;
				}
			}

			if server.is_draining {
				wf.listen("cluster_server_drain", json!({}))?;

				// Don't need to add the sub workflow, it should be idempotent
			}

			if server.is_tainted {
				wf.listen("cluster_server_taint", json!({}))?;
			}

			Ok(())
		})?;
	}

	bctx.execute(&mut tx).await?;

	tx.commit().await?;

	tracing::info!("backfill complete");

	Ok(())
}

async fn create_token(ctx: &StandaloneCtx) -> GlobalResult<String> {
	use rivet_operation::prelude::proto::{self, backend::pkg::*};

	// Create server token for authenticating API calls from the server
	let token_res = op!([ctx] token_create {
		token_config: Some(token::create::request::TokenConfig {
			ttl: cluster::util::SERVER_TOKEN_TTL,
		}),
		refresh_token_config: None,
		issuer: "cluster-workflow-backfill".to_owned(),
		client: None,
		kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
			entitlements: vec![
				proto::claims::Entitlement {
					kind: Some(
						proto::claims::entitlement::Kind::ProvisionedServer(
							proto::claims::entitlement::ProvisionedServer {}
						)
					)
				}
			],
		})),
		label: Some("srv".to_owned()),
		..Default::default()
	})
	.await?;
	let server_token = unwrap_ref!(token_res.token).token.clone();

	Ok(server_token)
}
