use anyhow::Result;
use duct::cmd;
use serde::Deserialize;
use std::path::Path;
use tokio::task::block_in_place;

use crate::{
	config, context::ProjectContext, dep::terraform, tasks, tasks::ssh::TempSshKey, utils,
};

#[derive(Clone, Debug, Default)]
pub struct ApplyOpts {
	pub verbose: bool,
	pub sls: Option<Vec<String>>,
}

pub async fn apply_all(
	ctx: &ProjectContext,
	opts: &ApplyOpts,
	config_opts: &super::config::BuildOpts,
) -> Result<()> {
	apply(ctx, "*", opts, config_opts).await
}

pub async fn apply(
	ctx: &ProjectContext,
	filter: &str,
	opts: &ApplyOpts,
	config_opts: &super::config::BuildOpts,
) -> Result<()> {
	let mut event = utils::telemetry::build_event(ctx, "bolt_salt_apply").await?;
	event.insert_prop("filter", filter)?;
	event.insert_prop("sls", &opts.sls)?;
	utils::telemetry::capture_event(ctx, event).await?;

	// Write Salt configs
	eprintln!();
	rivet_term::status::progress("Writing configs", "");
	match ctx.ns().cluster.kind {
		config::ns::ClusterKind::SingleNode { .. } => {
			// Write salt
			block_in_place(|| cmd!("rm", "-rf", "/srv/salt", "/srv/pillar").run())?;
			block_in_place(|| cmd!("cp", "-r", ctx.salt_path().join("salt"), "/srv/salt").run())?;
			block_in_place(|| {
				cmd!("cp", "-r", ctx.salt_path().join("pillar"), "/srv/pillar").run()
			})?;
			block_in_place(|| {
				cmd!(
					"cp",
					"-r",
					ctx.path().join("infra").join("nix"),
					"/srv/salt/nix/files/source"
				)
				.run()
			})?;

			// Write salt context
			gen_salt_context(ctx, config_opts, Path::new("/srv/salt-context")).await?;
		}
		config::ns::ClusterKind::Distributed { .. } => {
			tokio::try_join!(
				async {
					// /srv/salt
					rsync_dir(ctx, &ctx.salt_path().join("salt"), "/srv/salt").await?;

					// /srv/salt/nix/files/source
					rsync_dir(
						ctx,
						&ctx.path().join("infra").join("nix"),
						"/srv/salt/nix/files/source",
					)
					.await?;

					Ok(())
				},
				// /srv/pillar
				async { rsync_dir(ctx, &ctx.salt_path().join("pillar"), "/srv/pillar").await },
				// /srv/salt-context
				async {
					let tmp_dir = tempfile::TempDir::new()?;
					gen_salt_context(ctx, config_opts, tmp_dir.path()).await?;
					rsync_dir(ctx, tmp_dir.path(), "/srv/salt-context").await?;
					Ok(())
				}
			)?;
		}
	}

	// Refresh pillars
	eprintln!();
	rivet_term::status::progress("Refreshing pillars", "saltutil.refresh_pillar");
	match ctx.ns().cluster.kind {
		config::ns::ClusterKind::SingleNode { .. } => {
			block_in_place(|| cmd!("salt", "-C", filter, "saltutil.refresh_pillar").run())?;
		}
		config::ns::ClusterKind::Distributed { .. } => {
			exec_master_cmd(ctx, &format!("salt -C '{filter}' saltutil.refresh_pillar")).await?
		}
	}

	// Update mines
	eprintln!();
	rivet_term::status::progress("Updating mines", "mine.update");
	match ctx.ns().cluster.kind {
		config::ns::ClusterKind::SingleNode { .. } => {
			block_in_place(|| cmd!("salt", "-C", filter, "mine.update").run())?;
		}
		config::ns::ClusterKind::Distributed { .. } => {
			exec_master_cmd(ctx, &format!("salt -C '{filter}' mine.update")).await?
		}
	}

	// Apply state
	eprintln!();
	rivet_term::status::progress("Applying state", "state.apply");
	let extra_flags = if opts.verbose {
		"--state-output=full"
	} else {
		"--state-output=terse"
	};
	match ctx.ns().cluster.kind {
		config::ns::ClusterKind::SingleNode { .. } => {
			if let Some(sls) = &opts.sls {
				block_in_place(|| {
					cmd!(
						"salt",
						extra_flags,
						"-C",
						filter,
						"state.apply",
						sls.join(",")
					)
					.run()
				})?;
			} else {
				block_in_place(|| cmd!("salt", extra_flags, "-C", filter, "state.apply").run())?;
			}
		}
		config::ns::ClusterKind::Distributed { .. } => {
			if let Some(sls) = &opts.sls {
				exec_master_cmd(
					ctx,
					&format!(
						"salt {extra_flags} -C '{filter}' state.apply '{}'",
						sls.join(",")
					),
				)
				.await?
			} else {
				exec_master_cmd(
					ctx,
					&format!("salt {extra_flags} -C '{filter}' state.apply"),
				)
				.await?
			}
		}
	}

	eprintln!();
	rivet_term::status::progress("Applied", "");

	Ok(())
}

