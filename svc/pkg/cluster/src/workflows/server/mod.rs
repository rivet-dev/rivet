use std::{
	convert::TryInto,
	net::{IpAddr, Ipv4Addr},
};

use chirp_workflow::prelude::*;
use rand::Rng;

pub(crate) mod dns_create;
pub(crate) mod dns_delete;
pub(crate) mod drain;
pub(crate) mod install;
pub(crate) mod undrain;

use crate::{
	types::{Datacenter, PoolType, Provider},
	util::metrics,
};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Input {
	pub datacenter_id: Uuid,
	pub server_id: Uuid,
	pub pool_type: PoolType,
	pub tags: Vec<String>,
}

#[workflow]
pub(crate) async fn cluster_server(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	let dc = ctx
		.activity(GetDcInput {
			datacenter_id: input.datacenter_id,
		})
		.await?;

	let pool = unwrap!(
		dc.pools.iter().find(|p| p.pool_type == input.pool_type),
		"datacenter does not have this type of pool configured"
	);

	// Get a new vlan ip
	let vlan_ip = ctx
		.activity(GetVlanIpInput {
			datacenter_id: input.datacenter_id,
			server_id: input.server_id,
			pool_type: input.pool_type.clone(),
		})
		.await?;

	let custom_image = if dc.prebakes_enabled {
		let image_res = ctx
			.activity(GetPrebakeInput {
				datacenter_id: input.datacenter_id,
				pool_type: input.pool_type.clone(),
				provider: dc.provider.clone(),
			})
			.await?;

		// Start custom image creation process
		if image_res.updated {
			ctx.workflow(crate::workflows::prebake::Input {
				datacenter_id: input.datacenter_id,
				provider: dc.provider.clone(),
				pool_type: input.pool_type.clone(),
				install_script_hash: crate::util::INSTALL_SCRIPT_HASH.to_string(),
				tags: Vec::new(),
			})
			.dispatch()
			.await?;
		}

		image_res.custom_image
	} else {
		None
	};
	let already_installed = custom_image.is_some();

	// Iterate through list of hardware and attempt to schedule a server. Goes to the next
	// hardware if an error happens during provisioning
	let mut hardware_list = pool.hardware.iter();
	let provision_res = loop {
		// List exhausted
		let Some(hardware) = hardware_list.next() else {
			break None;
		};

		tracing::info!(
			"attempting to provision hardware: {}",
			hardware.provider_hardware,
		);

		match dc.provider {
			Provider::Linode => {
				let workflow_id = ctx
					.workflow(linode::workflows::server::Input {
						server_id: input.server_id,
						provider_datacenter_id: dc.provider_datacenter_id.clone(),
						custom_image: custom_image.clone(),
						api_token: dc.provider_api_token.clone(),
						hardware: hardware.provider_hardware.clone(),
						firewall_preset: match input.pool_type {
							PoolType::Job => linode::types::FirewallPreset::Job,
							PoolType::Gg => linode::types::FirewallPreset::Gg,
							PoolType::Ats => linode::types::FirewallPreset::Ats,
						},
						vlan_ip: Some(vlan_ip),
						tags: input.tags.clone(),
					})
					.tag("server_id", input.server_id)
					.dispatch()
					.await?;

				match ctx.listen::<Linode>().await? {
					Linode::ProvisionComplete(sig) => {
						break Some(ProvisionResponse {
							provider_server_workflow_id: workflow_id,
							provider_server_id: sig.linode_id.to_string(),
							provider_hardware: hardware.provider_hardware.clone(),
							public_ip: sig.public_ip,
						});
					}
					Linode::ProvisionFailed(_) => {
						tracing::error!(
							provision_workflow_id=%workflow_id,
							server_id=?input.server_id,
							"failed to provision server"
						);
					}
				}
			}
		}
	};

	let provider_server_workflow_id = if let Some(provision_res) = provision_res {
		let provider_server_workflow_id = provision_res.provider_server_workflow_id;
		let public_ip = provision_res.public_ip;

		ctx.activity(UpdateDbInput {
			server_id: input.server_id,
			pool_type: input.pool_type.clone(),
			cluster_id: dc.cluster_id,
			datacenter_id: dc.datacenter_id,
			provider_datacenter_id: dc.provider_datacenter_id.clone(),
			datacenter_name_id: dc.name_id.clone(),
			provider_server_id: provision_res.provider_server_id.clone(),
			provider_hardware: provision_res.provider_hardware.clone(),
			public_ip: provision_res.public_ip,
			already_installed,
		})
		.await?;

		// Install components on server
		if !already_installed {
			let install_res = ctx
				.workflow(install::Input {
					datacenter_id: input.datacenter_id,
					server_id: Some(input.server_id),
					public_ip,
					pool_type: input.pool_type.clone(),
					initialize_immediately: true,
				})
				.run()
				.await;

			// If the server failed all attempts to install, clean it up
			if let Err(err) = ctx.catch_unrecoverable(install_res)? {
				tracing::warn!(?err, "failed installing server, cleaning up");

				ctx.activity(MarkDestroyedInput {
					server_id: input.server_id,
				})
				.await?;

				cleanup(ctx, input, &dc.provider, provider_server_workflow_id, false).await?;

				return Err(err);
			}
		}

		// Scale to get rid of tainted servers
		ctx.signal(crate::workflows::datacenter::Scale {})
			.tag("datacenter_id", input.datacenter_id)
			.send()
			.await?;

		// Create DNS record because the server is already installed
		if let PoolType::Gg = input.pool_type {
			ctx.workflow(dns_create::Input {
				server_id: input.server_id,
			})
			.run()
			.await?;
		}

		provider_server_workflow_id
	} else {
		tracing::error!(
			server_id=?input.server_id,
			hardware_options=?pool.hardware.len(),
			"failed all attempts to provision server"
		);

		// Mark as destroyed (cleanup already occurred in the linode server workflow)
		ctx.activity(MarkDestroyedInput {
			server_id: input.server_id,
		})
		.await?;

		// Scale to bring up a new server to take this server's place
		ctx.signal(crate::workflows::datacenter::Scale {})
			.tag("datacenter_id", input.datacenter_id)
			.send()
			.await?;

		bail!("failed all attempts to provision server");
	};

	// NOTE: This loop has side effects (for state) so we do not use `ctx.repeat`
	let mut state = State::default();
	loop {
		match state.run(ctx).await? {
			Main::DnsCreate(_) => {
				ctx.workflow(dns_create::Input {
					server_id: input.server_id,
				})
				.run()
				.await?;
			}
			Main::DnsDelete(_) => {
				ctx.workflow(dns_delete::Input {
					server_id: input.server_id,
				})
				.run()
				.await?;
			}
			Main::NomadRegistered(sig) => {
				ctx.activity(SetNomadNodeIdInput {
					server_id: input.server_id,
					cluster_id: dc.cluster_id,
					datacenter_id: dc.datacenter_id,
					provider_datacenter_id: dc.provider_datacenter_id.clone(),
					datacenter_name_id: dc.name_id.clone(),
					node_id: sig.node_id,
				})
				.await?;

				// Scale to get rid of tainted servers
				ctx.signal(crate::workflows::datacenter::Scale {})
					.tag("datacenter_id", input.datacenter_id)
					.send()
					.await?;
			}
			Main::Drain(_) => {
				ctx.workflow(drain::Input {
					datacenter_id: input.datacenter_id,
					server_id: input.server_id,
					pool_type: input.pool_type.clone(),
				})
				.run()
				.await?;
			}
			Main::Undrain(_) => {
				ctx.workflow(undrain::Input {
					datacenter_id: input.datacenter_id,
					server_id: input.server_id,
					pool_type: input.pool_type.clone(),
				})
				.run()
				.await?;
			}
			Main::Taint(_) => {} // Only for state
			Main::Destroy(_) => break,
		}
	}

	cleanup(
		ctx,
		input,
		&dc.provider,
		provider_server_workflow_id,
		state.has_dns,
	)
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub(crate) struct GetDcInput {
	pub datacenter_id: Uuid,
}

#[activity(GetDc)]
pub(crate) async fn get_dc(ctx: &ActivityCtx, input: &GetDcInput) -> GlobalResult<Datacenter> {
	let dcs_res = ctx
		.op(crate::ops::datacenter::get::Input {
			datacenter_ids: vec![input.datacenter_id],
		})
		.await?;
	let dc = unwrap!(dcs_res.datacenters.into_iter().next());

	Ok(dc)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct GetVlanIpInput {
	datacenter_id: Uuid,
	server_id: Uuid,
	pool_type: PoolType,
}

#[activity(GetVlanIp)]
async fn get_vlan_ip(ctx: &ActivityCtx, input: &GetVlanIpInput) -> GlobalResult<Ipv4Addr> {
	// Find next available vlan index
	let mut vlan_addr_range = match input.pool_type {
		PoolType::Job => util::net::job::vlan_addr_range(),
		PoolType::Gg => util::net::gg::vlan_addr_range(),
		PoolType::Ats => util::net::ats::vlan_addr_range(),
	};
	let max_idx = vlan_addr_range.count() as i64;

	let (network_idx,) = sql_fetch_one!(
		[ctx, (i64,)]
		"
		WITH
			get_next_network_idx AS (
				SELECT mod(idx + $1, $2) AS idx
				FROM generate_series(0, $2) AS s(idx)
				WHERE NOT EXISTS (
					SELECT 1
					FROM db_cluster.servers
					WHERE
						pool_type2 = $3 AND
						-- Technically this should check all servers where their datacenter's provider and
						-- provider_datacenter_id are the same because VLAN is separated by irl datacenter
						-- but this is good enough
						datacenter_id = $4 AND
						network_idx = mod(idx + $1, $2) AND
						cloud_destroy_ts IS NULL
				)
				LIMIT 1
			),
			update_network_idx AS (
				UPDATE db_cluster.servers
				SET network_idx = (SELECT idx FROM get_next_network_idx) 
				WHERE server_id = $5
				RETURNING 1
			)
		SELECT idx FROM get_next_network_idx
		",
		// Choose a random index to start from for better index spread
		rand::thread_rng().gen_range(0i64..max_idx),
		max_idx,
		serde_json::to_string(&input.pool_type)?,
		input.datacenter_id,
		input.server_id,
	)
	.await?;

	let vlan_ip = unwrap!(vlan_addr_range.nth(network_idx.try_into()?));

	// Write vlan ip
	sql_execute!(
		[ctx]
		"
		UPDATE db_cluster.servers
		SET vlan_ip = $2
		WHERE server_id = $1
		",
		input.server_id,
		IpAddr::V4(vlan_ip),
	)
	.await?;

	Ok(vlan_ip)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct GetPrebakeInput {
	datacenter_id: Uuid,
	pool_type: PoolType,
	provider: Provider,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct GetPrebakeOutput {
	custom_image: Option<String>,
	updated: bool,
}

#[activity(GetPrebake)]
async fn get_prebake(ctx: &ActivityCtx, input: &GetPrebakeInput) -> GlobalResult<GetPrebakeOutput> {
	// Get the custom image id for this server, or insert a record and start creating one
	let (image_id, updated) = sql_fetch_one!(
		[ctx, (Option<String>, bool)]
		"
		WITH
			updated AS (
				INSERT INTO db_cluster.server_images2 AS s (
					provider, install_hash, datacenter_id, pool_type, create_ts
				)
				VALUES ($1, $2, $3, $4, $5)
				ON CONFLICT (provider, install_hash, datacenter_id, pool_type) DO UPDATE
					SET
						provider_image_id = NULL,
						create_ts = $5
					WHERE s.create_ts < $6
				RETURNING provider, install_hash, datacenter_id, pool_type
			),
			selected AS (
				SELECT provider, install_hash, datacenter_id, pool_type, provider_image_id
				FROM db_cluster.server_images2
				WHERE
					provider = $1 AND
					install_hash = $2 AND
					datacenter_id = $3 AND
					pool_type = $4
			)
		SELECT
			selected.provider_image_id,
			-- Primary key is not null
			(updated.provider IS NOT NULL) AS updated
		FROM selected
		FULL OUTER JOIN updated
		ON true
		",
		serde_json::to_string(&input.provider)?,
		crate::util::INSTALL_SCRIPT_HASH,
		input.datacenter_id,
		serde_json::to_string(&input.pool_type)?,
		util::timestamp::now(),
		// 5 month expiration
		util::timestamp::now() - util::duration::days(5 * 30),
	)
	.await?;

	// Updated is true if this specific sql call either reset (if expired) or inserted the row
	Ok(GetPrebakeOutput {
		custom_image: if updated { None } else { image_id },
		updated,
	})
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct ProvisionResponse {
	provider_server_workflow_id: Uuid,
	provider_server_id: String,
	provider_hardware: String,
	public_ip: Ipv4Addr,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateDbInput {
	server_id: Uuid,
	pool_type: PoolType,
	cluster_id: Uuid,
	datacenter_id: Uuid,
	provider_datacenter_id: String,
	datacenter_name_id: String,
	provider_server_id: String,
	provider_hardware: String,
	public_ip: Ipv4Addr,
	already_installed: bool,
}

#[activity(UpdateDb)]
async fn update_db(ctx: &ActivityCtx, input: &UpdateDbInput) -> GlobalResult<()> {
	let provision_complete_ts = util::timestamp::now();

	let (create_ts,) = sql_fetch_one!(
		[ctx, (i64,)]
		"
		UPDATE db_cluster.servers
		SET
			provider_server_id = $2,
			provider_hardware = $3,
			public_ip = $4,
			provision_complete_ts = $5,
			install_complete_ts = $6
		WHERE server_id = $1
		RETURNING create_ts
		",
		input.server_id,
		&input.provider_server_id,
		&input.provider_hardware,
		IpAddr::V4(input.public_ip),
		provision_complete_ts,
		if input.already_installed {
			Some(provision_complete_ts)
		} else {
			None
		},
	)
	.await?;

	insert_metrics(
		input.cluster_id,
		input.datacenter_id,
		&input.provider_datacenter_id,
		&input.datacenter_name_id,
		&input.pool_type,
		provision_complete_ts,
		create_ts,
	)
	.await;

	Ok(())
}

async fn insert_metrics(
	cluster_id: Uuid,
	datacenter_id: Uuid,
	provider_datacenter_id: &str,
	datacenter_name_id: &str,
	pool_type: &PoolType,
	provision_complete_ts: i64,
	create_ts: i64,
) {
	let dt = (provision_complete_ts - create_ts) as f64 / 1000.0;

	metrics::PROVISION_DURATION
		.with_label_values(&[
			&cluster_id.to_string(),
			&datacenter_id.to_string(),
			provider_datacenter_id,
			datacenter_name_id,
			match pool_type {
				PoolType::Job => "job",
				PoolType::Gg => "gg",
				PoolType::Ats => "ats",
			},
		])
		.observe(dt);
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct MarkDestroyedInput {
	server_id: Uuid,
}

#[activity(MarkDestroyed)]
async fn mark_destroyed(ctx: &ActivityCtx, input: &MarkDestroyedInput) -> GlobalResult<()> {
	// Mark servers for destruction in db
	sql_execute!(
		[ctx]
		"
		UPDATE db_cluster.servers
		SET cloud_destroy_ts = $2
		WHERE server_id = $1
		",
		input.server_id,
		util::timestamp::now(),
	)
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct SetNomadNodeIdInput {
	server_id: Uuid,
	cluster_id: Uuid,
	datacenter_id: Uuid,
	provider_datacenter_id: String,
	datacenter_name_id: String,
	node_id: String,
}

#[activity(SetNomadNodeId)]
async fn set_nomad_node_id(ctx: &ActivityCtx, input: &SetNomadNodeIdInput) -> GlobalResult<()> {
	let nomad_join_ts = util::timestamp::now();

	let (old_nomad_node_id, install_complete_ts) = sql_fetch_one!(
		[ctx, (Option<String>, Option<i64>)]
		"
		UPDATE db_cluster.servers
		SET
			nomad_node_id = $2,
			nomad_join_ts = $3
		WHERE
			server_id = $1
		RETURNING nomad_node_id, install_complete_ts
		",
		input.server_id,
		&input.node_id,
		nomad_join_ts,
	)
	.await?;

	if let Some(old_nomad_node_id) = old_nomad_node_id {
		tracing::warn!(%old_nomad_node_id, "nomad node id was already set");
	}

	// Insert metrics
	if let Some(install_complete_ts) = install_complete_ts {
		insert_nomad_metrics(
			input.cluster_id,
			input.datacenter_id,
			&input.provider_datacenter_id,
			&input.datacenter_name_id,
			nomad_join_ts,
			install_complete_ts,
		);
	} else {
		tracing::warn!("missing install_complete_ts");
	}

	Ok(())
}

fn insert_nomad_metrics(
	cluster_id: Uuid,
	datacenter_id: Uuid,
	provider_datacenter_id: &str,
	datacenter_name_id: &str,
	nomad_join_ts: i64,
	install_complete_ts: i64,
) {
	let dt = (nomad_join_ts - install_complete_ts) as f64 / 1000.0;

	metrics::NOMAD_JOIN_DURATION
		.with_label_values(&[
			&cluster_id.to_string(),
			&datacenter_id.to_string(),
			provider_datacenter_id,
			datacenter_name_id,
		])
		.observe(dt);
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct SetDrainCompleteInput {
	server_id: Uuid,
}

#[activity(SetDrainComplete)]
async fn set_drain_complete(ctx: &ActivityCtx, input: &SetDrainCompleteInput) -> GlobalResult<()> {
	// Set as completed draining. Will be destroyed by `cluster-datacenter-scale`
	sql_execute!(
		[ctx]
		"
		UPDATE db_cluster.servers
		SET drain_complete_ts = $2
		WHERE server_id = $1
		",
		input.server_id,
		util::timestamp::now(),
	)
	.await?;

	Ok(())
}

async fn cleanup(
	ctx: &mut WorkflowCtx,
	input: &Input,
	provider: &Provider,
	provider_server_workflow_id: Uuid,
	cleanup_dns: bool,
) -> GlobalResult<()> {
	if cleanup_dns {
		// Cleanup DNS
		if let PoolType::Gg = input.pool_type {
			ctx.workflow(dns_delete::Input {
				server_id: input.server_id,
			})
			.run()
			.await?;
		}
	}

	// Cleanup server
	match provider {
		Provider::Linode => {
			tracing::info!(server_id=?input.server_id, "destroying linode server");

			ctx.signal(linode::workflows::server::Destroy {})
				.tag("server_id", input.server_id)
				.send()
				.await?;

			// Wait for workflow to complete
			ctx.wait_for_workflow::<linode::workflows::server::Workflow>(
				provider_server_workflow_id,
			)
			.await?;
		}
	}

	Ok(())
}

/// Finite state machine for handling server updates.
struct State {
	draining: bool,
	has_dns: bool,
	is_tainted: bool,
}

impl State {
	async fn run(&mut self, ctx: &mut WorkflowCtx) -> GlobalResult<Main> {
		let signal = ctx.custom_listener(self).await?;

		// Update state
		self.transition(&signal);

		Ok(signal)
	}

	fn transition(&mut self, signal: &Main) {
		match signal {
			Main::Drain(_) => self.draining = true,
			Main::Undrain(_) => self.draining = false,
			Main::Taint(_) => self.is_tainted = true,
			Main::DnsCreate(_) => self.has_dns = true,
			Main::DnsDelete(_) => self.has_dns = false,
			_ => {}
		}
	}
}

#[async_trait::async_trait]
impl CustomListener for State {
	type Output = Main;

	/* ==== BINARY CONDITION DECOMPOSITION ====

	// state
	drain  dns  taint // available actions
		0    0      0 // drain,   taint, dns create
		0    0      1 // drain
		0    1      0 // drain,   taint, dns delete
		0    1      1 // drain,          dns delete
		1    0      0 // undrain, taint,             nomad drain complete
		1    0      1 //                             nomad drain complete
		1    1      0 // undrain, taint, dns delete, nomad drain complete
		1    1      1 //                             nomad drain complete

	destroy				 // always
	drain				 // if !drain
	undrain				 // if drain && !taint
	taint				 // if !taint
	dns create			 // if !dns && !drain && !taint
	dns delete			 // if dns && (!drain || !taint)
	nomad registered	 // always
	nomad drain complete // if drain
	*/
	async fn listen(&self, ctx: &ListenCtx) -> WorkflowResult<Self::Output> {
		// Determine which signals to listen to
		let mut signals = vec![Destroy::NAME, NomadRegistered::NAME];

		if !self.draining {
			signals.push(Drain::NAME);
		} else if !self.is_tainted {
			signals.push(Undrain::NAME);
		}

		if !self.is_tainted {
			signals.push(Taint::NAME);
		}

		if !self.has_dns && !self.draining && !self.is_tainted {
			signals.push(DnsCreate::NAME);
		}

		if self.has_dns && (!self.draining || !self.is_tainted) {
			signals.push(DnsDelete::NAME);
		}

		let row = ctx.listen_any(&signals).await?;
		Self::parse(&row.signal_name, row.body)
	}

	fn parse(name: &str, body: serde_json::Value) -> WorkflowResult<Self::Output> {
		Main::parse(name, body)
	}
}

impl Default for State {
	fn default() -> Self {
		State {
			draining: false,
			has_dns: true,
			is_tainted: false,
		}
	}
}

// Listen for linode provision signals
type ProvisionComplete = linode::workflows::server::ProvisionComplete;
type ProvisionFailed = linode::workflows::server::ProvisionFailed;
join_signal!(pub(crate) Linode, [ProvisionComplete, ProvisionFailed]);

#[signal("cluster_server_drain")]
pub struct Drain {}

#[signal("cluster_server_undrain")]
pub struct Undrain {}

#[signal("cluster_server_taint")]
pub struct Taint {}

#[signal("cluster_server_dns_create")]
pub struct DnsCreate {}

#[signal("cluster_server_dns_delete")]
pub struct DnsDelete {}

#[signal("cluster_server_destroy")]
pub struct Destroy {}

#[signal("cluster_server_nomad_registered")]
pub struct NomadRegistered {
	pub node_id: String,
}

join_signal!(
	Main,
	[
		Drain,
		Undrain,
		Taint,
		DnsCreate,
		DnsDelete,
		Destroy,
		NomadRegistered
	]
);
