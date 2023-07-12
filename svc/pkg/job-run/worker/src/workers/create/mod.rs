use std::collections::HashMap;

use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

mod create_job;

// TODO: Reduce disk space for allocations

const MAX_PARAMETER_KEY_LEN: usize = 64;
const MAX_PARAMETER_VALUE_LEN: usize = 8_192; // 8 KB

lazy_static::lazy_static! {
	static ref NOMAD_CONFIG: nomad_client::apis::configuration::Configuration =
		nomad_util::config_from_env().unwrap();
}

#[tracing::instrument]
async fn fail(
	client: &chirp_client::Client,
	run_id: Uuid,
	error_code: job_run::msg::fail::ErrorCode,
) -> GlobalResult<()> {
	tracing::warn!(%run_id, ?error_code, "job run fail");

	msg!([client] job_run::msg::fail(run_id) {
		run_id: Some(run_id.into()),
		error_code: error_code as i32,
	})
	.await?;

	Ok(())
}

#[worker(name = "job-run-create")]
async fn worker(ctx: &OperationContext<job_run::msg::create::Message>) -> Result<(), GlobalError> {
	let crdb = ctx.crdb("db-job-state").await?;

	let run_id = internal_unwrap!(ctx.run_id).as_uuid();
	let region_id = internal_unwrap!(ctx.region_id).as_uuid();

	// Check for stale message
	if ctx.req_dt() > util::duration::seconds(60) {
		tracing::warn!("discarding stale message");

		return fail(
			ctx.chirp(),
			run_id,
			job_run::msg::fail::ErrorCode::StaleMessage,
		)
		.await;
	}

	// Validate the parameter data lengths
	for parameter in &ctx.parameters {
		internal_assert!(
			parameter.key.len() <= MAX_PARAMETER_KEY_LEN,
			"parameter key too long"
		);
		internal_assert!(
			parameter.value.len() <= MAX_PARAMETER_VALUE_LEN,
			"parameter value too long"
		);
	}

	// Get the region to dispatch in
	let region_res = op!([ctx] region_get {
		region_ids: vec![region_id.into()],
	})
	.await?;
	let region = internal_unwrap_owned!(region_res.regions.first());

	// Create the job
	let create_job_perf = ctx.perf().start("create-job").await;
	let nomad_job_id = create_job::create_job(&ctx.job_spec_json, region).await?;
	create_job_perf.end();

	// Create a token for the run
	let (job_run_token, job_run_token_session_id) = create_run_token(ctx, run_id).await?;

	// Write to the database before doing anything
	let db_write_perf = ctx.perf().start("write-to-db-before-run").await;
	rivet_pools::utils::crdb::tx(&crdb, |tx| {
		Box::pin(write_to_db_before_run(
			tx,
			ctx.body().clone(),
			ctx.ts(),
			region_id,
			run_id,
			job_run_token_session_id,
		))
	})
	.await?;
	db_write_perf.end();

	// Run the job
	let run_job_perf = ctx.perf().start("run-job").await;
	let nomad_dispatched_job_id = run_job(
		ctx.body(),
		run_id,
		job_run_token,
		&nomad_job_id,
		&region.nomad_region,
	)
	.await?;
	let nomad_dispatched_job_id = if let Some(x) = nomad_dispatched_job_id {
		x
	} else {
		// Cleanup the job
		msg!([ctx] job_run::msg::stop(run_id) {
			run_id: Some(run_id.into()),
			..Default::default()
		})
		.await?;

		return fail(
			ctx.chirp(),
			run_id,
			job_run::msg::fail::ErrorCode::NomadDispatchFailed,
		)
		.await;
	};
	run_job_perf.end();

	let db_write_perf = ctx.perf().start("write-to-db-after-run").await;
	write_to_db_after_run(&crdb, run_id, &nomad_dispatched_job_id).await?;
	db_write_perf.end();

	msg!([ctx] job_run::msg::create_complete(run_id) {
		run_id: Some(run_id.into()),
	})
	.await?;

	msg!([ctx] job_run::msg::nomad_dispatched_job(run_id, &nomad_dispatched_job_id) {
		run_id: Some(run_id.into()),
		dispatched_job_id: nomad_dispatched_job_id.clone(),
	})
	.await?;

	Ok(())
}

#[tracing::instrument(skip(req))]
async fn run_job(
	req: &job_run::msg::create::Message,
	run_id: Uuid,
	job_run_token: String,
	nomad_job_id: &str,
	nomad_region: &str,
) -> GlobalResult<Option<String>> {
	let job_params: Vec<(String, String)> = vec![
		("job_run_id".into(), run_id.to_string()),
		("job_run_token".into(), job_run_token),
	];
	let dispatch_res = nomad_client::apis::jobs_api::dispatch_job(
		&NOMAD_CONFIG,
		nomad_job_id,
		None,
		Some(nomad_region),
		None,
		None,
		Some(nomad_client::models::JobDispatchRequest {
			payload: None,
			meta: Some(
				req.parameters
					.iter()
					.map(|p| (p.key.clone(), p.value.clone()))
					.chain(job_params.into_iter())
					.collect::<HashMap<String, String>>(),
			),
		}),
	)
	.await;
	match dispatch_res {
		Ok(dispatch_res) => {
			// We will use the dispatched job ID to identify this allocation for the future. We can't use
			// eval ID, since that changes if we mutate the allocation (i.e. try to stop it).
			let nomad_dispatched_job_id = internal_unwrap!(dispatch_res.dispatched_job_id);
			Ok(Some(nomad_dispatched_job_id.clone()))
		}
		Err(err) => {
			tracing::error!(?err, "failed to dispatch job");
			Ok(None)
		}
	}
}

