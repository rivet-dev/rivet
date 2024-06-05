// TODO: This test doesnt work and should be removed

use std::{
	net::{IpAddr, Ipv4Addr},
	sync::Arc,
	time::Duration,
};

use anyhow::*;
// use futures_util::{FutureExt, StreamExt, TryStreamExt};
use indoc::indoc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wf::*;

mod common;

#[tokio::test]
async fn provisioning() -> Result<()> {
	common::setup();

	let db =
		DatabasePostgres::new("postgres://root@127.0.0.1:26257/postgres?sslmode=disable").await?;

	let mut registry = Registry::new();
	registry.register_workflow::<ServerWorkflow>();
	let registry = registry.handle();

	let worker = Worker::new(registry.clone(), db.clone());
	tokio::spawn(async move {
		if let Err(err) = worker.start().await {
			tracing::error!(?err, "worker failed");
		}
	});

	let ctx = common::TestCtx::new(db.clone());
	dc_scale(ctx, db).await?;

	Ok(())
}

async fn dc_scale(ctx: common::TestCtxHandle, db: Arc<DatabasePostgres>) -> Result<()> {
	let server_id = Uuid::new_v4();
	let datacenter_id = Uuid::new_v4();

	let workflow_id = Uuid::new_v4();

	sqlx::query(indoc!(
		"
		INSERT INTO servers (
			server_id,
			datacenter_id,
			pool_type,
			create_ts
		)
		VALUES($1, $2, $3, $4)
		",
	))
	.bind(server_id)
	.bind(workflow_id)
	.bind(datacenter_id)
	.bind(serde_json::to_string(&PoolType::Job)?)
	.bind(util::now())
	.execute(&mut *db.conn().await?)
	.await
	.map_err(WorkflowError::Sqlx)?;

	// Provision server
	ctx.dispatch_workflow_with_id(
		workflow_id,
		ServerInput {
			provider: Provider::Linode,
		},
	)
	.await?;

	tokio::time::sleep(std::time::Duration::from_secs(5)).await;
	ctx.signal(workflow_id, DestroyServer {}).await?;

	let output = ctx.wait_for_workflow::<ServerWorkflow>(workflow_id).await?;

	Ok(())
}

#[derive(sqlx::FromRow)]
struct ServerRow {
	server_id: Uuid,
	datacenter_id: Uuid,
	pool_type: sqlx::types::Json<PoolType>,
	create_ts: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInput {
	server_id: Uuid,
	provider: Provider,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerOutput {}

#[macros::workflow(ServerWorkflow)]
async fn server_workflow(ctx: &mut WorkflowCtx, input: &ServerInput) -> Result<ServerOutput> {
	let db =
		DatabasePostgres::new("postgres://root@127.0.0.1:26257/postgres?sslmode=disable").await?;

	ctx.activity(SetServerWorkflowInput {
		server_id: input.server_id,
	})
	.await?;

	// TODO: Replace with server_get operation
	let server = sqlx::query_as::<_, ServerRow>(indoc!(
		"
		SELECT *
		FROM servers
		WHERE server_id = $1
		",
	))
	.bind(input.server_id)
	.fetch_one(&mut *db.conn().await?)
	.await
	.map_err(WorkflowError::Sqlx)?;

	let dc_get_res = ctx
		.op(DatacenterGetInput {
			datacenter_ids: vec![server.datacenter_id],
		})
		.await?;
	let dc = dc_get_res.datacenters.first().context("dc not found")?;
	let pool = dc
		.pools
		.iter()
		.find(|p| p.pool_type == *server.pool_type)
		.context("datacenter does not have this type of pool configured")?;

	ctx.activity(AssignVlanIpInput {
		server_id: input.server_id,
	})
	.await?;

	let provision_res = ctx.workflow(ProvisionServerInput {}).await?;

	// Destroy early
	if let Some(destroy_signal) = ctx.query_signal::<DestroyServer>().await? {
		ctx.workflow(DestroyServerInput {}).await?;

		return Ok(ServerOutput {});
	}

	// Install components
	if !provision_res.already_installed {
		ctx.workflow(ServerInstallInput {}).await?;
	}

	// Create DNS record
	if let PoolType::Gg = input.pool_type {
		ctx.workflow(ServerCreateDnsInput {}).await?;
	}

	match ctx.listen::<LiveJoin>().await? {
		LiveJoin::DrainServer(sig) => {
			ctx.workflow(DrainServerInput {}).await?;

			match ctx.listen::<UndrainJoin>().await? {
				UndrainJoin::UndrainServer(sig) => {}
				UndrainJoin::DestroyServer(sig) => ctx.workflow(DestroyServerInput {}).await?,
			}
		}
		LiveJoin::TaintServer(sig) => {
			ctx.workflow(TaintServerInput {}).await?;

			let destroy_sig = ctx.listen::<DestroyServer>().await?;
		}
		LiveJoin::DestroyServer(sig) => ctx.workflow(DestroyServerInput {}).await?,
	}

	Ok(ServerOutput {})
}

join_signal!(LiveJoin, [DrainServer, TaintServer, DestroyServer]);
join_signal!(UndrainJoin, [DrainServer, DestroyServer]);

fn provision_server() {
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
			hardware.provider_hardware
		);

		match input.provider {
			Provider::Linode => {
				// TODO: Workflow
				// let res = op!([ctx] linode_server_provision {
				// 	datacenter_id: ctx.datacenter_id,
				// 	server_id: ctx.server_id,
				// 	provider_datacenter_id: datacenter.provider_datacenter_id.clone(),
				// 	hardware: Some(hardware.clone()),
				// 	pool_type: server.pool_type,
				// 	vlan_ip: vlan_ip.to_string(),
				// 	tags: ctx.tags.clone(),
				// })
				// .await;

				match res {
					Ok(res) => {
						break Some(ProvisionResponse {
							provider_server_id: res.provider_server_id.clone(),
							provider_hardware: hardware.provider_hardware.clone(),
							public_ip: res.public_ip.clone(),
							already_installed: res.already_installed,
						})
					}
					Err(err) => {
						tracing::error!(
							?err,
							?server_id,
							"failed to provision server, cleaning up"
						);

						// TODO: Workflow
						// cleanup(&ctx, server_id).await?;
					}
				}
			}
		}
	};

