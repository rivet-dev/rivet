use std::{str::FromStr, time::Duration};

use anyhow::{Context, Result, bail, ensure};
use clap::Parser;
use gas::{
	db::{Database as _, DatabaseKv},
	prelude::{Id, StandaloneCtx},
};
use rand::RngCore;

#[derive(Parser)]
pub enum SubCommand {
	/// Manage JWT signing keys
	Jwt {
		#[clap(subcommand)]
		command: JwtSubCommand,
	},
}

#[derive(Parser)]
pub enum JwtSubCommand {
	/// Show the authoritative public key ring and signing deadlines
	Keys,
	/// Stage a generation-fenced normal rotation with the configured publication lead
	Rotate {
		#[arg(long)]
		expected_generation: u64,
		/// Confirm the operational change
		#[arg(long)]
		yes: bool,
	},
	/// Immediately replace the active signer and revoke named public keys
	RevokeKey {
		/// Key ID to revoke. Repeat to revoke multiple keys.
		#[arg(long, required = true)]
		kid: Vec<String>,
		#[arg(long)]
		expected_generation: u64,
		/// Confirm the disruptive emergency change
		#[arg(long)]
		yes: bool,
	},
}

impl SubCommand {
	pub async fn execute(self, config: rivet_config::Config) -> Result<()> {
		let Self::Jwt { command } = self;
		command.execute(config).await
	}
}

impl JwtSubCommand {
	async fn execute(self, config: rivet_config::Config) -> Result<()> {
		let ctx = standalone_ctx(&config).await?;
		match self {
			Self::Keys => print_status(read_status(&ctx).await?),
			Self::Rotate {
				expected_generation,
				yes,
			} => {
				ensure!(yes, "pass --yes to confirm JWT key rotation");
				preflight_generation(&ctx, expected_generation).await?;
				ctx.op(rivet_auth_jwt::ops::rotation::request_normal::Input {
					expected_generation,
				})
				.await?;
				let status = wait_for_status(&ctx, |status| {
					status.generation > expected_generation && status.pending.is_some()
				})
				.await?;
				println!("Normal JWT rotation staged.");
				print_status(status)
			}
			Self::RevokeKey {
				kid,
				expected_generation,
				yes,
			} => {
				ensure!(yes, "pass --yes to confirm emergency JWT key revocation");
				preflight_generation(&ctx, expected_generation).await?;
				let revoke_kids = kid
					.iter()
					.map(|kid| rivet_auth_jwt::KeyId::from_str(kid))
					.collect::<std::result::Result<Vec<_>, _>>()
					.context("invalid JWT key id")?;
				let mut request_id = [0; 16];
				rand::rngs::OsRng.fill_bytes(&mut request_id);
				ctx.op(rivet_auth_jwt::ops::rotation::request_emergency::Input {
					request_id,
					expected_generation,
					revoke_kids,
				})
				.await?;
				let status = wait_for_status(&ctx, |status| {
					status
						.last_emergency_receipt
						.is_some_and(|(receipt_id, _)| receipt_id == request_id)
				})
				.await?;
				println!("Emergency JWT key revocation committed.");
				print_status(status)
			}
		}
	}
}

async fn standalone_ctx(config: &rivet_config::Config) -> Result<StandaloneCtx> {
	let pools = rivet_pools::Pools::new(config.clone()).await?;
	let cache = rivet_cache::CacheInner::from_env(config, pools.clone())?;
	StandaloneCtx::new(
		DatabaseKv::new(config.clone(), pools.clone()).await?,
		config.clone(),
		pools,
		cache,
		"auth_jwt_cli",
		Id::new_v1(config.dc_label()),
		Id::new_v1(config.dc_label()),
	)
	.map_err(Into::into)
}

async fn read_status(ctx: &StandaloneCtx) -> Result<rivet_auth_jwt::ops::rotation::status::Output> {
	ctx.op(rivet_auth_jwt::ops::rotation::status::Input)
		.await?
		.context("JWT key ring is not initialized")
}

async fn preflight_generation(ctx: &StandaloneCtx, expected_generation: u64) -> Result<()> {
	let status = read_status(ctx).await?;
	ensure!(
		status.generation == expected_generation,
		"JWT key-ring generation changed: expected {expected_generation}, authoritative generation is {}",
		status.generation
	);
	Ok(())
}

async fn wait_for_status(
	ctx: &StandaloneCtx,
	done: impl Fn(&rivet_auth_jwt::ops::rotation::status::Output) -> bool,
) -> Result<rivet_auth_jwt::ops::rotation::status::Output> {
	let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
	loop {
		let status = read_status(ctx).await?;
		if done(&status) {
			return Ok(status);
		}
		if tokio::time::Instant::now() >= deadline {
			bail!("timed out waiting for JWT key-rotation workflow");
		}
		tokio::time::sleep(Duration::from_millis(500)).await;
	}
}

fn print_status(status: rivet_auth_jwt::ops::rotation::status::Output) -> Result<()> {
	println!("generation: {}", status.generation);
	println!("JWT enabled: {}", status.jwt_enabled);
	println!("issuance enabled: {}", status.issuance_enabled);
	println!("desired issuer: {}", status.desired_issuer);
	println!(
		"issuer generation: configured={} active={}",
		status.configured_issuer_generation, status.active_issuer_generation
	);
	println!("active issuer: {}", status.active_issuer);
	println!(
		"issuer configuration valid: {}",
		status.issuer_configuration_valid
	);
	for issuer in &status.retiring_issuers {
		println!(
			"retiring issuer: {} (accepted through {})",
			issuer.issuer,
			format_timestamp(issuer.accept_until_ts)
		);
	}
	println!(
		"leader: datacenter={} desired_datacenter={} epoch={} config_generation={}",
		status.leader_datacenter_id,
		status.desired_leader_datacenter_id,
		status.leader_epoch,
		status.leader_config_generation
	);
	println!("active kid: {}", status.active.kid);
	println!(
		"normal rotation due: {}",
		format_timestamp(status.active_rotation_due_ts)
	);
	println!(
		"hard signing deadline: {}",
		format_timestamp(status.active_hard_signing_deadline_ts)
	);
	if let Some((pending, activate_after_ts)) = status.pending {
		println!("pending kid: {}", pending.kid);
		println!(
			"pending activation: {}",
			format_timestamp(activate_after_ts)
		);
	} else {
		println!("pending kid: none");
	}
	for (retiring, verify_until_ts) in status.retiring {
		println!(
			"retiring kid: {} (verify through {})",
			retiring.kid,
			format_timestamp(verify_until_ts)
		);
	}
	Ok(())
}

fn format_timestamp(timestamp_ms: i64) -> String {
	chrono::DateTime::from_timestamp_millis(timestamp_ms)
		.map(|timestamp| timestamp.to_rfc3339())
		.unwrap_or_else(|| timestamp_ms.to_string())
}