/// Creates a token that is passed to the job used to shut down the job.
#[tracing::instrument]
async fn create_run_token(
	ctx: &OperationContext<job_run::msg::create::Message>,
	run_id: Uuid,
) -> GlobalResult<(String, Uuid)> {
	let token_res = op!([ctx] token_create {
		issuer: "job-run-create".into(),
		token_config: Some(token::create::request::TokenConfig {
			ttl: util::duration::days(365),
		}),
		refresh_token_config: None,
		client: None,
		kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
			entitlements: vec![
				proto::claims::Entitlement {
					kind: Some(
						proto::claims::entitlement::Kind::JobRun(proto::claims::entitlement::JobRun {
							run_id: Some(run_id.into()),
						})
					)
				}
			],
		})),
		label: Some("jr".into()),
		..Default::default()
	})
	.await?;

	let token = internal_unwrap!(token_res.token).token.clone();
	let token_session_id = internal_unwrap!(token_res.session_id).as_uuid();
	Ok((token, token_session_id))
}

#[tracing::instrument(skip_all)]
async fn write_to_db_before_run(
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	req: job_run::msg::create::Message,
	ts: i64,
	region_id: Uuid,
	run_id: Uuid,
	token_session_id: Uuid,
) -> GlobalResult<()> {
	sqlx::query(indoc!(
		"
		INSERT INTO runs (run_id, region_id, create_ts, token_session_id)
		VALUES ($1, $2, $3, $4)
		"
	))
	.bind(run_id)
	.bind(region_id)
	.bind(ts)
	.bind(token_session_id)
	.execute(&mut *tx)
	.await?;

	sqlx::query(indoc!("INSERT INTO run_meta_nomad (run_id) VALUES ($1)"))
		.bind(run_id)
		.execute(&mut *tx)
		.await?;

	// Validate that the proxied ports point to existing ports
	for proxied_port in &req.proxied_ports {
		internal_assert!(
			!proxied_port.ingress_hostnames.is_empty(),
			"ingress host not provided"
		);

		for host in &proxied_port.ingress_hostnames {
			internal_assert!(
				host.chars()
					.all(|x| x.is_alphanumeric() || x == '.' || x == '-'),
				"invalid ingress host"
			);
		}

		let ingress_port = choose_ingress_port(proxied_port)?;

		tracing::info!(?run_id, ?proxied_port, "inserting run proxied port");

		let mut ingress_hostnames_sorted = proxied_port.ingress_hostnames.clone();
		ingress_hostnames_sorted.sort();

		sqlx::query(indoc!(
			"
					INSERT INTO run_proxied_ports (
						run_id,
						target_nomad_port_label,
						ingress_port,
						ingress_hostnames,
						ingress_hostnames_str,
						proxy_protocol,
						ssl_domain_mode
					)
					VALUES ($1, $2, $3, $4, $5, $6, $7)
					"
		))
		.bind(run_id)
		.bind(proxied_port.target_nomad_port_label.clone())
		.bind(ingress_port)
		.bind(&ingress_hostnames_sorted)
		.bind(ingress_hostnames_sorted.join(","))
		.bind(proxied_port.proxy_protocol)
		.bind(proxied_port.ssl_domain_mode)
		.execute(&mut *tx)
		.await?;
	}

	Ok(())
}

#[tracing::instrument(skip(crdb))]
async fn write_to_db_after_run(
	crdb: &CrdbPool,
	run_id: Uuid,
	dispatched_job_id: &str,
) -> GlobalResult<()> {
	sqlx::query("UPDATE run_meta_nomad SET dispatched_job_id = $2 WHERE run_id = $1")
		.bind(run_id)
		.bind(dispatched_job_id)
		.execute(crdb)
		.await?;

	Ok(())
}

/// Choose which port to assign for a job's ingress port.
///
/// If not provided by `ProxiedPort`, then:
/// - HTTP: 80
/// - HTTPS: 443
/// - TCP/TLS: random
/// - UDP: random
///
/// This is very poorly written for TCP & UDP ports and will bite us in the ass
/// some day. See https://linear.app/rivet-gg/issue/RIV-1799
fn choose_ingress_port(proxied_port: &job_run::msg::create::ProxiedPort) -> GlobalResult<i32> {
	use backend::job::ProxyProtocol;

	let ingress_port = if let Some(ingress_port) = proxied_port.ingress_port {
		ingress_port as i32
	} else {
		match internal_unwrap_owned!(backend::job::ProxyProtocol::from_i32(
			proxied_port.proxy_protocol
		)) {
			ProxyProtocol::Http => 80_i32,
			ProxyProtocol::Https => 443,
			// TODO: https://linear.app/rivet-gg/issue/RIV-1799
			ProxyProtocol::Tcp | ProxyProtocol::TcpTls => rand::thread_rng().gen_range(
				util_job::consts::MIN_INGRESS_PORT_TCP..=util_job::consts::MAX_INGRESS_PORT_TCP,
			) as i32,
			// TODO: https://linear.app/rivet-gg/issue/RIV-1799
			ProxyProtocol::Udp => rand::thread_rng().gen_range(
				util_job::consts::MIN_INGRESS_PORT_UDP..=util_job::consts::MAX_INGRESS_PORT_UDP,
			) as i32,
		}
	};

	Ok(ingress_port)
}