	if let Some(provision_res) = provision_res {
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
			server_id,
			&provision_res.provider_server_id,
			&provision_res.provider_hardware,
			&provision_res.public_ip,
			provision_complete_ts,
			if provision_res.already_installed {
				Some(provision_complete_ts)
			} else {
				None
			},
		)
		.await?;
	} else {
		tracing::error!(?server_id, hardware_options=?pool.hardware.len(), "failed all attempts to provision server");
		bail!("failed all attempts to provision server");
	}
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct SetServerWorkflowInput {
	server_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct SetServerWorkflowOutput {}

#[macros::activity(SetServerWorkflow)]
pub fn set_server_workflow(
	ctx: &mut ActivityCtx,
	input: &SetServerWorkflowInput,
) -> Result<SetServerWorkflowOutput> {
	let db =
		DatabasePostgres::new("postgres://root@127.0.0.1:26257/postgres?sslmode=disable").await?;

	sqlx::query(indoc!(
		"
		UPDATE servers
		SET workflow_id = $2
		WHERE server_id = $1
		",
	))
	.bind(input.server_id)
	.bind(ctx.workflow_id)
	.execute(&mut *db.conn().await?)
	.await
	.map_err(WorkflowError::Sqlx)?;

	Ok(SetServerWorkflowOutput {})
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct AssignVlanIpInput {
	server_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct AssignVlanIpOutput {
	vlan_ip: Ipv4Addr,
}

#[macros::activity(AssignVlanIp)]
pub fn assign_vlan_ip(
	ctx: &mut ActivityCtx,
	input: &AssignVlanIpInput,
) -> Result<AssignVlanIpOutput> {
	let db =
		DatabasePostgres::new("postgres://root@127.0.0.1:26257/postgres?sslmode=disable").await?;

	// Get a new vlan ip
	let vlan_ip = Ipv4Addr::new(1, 2, 3, 4);

	sqlx::query(indoc!(
		"
		UPDATE servers
		SET vlan_ip = $2
		WHERE server_id = $1
		",
	))
	.bind(input.server_id)
	.bind(IpAddr::V4(vlan_ip))
	.execute(&mut *db.conn().await?)
	.await
	.map_err(WorkflowError::Sqlx)?;

	Ok(AssignVlanIpOutput { vlan_ip })
}

mod ops {
	use anyhow::*;
	use serde::{Deserialize, Serialize};
	use uuid::Uuid;
	use wf::*;

	pub struct DatacenterGetInput {
		pub datacenter_ids: Vec<Uuid>,
	}

	pub struct DatacenterGetOutput {
		pub datacenters: Vec<Datacenter>,
	}

	#[macros::operation(DatacenterGet)]
	pub fn datacenter_get(
		ctx: &mut OperationCtx,
		input: &DatacenterGetInput,
	) -> Result<DatacenterGetOutput> {
		Ok(DatacenterGetOutput {
			datacenters: input
				.datacenter_ids
				.iter()
				.map(|id| Datacenter {
					datacenter_id: *id,
					pools: vec![Pool {
						pool_type: PoolType::Job,
					}],
				})
				.collect(),
		})
	}

	pub struct Datacenter {
		pub datacenter_id: Uuid,
		pub pools: Vec<Pool>,
	}

	pub struct Pool {
		pub pool_type: PoolType,
	}

	#[derive(Serialize, Deserialize, PartialEq)]
	pub enum PoolType {
		Job,
		Gg,
		Ats,
	}

	#[derive(Serialize, Deserialize, PartialEq)]
	pub enum Provider {
		Linode,
	}
}
use ops::*;
