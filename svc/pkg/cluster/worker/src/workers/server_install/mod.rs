use std::{
	io::{Read, Write},
	net::TcpStream,
	path::Path,
};

use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use ssh2::Session;

mod install_scripts;

// 6 months
pub const TOKEN_TTL: i64 = util::duration::days(30 * 6);

#[worker(name = "cluster-server-install", timeout = 200)]
async fn worker(ctx: &OperationContext<cluster::msg::server_install::Message>) -> GlobalResult<()> {
	// Check for stale message
	if ctx.req_dt() > util::duration::hours(1) {
		tracing::warn!("discarding stale message");

		return Ok(());
	}

	if let Some(server_id) = ctx.server_id {
		let (is_destroying,) = sql_fetch_one!(
			[ctx, (bool,)]
			"
			SELECT EXISTS(
				SELECT 1
				FROM db_cluster.servers
				WHERE server_id = $1 AND
				cloud_destroy_ts IS NOT NULL
			)
			",
			server_id.as_uuid(),
		)
		.await?;

		if is_destroying {
			tracing::info!("server marked for deletion, not installing");
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
			ttl: TOKEN_TTL,
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

	let install_script =
		install_scripts::gen_install(pool_type, ctx.initialize_immediately).await?;
	let hook_script = install_scripts::gen_hook(server_token).await?;
	let initialize_script =
		install_scripts::gen_initialize(pool_type, ctx.initialize_immediately, server_token)
			.await?;

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

	msg!([ctx] cluster::msg::server_install_complete(&ctx.public_ip) {
		public_ip: ctx.public_ip.clone(),
		datacenter_id: ctx.datacenter_id,
		server_id: ctx.server_id,
		provider: ctx.provider,
	})
	.await?;

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
