use anyhow::*;
use bolt_config::ns::ClusterKind;

use crate::{
	config,
	context::ProjectContext,
	dep::{salt, terraform},
	tasks,
	utils::command_helper::CommandHelper,
};

pub mod migrate;

pub struct ExecutePlanOpts {
	pub auto_approve: bool,
}

#[derive(Debug, Clone)]
pub enum PlanStep {
	Terraform {
		plan_id: String,

		/// If this plan needs to be destroyed when destroying all plans.
		///
		/// This should be false on plans that don't have side effects outside of
		/// provisioned infra. For example, plans that update configurations.
		///
		/// The purpose of this is to speed up the destroy step in CI.
		needs_destroy: bool,
	},
	Salt {
		filter: Option<String>,
		/// Filter which SLS files to apply.
		sls: Option<Vec<String>>,
		config_opts: salt::config::BuildOpts,
	},
	Migrate,
	Up,
}

impl PlanStep {
	async fn execute(&self, ctx: ProjectContext, opts: &ExecutePlanOpts) -> Result<()> {
		match self {
			PlanStep::Terraform { plan_id, .. } => {
				let varfile_path = ctx.gen_tf_env_path();

				let mut cmd = terraform::cli::build_command(&ctx, plan_id).await;
				cmd.arg("apply")
					.arg(format!("-var-file={}", varfile_path.display()))
					.arg("-parallelism=16");
				if opts.auto_approve {
					cmd.arg("-auto-approve");
				}
				cmd.exec().await?;

				terraform::output::clear_cache(&ctx, &plan_id).await;
			}
			PlanStep::Salt {
				filter,
				sls,
				config_opts,
			} => {
				let apply_opts = salt::cli::ApplyOpts {
					sls: (*sls).clone(),
					..Default::default()
				};
				if let Some(filter) = &filter {
					salt::cli::apply(&ctx, filter, &apply_opts, config_opts).await?;
				} else {
					salt::cli::apply_all(&ctx, &apply_opts, config_opts).await?;
				}
			}
			PlanStep::Migrate => {
				tasks::migrate::up_all(&ctx).await?;
			}
			PlanStep::Up => {
				tasks::up::up_all(
					&ctx,
					tasks::up::UpOpts {
						auto_approve: opts.auto_approve,
						..Default::default()
					},
				)
				.await?
			}
		}

		Ok(())
	}

	async fn destroy(&self, ctx: ProjectContext, opts: &ExecutePlanOpts) -> Result<()> {
		match self {
			PlanStep::Terraform {
				plan_id,
				needs_destroy,
			} => {
				if !needs_destroy {
					return Ok(());
				}

				let varfile_path = ctx.gen_tf_env_path();

				let mut cmd = terraform::cli::build_command(&ctx, plan_id).await;
				cmd.arg("destroy")
					.arg(format!("-var-file={}", varfile_path.display()))
					.arg("-parallelism=16");
				if opts.auto_approve {
					cmd.arg("-auto-approve");
				}
				cmd.exec().await?;

				terraform::output::clear_cache(&ctx, &plan_id).await;
			}
			PlanStep::Salt { .. } | PlanStep::Migrate | PlanStep::Up => {
				// Do nothing
			}
		}

		Ok(())
	}
}

pub fn build_plan(ctx: &ProjectContext) -> Result<Vec<PlanStep>> {
	let mut plan = Vec::new();

	// TLS
	plan.push(PlanStep::Terraform {
		plan_id: "tls".into(),
		needs_destroy: true,
	});

	// Nebula
	plan.push(PlanStep::Terraform {
		plan_id: "nebula".into(),
		needs_destroy: false,
	});

	// Master
	match ctx.ns().cluster.kind {
		ClusterKind::SingleNode { .. } => {
			plan.push(PlanStep::Terraform {
				plan_id: "master_local".into(),
				needs_destroy: false,
			});
		}
		ClusterKind::Distributed { .. } => {
			plan.push(PlanStep::Terraform {
				plan_id: "master_cluster".into(),
				needs_destroy: true,
			});
		}
	}

	// Pools
	plan.push(PlanStep::Terraform {
		plan_id: "pools".into(),
		needs_destroy: true,
	});

	// DNS
	plan.push(PlanStep::Terraform {
		plan_id: "dns".into(),
		needs_destroy: true,
	});

	// Cloudflare
	plan.push(PlanStep::Terraform {
		plan_id: "cloudflare_workers".into(),
		needs_destroy: true,
	});

	if let config::ns::DnsProvider::Cloudflare {
		access: Some(_), ..
	} = ctx.ns().dns.provider
	{
		plan.push(PlanStep::Terraform {
			plan_id: "cloudflare_tunnels".into(),
			needs_destroy: true,
		});
	}

	// Grafana
	if ctx.ns().grafana.is_some() {
		plan.push(PlanStep::Terraform {
			plan_id: "grafana".into(),
			needs_destroy: true,
		});
	}

	// S3
	match ctx.ns().s3.provider {
		config::ns::S3Provider::Minio { .. } => {
			// Install Minio for s3_minio Terraform plan
			plan.push(PlanStep::Salt {
				filter: Some("G@roles:minio".into()),
				sls: None,
				config_opts: salt::config::BuildOpts { skip_s3: true },
			});

			plan.push(PlanStep::Terraform {
				plan_id: "s3_minio".into(),
				needs_destroy: false,
			});
		}
		config::ns::S3Provider::Backblaze { .. } => {
			plan.push(PlanStep::Terraform {
				plan_id: "s3_backblaze".into(),
				needs_destroy: true,
			});
		}
	}

	// Apply the rest of the Salt configs
	plan.push(PlanStep::Salt {
		filter: None,
		sls: None,
		config_opts: Default::default(),
	});

	plan.push(PlanStep::Terraform {
		plan_id: "nomad".into(),
		needs_destroy: false,
	});

	plan.push(PlanStep::Migrate);

	plan.push(PlanStep::Up);

	Ok(plan)
}

/// List all of the Terraform plans in use for the generated plan.
pub fn all_terraform_plans(ctx: &ProjectContext) -> Result<Vec<String>> {
	let plan_ids = build_plan(ctx)?
		.into_iter()
		.flat_map(|x| {
			if let PlanStep::Terraform { plan_id, .. } = x {
				Some(plan_id)
			} else {
				None
			}
		})
		.collect::<Vec<_>>();

	Ok(plan_ids)
}

pub async fn execute_plan(
	ctx: &ProjectContext,
	plan: &[PlanStep],
	opts: ExecutePlanOpts,
) -> Result<()> {
	tasks::gen::generate_project(&ctx).await;

	for (i, step) in plan.iter().enumerate() {
		eprintln!();
		eprintln!();
		rivet_term::status::info("Executing", format!("({}/{}) {step:?}", i + 1, plan.len()));
		step.execute(ctx.clone(), &opts).await?;
	}

	Ok(())
}

pub async fn destroy_plan(
	ctx: &ProjectContext,
	plan: &[PlanStep],
	opts: ExecutePlanOpts,
) -> Result<()> {
	tasks::gen::generate_project(&ctx).await;

	for (i, step) in plan.iter().enumerate().rev() {
		eprintln!();
		eprintln!();
		rivet_term::status::info("Destroying", format!("({}/{}) {step:?}", i + 1, plan.len()));
		step.destroy(ctx.clone(), &opts).await?;
	}

	Ok(())
}