/// JSON structure to read from Terraform plan.
#[derive(Deserialize)]
struct TerraformSaltOutput {
	salt_output: Option<terraform::output::TerraformOutputValue<serde_json::Value>>,
}

/// Generates config files from Bolt for use within Salt configs.
async fn gen_salt_context(
	ctx: &ProjectContext,
	config_opts: &super::config::BuildOpts,
	dir: &Path,
) -> Result<()> {
	// Clear folder
	block_in_place(|| cmd!("rm", "-rf", dir).run())?;
	block_in_place(|| cmd!("mkdir", "-p", dir.join("terraform"), dir.join("rivet"),).run())?;

	// Write Rivet config
	let config = super::config::build(ctx, config_opts).await?;
	let config_buf = serde_json::to_vec(&config)?;
	tokio::fs::write(dir.join("rivet/config.json"), config_buf).await?;

	// Write Rivet secrets
	let secrets = super::secrets::build_secrets(ctx).await?;
	let secrets_buf = serde_json::to_vec(&secrets)?;
	tokio::fs::write(dir.join("rivet/secrets.json"), secrets_buf).await?;

	// Write Terraform outputs
	for plan_id in crate::tasks::infra::all_terraform_plans(ctx)? {
		let plan = terraform::output::read_plan::<TerraformSaltOutput>(ctx, &plan_id).await;
		let json_buf = serde_json::to_vec(&plan.salt_output.map(|x| x.value))?;
		tokio::fs::write(dir.join(format!("terraform/{plan_id}.json")), json_buf).await?;
	}

	Ok(())
}

/// Sync a directory to the Salt master.
async fn rsync_dir(ctx: &ProjectContext, src: &Path, dst: &str) -> Result<()> {
	let master_ip = get_master_ip(ctx).await?;
	let ssh_key = TempSshKey::new(&ctx, "salt_master").await?;
	block_in_place(|| {
		cmd!(
			"rsync",
			"-e",
			format!("ssh -i {}", ssh_key.path().display()),
			"-Pav",
			"--delete",
			format!("{}/", src.display()),
			format!("root@{master_ip}:{dst}")
		)
		// .stdout_null()
		.run()
	})?;

	Ok(())
}

/// Executes a command on the Salt master.
async fn exec_master_cmd(ctx: &ProjectContext, command: &str) -> Result<()> {
	tasks::ssh::pool(&ctx, "salt_master", Some(command)).await?;

	Ok(())
}

async fn get_master_ip(ctx: &ProjectContext) -> Result<String> {
	let tf_master_cluster = terraform::output::read_master_cluster(&ctx).await;

	Ok(tf_master_cluster.salt_master_host.value)
}
