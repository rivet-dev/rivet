use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "faker-job-run")]
async fn handle(
	ctx: OperationContext<faker::job_run::Request>,
) -> GlobalResult<faker::job_run::Response> {
	let job_spec_json = if let Some(job_spec_json) = ctx.job_spec_json.clone() {
		job_spec_json
	} else {
		let template_res = op!([ctx] faker_job_template {
			kind: Some(faker::job_template::request::Kind::EchoServer(Default::default())),
			..Default::default()
		})
		.await?;
		template_res.job_spec_json.clone()
	};

	let region_id = if let Some(region_id) = &ctx.region_id {
		region_id.as_uuid()
	} else {
		let region_res = op!([ctx] faker_region {}).await?;

		unwrap_ref!(region_res.region_id).as_uuid()
	};

	let run_id = if let Some(run_id) = ctx.run_id.as_ref() {
		run_id.as_uuid()
	} else {
		Uuid::new_v4()
	};

	let server_response = Uuid::new_v4().to_string();
	let mut planned_sub = subscribe!([ctx] job_run::msg::alloc_planned(run_id)).await?;
	let mut started_sub = subscribe!([ctx] job_run::msg::started(run_id)).await?;
	let mut fail_sub = subscribe!([ctx] job_run::msg::fail(run_id)).await?;
	msg!([ctx] job_run::msg::create(run_id) {
		run_id: Some(run_id.into()),
		region_id: Some(region_id.into()),
		parameters: vec![
			job_run::msg::create::Parameter {
				key: "test_id".into(),
				value: server_response.clone(),
			},
		],
		job_spec_json: job_spec_json,
		proxied_ports: if !ctx.proxied_ports.is_empty() {
			ctx.proxied_ports.clone()
		} else {
			vec![
				job_run::msg::create::ProxiedPort {
					target_nomad_port_label: Some("http".into()),
					ingress_port: None,
					ingress_hostnames: vec!["test1.com".into(), "test2.com".into()],
					proxy_protocol: backend::job::ProxyProtocol::Https as i32,
					ssl_domain_mode: backend::job::SslDomainMode::Exact as i32,
				},
			]
		},
		..Default::default()
	})
	.await?;
	tokio::select! {
		res = async {
			planned_sub.next().await?;
			started_sub.next().await?;
			GlobalResult::Ok(())
		} => {
			let _ = res?;
			tracing::info!("started");
		}
		res = fail_sub.next() => {
			let msg = res?;
			tracing::error!(?msg, "run failed");
			rivet_operation::prelude::bail!("run failed");
		}

	}

	let job_res = op!([ctx] job_run_get {
		run_ids: vec![run_id.into()],
	})
	.await?;
	let run = unwrap!(job_res.runs.first());
	ensure_eq!(1, run.ports.len(), "wrong port count");
	let test_server_port = unwrap_ref!(run.ports.first()).source;

	Ok(faker::job_run::Response {
		run_id: Some(run_id.into()),
		region_id: Some(region_id.into()),
		test_server_response: server_response.clone(),
		test_server_port,
	})
}
