use std::{
	io::{Read, Write},
	net::{Ipv4Addr, TcpStream},
	path::Path,
};

use chirp_workflow::prelude::*;
use rivet_operation::prelude::proto::{self, backend::pkg::token};
use ssh2::Session;

use crate::{
	types::{Datacenter, PoolType},
	util::metrics,
};

mod install_scripts;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Input {
	pub datacenter_id: Uuid,
	pub server_id: Option<Uuid>,
	pub public_ip: Ipv4Addr,
	pub pool_type: PoolType,
	pub initialize_immediately: bool,
}

#[workflow]
pub(crate) async fn cluster_server_install(
	ctx: &mut WorkflowCtx,
	input: &Input,
) -> GlobalResult<()> {
	let server_token = ctx.activity(CreateTokenInput {}).await?;

	ctx.activity(InstallOverSshInput {
		datacenter_id: input.datacenter_id,
		public_ip: input.public_ip,
		pool_type: input.pool_type.clone(),
		initialize_immediately: input.initialize_immediately,
		server_token,
	})
	.await?;

	// If the server id is set this is not a prebake server
	if let Some(server_id) = input.server_id {
		ctx.activity(UpdateDbInput {
			datacenter_id: input.datacenter_id,
			server_id,
			pool_type: input.pool_type.clone(),
		})
		.await?;
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct CreateTokenInput {}

#[activity(CreateToken)]
async fn create_token(ctx: &ActivityCtx, input: &CreateTokenInput) -> GlobalResult<String> {
	// Create server token for authenticating API calls from the server
	let token_res = op!([ctx] token_create {
		token_config: Some(token::create::request::TokenConfig {
			ttl: crate::util::SERVER_TOKEN_TTL,
		}),
		refresh_token_config: None,
		issuer: "cluster-worker-server-install".to_owned(),
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

#[derive(Debug, Serialize, Deserialize, Hash)]
struct InstallOverSshInput {
	datacenter_id: Uuid,
	public_ip: Ipv4Addr,
	pool_type: PoolType,
	initialize_immediately: bool,
	server_token: String,
}

#[activity(InstallOverSsh)]
#[timeout = 300]
#[max_retries = 10]
async fn install_over_ssh(ctx: &ActivityCtx, input: &InstallOverSshInput) -> GlobalResult<()> {
	let public_ip = input.public_ip;
	let private_key_openssh =
		util::env::read_secret(&["ssh", "server", "private_key_openssh"]).await?;

	let install_script = install_scripts::gen_install(
		input.pool_type.clone(),
		input.initialize_immediately,
		&input.server_token,
		input.datacenter_id,
	)
	.await?;
	let hook_script = install_scripts::gen_hook(&input.server_token).await?;
	let initialize_script =
		install_scripts::gen_initialize(input.pool_type.clone(), input.datacenter_id).await?;

	// Spawn blocking thread for ssh (no async support)
	tokio::task::spawn_blocking(move || {
		tracing::info!(%public_ip, "connecting over ssh");
		let tcp = TcpStream::connect((public_ip, 22))?;
		let mut sess = Session::new()?;
		sess.set_tcp_stream(tcp);

		if let Err(err) = sess.handshake() {
			tracing::error!(%public_ip, ?err, "failed to connect over ssh");
			retry_bail!("failed to connect over ssh");
		}

		tracing::info!(%public_ip, "connected");

		sess.userauth_pubkey_memory("root", None, &private_key_openssh, None)?;

		tracing::info!("authenticated");

		tracing::info!("writing scripts");

		write_script(&sess, "rivet_install", &install_script)?;
		write_script(&sess, "rivet_hook", &hook_script)?;
		write_script(&sess, "rivet_initialize", &initialize_script)?;

		tracing::info!("executing install script");

		let mut channel = sess.channel_session()?;

		// Cannot run more than one command at a time in a channel, simply combine them
		let script = [
			"chmod +x /usr/bin/rivet_install.sh",
			"chmod +x /usr/bin/rivet_hook.sh",
			"chmod +x /usr/bin/rivet_initialize.sh",
			"/usr/bin/rivet_install.sh",
		]
		.join(" && ");

		channel.exec(&script)?;

		let mut stdout = String::new();
		channel.read_to_string(&mut stdout)?;
		let mut stderr = String::new();
		channel.stderr().read_to_string(&mut stderr)?;

		channel.wait_close()?;

		if channel.exit_status()? != 0 {
			tracing::error!(%public_ip, %stdout, %stderr, "failed to run script");
			bail!("failed to run script");
		}

		tracing::info!("install successful");

		GlobalResult::Ok(())
	})
	.await??;

	Ok(())
}

fn write_script(sess: &Session, script_name: &str, content: &str) -> GlobalResult<()> {
	let bytes = content.as_bytes();

	let mut script_file = sess.scp_send(
		Path::new(&format!("/usr/bin/{script_name}.sh")),
		0o644,
		bytes.len() as u64,
		None,
	)?;

	// Write script in chunks
	let mut idx = 0;
	loop {
		let start = idx;
		let end = (idx + 1024).min(bytes.len());

		script_file.write_all(&bytes[start..end])?;

		idx = end;
		if idx >= bytes.len() {
			break;
		}
	}

	// Close the channel and wait for the whole content to be transferred
	script_file.send_eof()?;
	script_file.wait_eof()?;
	script_file.close()?;
	script_file.wait_close()?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UpdateDbInput {
	datacenter_id: Uuid,
	server_id: Uuid,
	pool_type: PoolType,
}

#[activity(UpdateDb)]
async fn update_db(ctx: &ActivityCtx, input: &UpdateDbInput) -> GlobalResult<()> {
	let install_complete_ts = util::timestamp::now();

	let (provision_complete_ts,) = sql_fetch_one!(
		[ctx, (i64,)]
		"
		UPDATE db_cluster.servers
		SET install_complete_ts = $2
		WHERE server_id = $1
		RETURNING provision_complete_ts
		",
		input.server_id,
		install_complete_ts,
	)
	.await?;

	let datacenters_res = ctx
		.op(crate::ops::datacenter::get::Input {
			datacenter_ids: vec![input.datacenter_id],
		})
		.await?;
	let dc = unwrap!(datacenters_res.datacenters.first());

	insert_metrics(
		&input.pool_type,
		dc,
		install_complete_ts,
		provision_complete_ts,
	);

	Ok(())
}

fn insert_metrics(
	pool_type: &PoolType,
	dc: &Datacenter,
	install_complete_ts: i64,
	provision_complete_ts: i64,
) {
	let dt = (install_complete_ts - provision_complete_ts) as f64 / 1000.0;

	metrics::INSTALL_DURATION
		.with_label_values(&[
			&dc.cluster_id.to_string(),
			&dc.datacenter_id.to_string(),
			&dc.provider_datacenter_id,
			&dc.name_id,
			match pool_type {
				PoolType::Job => "job",
				PoolType::Gg => "gg",
				PoolType::Ats => "ats",
			},
		])
		.observe(dt);
}
