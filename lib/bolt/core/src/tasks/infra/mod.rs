use anyhow::*;

use tokio::time::Instant;

use crate::{
	config::ns, context::ProjectContext, dep::terraform, tasks,
	utils::command_helper::CommandHelper,
};

pub mod migrate;

pub struct ExecutePlanOpts {
	pub auto_approve: bool,
}

#[derive(Debug, Clone)]
pub struct PlanStep {
	pub name_id: &'static str,
	pub kind: PlanStepKind,
}

#[derive(Debug, Clone)]
pub enum PlanStepKind {
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
	Migrate,
	Up,
}

impl PlanStepKind {
	async fn execute(&self, ctx: ProjectContext, opts: &ExecutePlanOpts) -> Result<()> {
		// Generate the project before each step since things likely changed between steps
		tasks::gen::generate_project(&ctx, false).await;

		match self {
			PlanStepKind::Terraform { plan_id, .. } => {
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
			PlanStepKind::Migrate => {
				tasks::migrate::up_all(&ctx).await?;
			}
			PlanStepKind::Up => tasks::up::up_all(&ctx, false, false, false, false).await?,
		}

		Ok(())
	}

	async fn destroy(&self, ctx: ProjectContext, opts: &ExecutePlanOpts) -> Result<()> {
		match self {
			PlanStepKind::Terraform {
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
			PlanStepKind::Migrate | PlanStepKind::Up => {
				// Do nothing
			}
		}

		Ok(())
	}
}

pub fn build_plan(
	ctx: &ProjectContext,
	start_at: Option<String>,
	reverse: bool,
) -> Result<Vec<PlanStep>> {
	let mut plan = Vec::new();

	// Infra
	match ctx.ns().kubernetes.provider {
		ns::KubernetesProvider::K3d { .. } => {
			plan.push(PlanStep {
				name_id: "k8s-cluster-k3d",
				kind: PlanStepKind::Terraform {
					plan_id: "k8s_cluster_k3d".into(),
					needs_destroy: true,
				},
			});
		}
		ns::KubernetesProvider::AwsEks { .. } => {
			plan.push(PlanStep {
				name_id: "k8s-cluster-aws",
				kind: PlanStepKind::Terraform {
					plan_id: "k8s_cluster_aws".into(),
					needs_destroy: true,
				},
			});
		}
	}

	// Kubernetes
	plan.push(PlanStep {
		name_id: "k8s-infra",
		kind: PlanStepKind::Terraform {
			plan_id: "k8s_infra".into(),
			needs_destroy: false,
		},
	});

	if ctx.tls_enabled() {
		// TLS
		plan.push(PlanStep {
			name_id: "tls",
			kind: PlanStepKind::Terraform {
				plan_id: "tls".into(),
				needs_destroy: true,
			},
		});
	}

	// Redis
	match ctx.ns().redis.provider {
		ns::RedisProvider::Kubernetes {} => {
			plan.push(PlanStep {
				name_id: "redis-k8s",
				kind: PlanStepKind::Terraform {
					plan_id: "redis_k8s".into(),
					needs_destroy: false,
				},
			});
		}
		ns::RedisProvider::Aws { .. } => {
			plan.push(PlanStep {
				name_id: "redis-aws",
				kind: PlanStepKind::Terraform {
					plan_id: "redis_aws".into(),
					needs_destroy: true,
				},
			});
		}
		ns::RedisProvider::Aiven { .. } => {
			plan.push(PlanStep {
				name_id: "redis-aiven",
				kind: PlanStepKind::Terraform {
					plan_id: "redis_aiven".into(),
					needs_destroy: true,
				},
			});
		}
	}

	// CockroachDB
	match ctx.ns().cockroachdb.provider {
		ns::CockroachDBProvider::Kubernetes {} => {
			plan.push(PlanStep {
				name_id: "cockroachdb-k8s",
				kind: PlanStepKind::Terraform {
					plan_id: "cockroachdb_k8s".into(),
					needs_destroy: false,
				},
			});
		}
		ns::CockroachDBProvider::Managed { .. } => {
			plan.push(PlanStep {
				name_id: "cockroachdb-managed",
				kind: PlanStepKind::Terraform {
					plan_id: "cockroachdb_managed".into(),
					needs_destroy: true,
				},
			});
		}
	}

	// ClickHouse
	match ctx.ns().clickhouse.provider {
		ns::ClickHouseProvider::Kubernetes {} => {
			plan.push(PlanStep {
				name_id: "clickhouse-k8s",
				kind: PlanStepKind::Terraform {
					plan_id: "clickhouse_k8s".into(),
					needs_destroy: false,
				},
			});
		}
		ns::ClickHouseProvider::Managed { .. } => {
			plan.push(PlanStep {
				name_id: "clickhouse-managed",
				kind: PlanStepKind::Terraform {
					plan_id: "clickhouse_managed".into(),
					needs_destroy: true,
				},
			});
		}
	}

	// Vector
	plan.push(PlanStep {
		name_id: "vector",
		kind: PlanStepKind::Terraform {
			plan_id: "vector".into(),
			needs_destroy: false,
		},
	});

	// Pools
	if ctx.ns().dns.is_some() {
		plan.push(PlanStep {
			name_id: "pools",
			kind: PlanStepKind::Terraform {
				plan_id: "pools".into(),
				needs_destroy: true,
			},
		});
	}

	if let Some(dns) = &ctx.ns().dns {
		// TODO: Allow manual DNS config

		if let Some(ns::DnsProvider::Cloudflare { access, .. }) = &dns.provider {
			// DNS
			plan.push(PlanStep {
				name_id: "dns",
				kind: PlanStepKind::Terraform {
					plan_id: "dns".into(),
					needs_destroy: true,
				},
			});

			// Cloudflare
			plan.push(PlanStep {
				name_id: "cf-workers",
				kind: PlanStepKind::Terraform {
					plan_id: "cloudflare_workers".into(),
					needs_destroy: true,
				},
			});

			if access.is_some() {
				plan.push(PlanStep {
					name_id: "cf-tunnels",
					kind: PlanStepKind::Terraform {
						plan_id: "cloudflare_tunnels".into(),
						needs_destroy: true,
					},
				});
			}
		}
	}

	// BetterUptime
	if ctx.ns().better_uptime.is_some() {
		plan.push(PlanStep {
			name_id: "better_uptime",
			kind: PlanStepKind::Terraform {
				plan_id: "better_uptime".into(),
				needs_destroy: true,
			},
		});
	}

	// S3
	let s3_providers = &ctx.ns().s3.providers;
	if s3_providers.minio.is_some() {
		plan.push(PlanStep {
			name_id: "s3-minio",
			kind: PlanStepKind::Terraform {
				plan_id: "s3_minio".into(),
				needs_destroy: false,
			},
		});
	}
	if s3_providers.backblaze.is_some() {
		plan.push(PlanStep {
			name_id: "s3-backblaze",
			kind: PlanStepKind::Terraform {
				plan_id: "s3_backblaze".into(),
				needs_destroy: true,
			},
		});
	}
	if s3_providers.aws.is_some() {
		plan.push(PlanStep {
			name_id: "s3-aws",
			kind: PlanStepKind::Terraform {
				plan_id: "s3_aws".into(),
				needs_destroy: true,
			},
		});
	}

	plan.push(PlanStep {
		name_id: "infra-artifacts",
		kind: PlanStepKind::Terraform {
			plan_id: "infra_artifacts".into(),
			needs_destroy: false,
		},
	});

	plan.push(PlanStep {
		name_id: "migrate",
		kind: PlanStepKind::Migrate,
	});

	plan.push(PlanStep {
		name_id: "up",
		kind: PlanStepKind::Up,
	});

	// Start at the specified step
	if let Some(start_at) = start_at {
		let idx = plan
			.iter()
			.position(|x| x.name_id == start_at)
			.ok_or_else(|| anyhow!("invalid start_at value: {}", start_at))?;

		if reverse {
			plan = plan[..=idx].to_vec();
		} else {
			plan = plan[idx..].to_vec();
		}
	}

	Ok(plan)
}

/// List all of the Terraform plans in use for the generated plan.
pub fn all_terraform_plans(ctx: &ProjectContext) -> Result<Vec<String>> {
	let plan_ids = build_plan(ctx, None, false)?
		.into_iter()
		.flat_map(|x| {
			if let PlanStepKind::Terraform { plan_id, .. } = x.kind {
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
	tasks::gen::generate_project(&ctx, false).await;

	for (i, step) in plan.iter().enumerate() {
		eprintln!();
		eprintln!();

		rivet_term::status::info(
			"Executing step",
			format!("({}/{}) {}", i + 1, plan.len(), step.name_id),
		);
		let start = Instant::now();
		step.kind.execute(ctx.clone(), &opts).await?;
		rivet_term::status::progress(
			"Step complete",
			format!("{:.1}s", start.elapsed().as_secs_f32()),
		);
	}

	Ok(())
}

pub async fn destroy_plan(
	ctx: &ProjectContext,
	plan: &[PlanStep],
	opts: ExecutePlanOpts,
) -> Result<()> {
	tasks::gen::generate_project(&ctx, false).await;

	for (i, step) in plan.iter().enumerate().rev() {
		eprintln!();
		eprintln!();
		rivet_term::status::info(
			"Destroying",
			format!("({}/{}) {}", i + 1, plan.len(), step.name_id),
		);
		step.kind.destroy(ctx.clone(), &opts).await?;
	}

	Ok(())
}
