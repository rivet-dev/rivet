use anyhow::Result;
use serde_json::json;
use std::path::{Path, PathBuf};

use crate::{
	config::{
		self,
		ns::LoggingProvider,
		service::{ServiceDomain, ServiceKind, ServiceRouter},
	},
	context::{BuildContext, ProjectContext, RunContext, S3Provider, ServiceContext},
	dep::nomad::job_schema::*,
};

pub struct ExecServiceContext {
	pub svc_ctx: ServiceContext,
	pub build_context: BuildContext,
	pub driver: ExecServiceDriver,
}

pub enum ExecServiceDriver {
	Docker {
		image: String,
		force_pull: bool,
	},
	UploadedBinaryArtifact {
		artifact_key: String,
		/// Path to the executable within the archive.
		exec_path: String,
		args: Vec<String>,
	},
	LocalBinaryArtifact {
		/// Path to the executable relative to the project root.
		exec_path: PathBuf,
		args: Vec<String>,
	},
}

/// Generates a job definition for services for the development Nomad cluster.
pub async fn gen_svc(region_id: &str, exec_ctx: &ExecServiceContext) -> Job {
	let ExecServiceContext {
		svc_ctx,
		build_context: _,
		driver,
	} = exec_ctx;

	let project_ctx = svc_ctx.project().await;

	let node_class = match project_ctx.ns().cluster.kind {
		config::ns::ClusterKind::SingleNode { .. } => "local",
		config::ns::ClusterKind::Distributed { .. } => "svc",
	};

	let (job_type, enable_update, reschedule_attempts) = match svc_ctx.config().kind {
		ServiceKind::Headless { .. }
		| ServiceKind::Consumer { .. }
		| ServiceKind::Api { .. }
		| ServiceKind::Static { .. } => ("service", true, 0),
		ServiceKind::Oneshot { .. } | ServiceKind::Periodic { .. } => ("batch", false, 1),
		ServiceKind::Operation { .. }
		| ServiceKind::Database { .. }
		| ServiceKind::Cache { .. } => {
			unreachable!()
		}
	};

	let service_name = format!("rivet-{}", svc_ctx.name());
	let service_tags = vec![
		"rivet".into(),
		svc_ctx.name(),
		format!("service-{}", svc_ctx.config().kind.short()),
		format!("runtime-{}", svc_ctx.config().runtime.short()),
	];

	let has_health = project_ctx.ns().nomad.health_checks.unwrap_or_else(|| {
		match project_ctx.ns().cluster.kind {
			config::ns::ClusterKind::SingleNode { .. } => false,
			config::ns::ClusterKind::Distributed { .. } => true,
		}
	}) && matches!(
		svc_ctx.config().kind,
		ServiceKind::Headless { .. } | ServiceKind::Consumer { .. } | ServiceKind::Api { .. }
	);

	let has_metrics = matches!(
		svc_ctx.config().kind,
		ServiceKind::Headless { .. } | ServiceKind::Consumer { .. } | ServiceKind::Api { .. }
	);

	// Render env
	let (env, forward_services) = svc_ctx.env(RunContext::Service).await.unwrap();
	assert!(
		forward_services.is_empty(),
		"should not forward services for RunContext::Service"
	);

	// Render ports
	let (dynamic_ports, reserved_ports) = {
		let mut reserved_ports = vec![];
		let mut ports = Vec::new();

		ports.push(DynamicPort {
			label: Some("health".into()),
			to: Some(-1),
			..Default::default()
		});

		if has_metrics {
			ports.push(DynamicPort {
				label: Some("metrics".into()),
				to: Some(-1),
				..Default::default()
			});
		};

		if svc_ctx.enable_tokio_console() {
			ports.push(DynamicPort {
				label: Some("tokio-console".into()),
				to: Some(-1),
				..Default::default()
			});
		}

		if svc_ctx.config().kind.has_server() {
			if let ServiceKind::Api {
				port: Some(port), ..
			} = svc_ctx.config().kind
			{
				reserved_ports.push(json!({
					"Label": "http",
					"Value": port,
				}));
			} else {
				ports.push(DynamicPort {
					label: Some("http".into()),
					..Default::default()
				});
			}
		}

		(ports, reserved_ports)
	};
	let docker_ports = dynamic_ports
		.iter()
		.map(|x| x.label.clone().unwrap())
		.chain(
			reserved_ports
				.iter()
				.map(|x| x.get("Label").unwrap().as_str().unwrap().to_string()),
		)
		.collect::<Vec<String>>();

	let ns_service_config = svc_ctx.ns_service_config().await;

	// TODO: app.kubernetes.io/managed-by
	json!({})
}

mod chrono {
	pub fn min(x: i64) -> i64 {
		secs(x * 60)
	}

	pub fn secs(x: i64) -> i64 {
		millis(x * 1_000)
	}

	pub fn millis(x: i64) -> i64 {
		x * 1_000_000
	}
}

/// Define check restart config for connection health checks. On local, we want
/// the service to fail immediately so we can get immediate feedback if
/// something is wrong.
fn build_conn_check(name: &str, path: &str, interval: i64, on_update: &str) -> Check {
	// This check only affects Consul traffic. We don't restart the service here
	// because it would cause cascading failures if a database goes down.
	//
	// We need to check services frequently in order to quickly reroute traffic
	// to healthy nodes.

	Check {
		name: Some(name.into()),
		check_type: Some("http".into()),
		port_label: Some("health".into()),
		path: Some(path.into()),
		interval: Some(interval),
		timeout: Some(chrono::secs(5)),
		on_update: Some(on_update.into()),
		..Default::default()
	}
}

