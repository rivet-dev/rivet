use std::{
	io::{Read, Write},
	net::TcpStream,
	path::Path,
};

use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use ssh2::Session;
use util_cluster::metrics;

mod install_scripts;

#[worker(name = "cluster-server-install", timeout = 200)]
async fn worker(ctx: &OperationContext<cluster::msg::server_install::Message>) -> GlobalResult<()> {
	let datacenter_id = unwrap!(ctx.datacenter_id).as_uuid();

	// Check for stale message
	if ctx.req_dt() > util::duration::hours(1) {
		tracing::warn!("discarding stale message");

		return Ok(());
	}

	if let Some(server_id) = ctx.server_id {
		let (is_destroying_or_draining,) = sql_fetch_one!(
			[ctx, (bool,)]
			"
			SELECT EXISTS(
				SELECT 1
				FROM db_cluster.servers
				WHERE server_id = $1 AND
				(cloud_destroy_ts IS NOT NULL OR drain_ts IS NOT NULL)
			)
			",
			server_id.as_uuid(),
		)
		.await?;

		if is_destroying_or_draining {
			tracing::info!("server marked for deletion/drain, not installing");
			return Ok(());
		}
	}

	let public_ip = ctx.public_ip.clone();
	let pool_type = unwrap!(backend::cluster::PoolType::from_i32(ctx.pool_type));
	let private_key_openssh =
		util::env::read_secret(&["ssh", "server", "private_key_openssh"]).await?;

	// Create server token for authenticating API calls from the server
	let token_res = op!([ctx] token_create {
		token_config: Some(token::create::request::TokenConfig {
			ttl: util_cluster::SERVER_TOKEN_TTL,
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
	let server_token = &unwrap_ref!(token_res.token).token;

	let install_script = install_scripts::gen_install(
		pool_type,
		ctx.initialize_immediately,
		server_token,
		datacenter_id,
	)
	.await?;
	let hook_script = install_scripts::gen_hook(server_token).await?;
	let initialize_script = install_scripts::gen_initialize(pool_type).await?;

	// Spawn blocking thread for ssh (no async support)
	tokio::task::spawn_blocking(move || {
		tracing::info!(%public_ip, "connecting over ssh");
		let tcp = TcpStream::connect((public_ip.as_str(), 22))?;
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
			tracing::error!(%stdout, %stderr, "failed to run script");
			bail!("failed to run script");
		}

		tracing::info!("install successful");

		GlobalResult::Ok(())
	})
	.await??;

	let request_id = unwrap_ref!(ctx.request_id).as_uuid();
	msg!([ctx] cluster::msg::server_install_complete(request_id) {
		request_id: ctx.request_id,
		public_ip: ctx.public_ip.clone(),
		datacenter_id: ctx.datacenter_id,
		server_id: ctx.server_id,
		provider: ctx.provider,
	})
	.await?;

	// If the server id is set this is not a prebake server
	if let Some(server_id) = ctx.server_id {
		let install_complete_ts = util::timestamp::now();

		let (provision_complete_ts,) = sql_fetch_one!(
			[ctx, (i64,)]
			"
			UPDATE db_cluster.servers
			SET install_complete_ts = $2
			WHERE server_id = $1
			RETURNING provision_complete_ts
			",
			server_id.as_uuid(),
			install_complete_ts,
		)
		.await?;

		// Scale to get rid of tainted servers
		msg!([ctx] @recursive cluster::msg::datacenter_scale(datacenter_id) {
			datacenter_id: ctx.datacenter_id,
		})
		.await?;

		insert_metrics(
			ctx,
			&pool_type,
			datacenter_id,
			install_complete_ts,
			provision_complete_ts,
		)
		.await?;
	}

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

async fn insert_metrics(
	ctx: &OperationContext<cluster::msg::server_install::Message>,
	pool_type: &backend::cluster::PoolType,
	datacenter_id: Uuid,
	install_complete_ts: i64,
	provision_complete_ts: i64,
) -> GlobalResult<()> {
	let datacenters_res = op!([ctx] cluster_datacenter_get {
		datacenter_ids: vec![datacenter_id.into()],
	})
	.await?;
	let dc = unwrap!(datacenters_res.datacenters.first());

	let datacenter_id = unwrap_ref!(dc.datacenter_id).as_uuid().to_string();
	let cluster_id = unwrap_ref!(dc.cluster_id).as_uuid().to_string();
	let dt = (install_complete_ts - provision_complete_ts) as f64 / 1000.0;

	metrics::INSTALL_DURATION
		.with_label_values(&[
			cluster_id.as_str(),
			datacenter_id.as_str(),
			&dc.provider_datacenter_id,
			&dc.name_id,
			match pool_type {
				backend::cluster::PoolType::Job => "job",
				backend::cluster::PoolType::Gg => "gg",
				backend::cluster::PoolType::Ats => "ats",
			},
		])
		.observe(dt);

	Ok(())
}
