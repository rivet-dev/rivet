use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let nomad_config = nomad_util::config_from_env().unwrap();

	let run_res = op!([ctx] faker_job_run {
		..Default::default()
	})
	.await
	.unwrap();
	let run_id = run_res.run_id.as_ref().unwrap().as_uuid();

	// Read the run
	let res = op!([ctx] job_run_get {
		run_ids: vec![run_id.into(), Uuid::new_v4().into()],
	})
	.await
	.unwrap();
	assert_eq!(1, res.runs.len());

	let run = res.runs.first().unwrap();
	let port = run
		.ports
		.iter()
		.find(|p| p.label == "http")
		.expect("missing port");
	let run_meta = match run.run_meta.as_ref().unwrap().kind.as_ref().unwrap() {
		backend::job::run_meta::Kind::Nomad(x) => x.clone(),
	};

	// Access the server to validate the test ID from the meta
	let test_server_addr = format!("http://{}:{}", port.ip, port.source);
	loop {
		tokio::time::sleep(std::time::Duration::from_secs(1)).await;

		tracing::info!(%test_server_addr, "fetching test id from job server");
		let curl_cmd = tokio::process::Command::new("curl")
			.arg(&test_server_addr)
			.output()
			.await
			.unwrap();
		if curl_cmd.status.code() == Some(0) {
			let body = String::from_utf8(curl_cmd.stdout).unwrap();
			tracing::info!(%body, "successful curl");

			assert_eq!(run_res.test_server_response, body, "wrong server response");

			break;
		} else {
			tracing::info!("server not ready yet");
		}
	}
}