async fn build_region_config_template(
	ctx: &ProjectContext,
	templates: &mut Vec<serde_json::Value>,
) {
	let regions_json = serde_json::to_string(&ctx.ns().regions).unwrap();

	templates.push(json!({
		"DestPath": format!("local/region-config.json"),
		"EmbeddedTmpl": Some(regions_json),
	}));
}

/// Generates service tags that are provided to Treafik to determine how to route traffic to the
/// service.
///
/// See [here](https://doc.traefik.io/traefik/providers/nomad/) for more details.
fn build_router_service_tags(
	project_ctx: &ProjectContext,
	svc_ctx: &ServiceContext,
	router: &ServiceRouter,
	tags: &mut Vec<String>,
) {
	// Enable Traefik if there are mounts
	if !router.mounts.is_empty() {}

	// Register all mounts with Traefik
	for (i, mount) in router.mounts.iter().enumerate() {
		// Determine which router to mount on
		for prefix in ["traefik-ing-px", "traefik-local"] {
			// Enable if not already
			let enable_tag = format!("{prefix}.enable=true");
			if !tags.contains(&enable_tag) {
				tags.push(enable_tag);
			}

			// Derive routing info
			let domain = match mount.domain {
				ServiceDomain::Base => project_ctx.domain_main(),
				ServiceDomain::BaseGame => project_ctx.domain_cdn(),
				ServiceDomain::BaseJob => project_ctx.domain_job(),
			};
			let host = if let Some(subdomain) = &mount.subdomain {
				format!("{}.{}", subdomain, domain)
			} else {
				domain
			};

			// Build rule
			let mut rule = format!("Host(`{host}`)");
			if let Some(path) = &mount.path {
				rule.push_str(&format!(" && PathPrefix(`{path}`)"));
			}

			// Build middlewares
			let mut middlewares = Vec::new();

			// Strip prefix
			if let Some(path) = &mount.path {
				let mw_name = format!("{}-{i}-strip-prefix", svc_ctx.name());
				middlewares.push(mw_name.clone());
				tags.extend([
					format!("{prefix}.http.middlewares.{mw_name}.stripPrefix.prefixes={path}"),
					format!("{prefix}.http.middlewares.{mw_name}.stripPrefix.forceSlash=true"),
				]);
			}

			// Compress
			{
				let mw_name = format!("{}-{i}-compress", svc_ctx.name());
				middlewares.push(mw_name.clone());
				tags.extend([format!("{prefix}.http.middlewares.{mw_name}.compress=true")]);
			}

			// In flight
			{
				let mw_name = format!("{}-{i}-inflight", svc_ctx.name());
				middlewares.push(mw_name.clone());
				tags.extend([format!(
					"{prefix}.http.middlewares.{mw_name}.inflightreq.amount=64"
				)]);
				tags.extend([format!("{prefix}.http.middlewares.{mw_name}.inflightreq.sourcecriterion.requestheadername=cf-connecting-ip")]);
			}

			// Build router
			let r_name = format!("{}-{i}", svc_ctx.name());
			tags.extend([
				format!("{prefix}.http.routers.{r_name}.entryPoints=lb-443"),
				format!("{prefix}.http.routers.{r_name}.rule={rule}"),
				format!(
					"{prefix}.http.routers.{r_name}.middlewares={}",
					middlewares.join(", ")
				),
				format!("{prefix}.http.routers.{r_name}.tls=true"),
			]);

			if svc_ctx.name() == "api-cf-verification" {
				tags.extend([
				format!("{prefix}.http.routers.cf-verification-challenge.rule=PathPrefix(`/.well-known/cf-custom-hostname-challenge/`)"),
				format!("{prefix}.http.routers.cf-verification-challenge.priority=90"),
			]);
			}
		}
	}
}

async fn nomad_docker_io_auth(ctx: &ProjectContext) -> Result<serde_json::Value> {
	if ctx.ns().docker.authenticate_all_docker_hub_pulls {
		let username = ctx
			.read_secret(&["docker", "docker_io", "username"])
			.await?;
		let password = ctx
			.read_secret(&["docker", "docker_io", "password"])
			.await?;
		Ok(json!({
			"username": username,
			"password": password,
		}))
	} else {
		Ok(json!(null))
	}
}

async fn nomad_loki_plugin_config(
	ctx: &ProjectContext,
	svc_ctx: &ServiceContext,
) -> Result<serde_json::Value> {
	match &ctx.ns().logging.as_ref().map(|x| &x.provider) {
		Some(LoggingProvider::Loki { .. }) => {
			// Create log labels
			let labels = [
				("ns", ctx.ns_id()),
				("service", &format!("rivet-{}", svc_ctx.name())),
				("node", "${node.unique.name}"),
				("alloc", "${NOMAD_ALLOC_ID}"),
				("dc", "${NOMAD_DC}"),
			]
			.iter()
			.map(|(k, v)| format!("{k}={v}"))
			.collect::<Vec<_>>()
			.join(",");

			// Remove default log labels
			let relabel_config = json!([
				{
					"action": "labeldrop",
					"regex": "^(host|filename)$",
				},
			]);

			Ok(json!({
				"type": "loki",
				"config": [{
					"loki-url": "http://127.0.0.1:9060/loki/api/v1/push",
					"loki-retries": 5,
					"loki-batch-size": 400,
					"no-file": true,
					"loki-external-labels": labels,
					"loki-relabel-config": serde_json::to_string(&relabel_config)?,
				}],
			}))
		}
		None => Ok(json!(null)),
	}
}
