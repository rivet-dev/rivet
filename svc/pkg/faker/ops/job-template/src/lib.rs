use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde_json::json;
use std::collections::HashMap;

#[operation(name = "faker-job-template")]
async fn handle(
	ctx: OperationContext<faker::job_template::Request>,
) -> GlobalResult<faker::job_template::Response> {
	let job_spec_json = serde_json::to_string(&gen_job(&ctx)?)?;

	Ok(faker::job_template::Response { job_spec_json })
}

fn gen_job(
	ctx: &OperationContext<faker::job_template::Request>,
) -> GlobalResult<nomad_client::models::Job> {
	use nomad_client::models::*;

	let GenTaskOutput {
		ports,
		meta_required,
		task,
	} = gen_task(ctx)?;

	Ok(Job {
		_type: Some("batch".into()),
		constraints: Some(vec![Constraint {
			l_target: Some("${node.class}".into()),
			r_target: Some("job".into()),
			operand: Some("=".into()),
		}]),
		parameterized_job: Some(Box::new(ParameterizedJobConfig {
			meta_required,
			meta_optional: Some(vec!["rivet_test_id".to_string()]),
			..ParameterizedJobConfig::new()
		})),
		task_groups: Some(vec![TaskGroup {
			name: Some(util_job::RUN_MAIN_TASK_NAME.into()),
			networks: Some(vec![NetworkResource {
				mode: Some("cni/rivet-job".into()),
				dynamic_ports: Some(ports),
				..NetworkResource::new()
			}]),
			services: Some(vec![Service {
				provider: Some("nomad".into()),
				ID: Some("test-job".into()),
				name: Some("test-job".into()),
				tags: Some(vec!["test".into()]),
				..Service::new()
			}]),
			ephemeral_disk: Some(Box::new(EphemeralDisk {
				size_mb: Some(32),
				..EphemeralDisk::new()
			})),
			tasks: Some(vec![task]),
			..TaskGroup::new()
		}]),
		..Job::new()
	})
}

struct GenTaskOutput {
	ports: Vec<nomad_client::models::Port>,
	meta_required: Option<Vec<String>>,
	task: nomad_client::models::Task,
}

fn gen_task(ctx: &OperationContext<faker::job_template::Request>) -> GlobalResult<GenTaskOutput> {
	use nomad_client::models::*;

	let base_task = Task {
		resources: Some(Box::new(Resources {
			CPU: Some(ctx.cpu.unwrap_or(100u32) as i32),
			memory_mb: Some(ctx.memory_mb.unwrap_or(128u32) as i32),
			..Resources::new()
		})),
		..Task::new()
	};

	Ok(match unwrap_ref!(ctx.kind) {
		faker::job_template::request::Kind::EchoServer(_) => GenTaskOutput {
			ports: vec![Port {
				label: Some("http".into()),
				value: None,
				to: Some(80),
			}],
			meta_required: Some(vec!["test_id".into()]),
			task: Task {
				name: Some(util_job::RUN_MAIN_TASK_NAME.into()),
				driver: Some("docker".into()),
				config: Some({
					let mut config = HashMap::new();
					config.insert("image".into(), json!("node:16.13.0-buster"));
					config.insert(
						"entrypoint".into(),
						json!(["node", "${NOMAD_TASK_DIR}/server.js"]),
					);
					config.insert("ports".into(), json!(["http"]));
					config
				}),
				templates: Some(vec![Template {
					dest_path: Some("local/server.js".into()),
					embedded_tmpl: Some(
						indoc!(
							r#"
							console.log("Listening on", process.env.NOMAD_PORT_http);
							require("http")
								.createServer((req, res) => {
									console.log("Received request");
									res.end(process.env.NOMAD_META_test_id);
								})
								.listen(process.env.NOMAD_PORT_http);
							"#
						)
						.into(),
					),
					..Template::new()
				}]),
				log_config: Some(Box::new(LogConfig {
					max_files: Some(1),
					max_file_size_mb: Some(4),
				})),
				..base_task
			},
		},
		faker::job_template::request::Kind::EchoServerTcp(_) => GenTaskOutput {
			ports: vec![Port {
				label: Some("tcp".into()),
				value: None,
				to: Some(80),
			}],
			meta_required: Some(vec!["test_id".into()]),
			task: Task {
				name: Some(util_job::RUN_MAIN_TASK_NAME.into()),
				driver: Some("docker".into()),
				config: Some({
					let mut config = HashMap::new();
					config.insert("image".into(), json!("node:16.13.0-buster"));
					config.insert(
						"entrypoint".into(),
						json!(["node", "${NOMAD_TASK_DIR}/server.js"]),
					);
					config.insert("ports".into(), json!(["tcp"]));
					config
				}),
				templates: Some(vec![Template {
					dest_path: Some("local/server.js".into()),
					embedded_tmpl: Some(
						indoc!(
							r#"
							console.log("Listening on", process.env.NOMAD_PORT_tcp);
							require("net")
								.createServer((socket) => {
									console.log('socket open');
									socket.on("data", (data) => {
										console.log("Received data", process.env.NOMAD_META_test_id);
										socket.write(process.env.NOMAD_META_test_id, 'utf8', () => {
											console.log("Data flushed");
											socket.destroy();
										});
									});
								})
								.listen(process.env.NOMAD_PORT_tcp);
							"#
						)
						.into(),
					),
					..Template::new()
				}]),
				log_config: Some(Box::new(LogConfig {
					max_files: Some(1),
					max_file_size_mb: Some(4),
				})),
				..base_task
			},
		},
		faker::job_template::request::Kind::EchoServerUdp(_) => GenTaskOutput {
			ports: vec![Port {
				label: Some("udp".into()),
				value: None,
				to: Some(80),
			}],
			meta_required: Some(vec!["test_id".into()]),
			task: Task {
				name: Some(util_job::RUN_MAIN_TASK_NAME.into()),
				driver: Some("docker".into()),
				config: Some({
					let mut config = HashMap::new();
					config.insert("image".into(), json!("node:16.13.0-buster"));
					config.insert(
						"entrypoint".into(),
						json!(["node", "${NOMAD_TASK_DIR}/server.js"]),
					);
					config.insert("ports".into(), json!(["udp"]));
					config
				}),
				templates: Some(vec![Template {
					dest_path: Some("local/server.js".into()),
					embedded_tmpl: Some(
						indoc!(
							r#"
							console.log("Listening on", process.env.NOMAD_PORT_udp);

							let server = require('dgram').createSocket("udp4");
							server
								.on("close", () => console.log("Socket closed"))
								.on("message", (message, remote) => {
									console.log(`Received message from ${remote.address}:${remote.port}: ${message.toString()}`);
									server.send(process.env.NOMAD_META_test_id, remote.port, remote.address, (err) => {
										if (err) throw err;
										console.log(`Message sent to ${remote.address}:${remote.port}`);
									});
								})
								.bind(process.env.NOMAD_PORT_udp);
							"#
						)
						.into(),
					),
					..Template::new()
				}]),
				log_config: Some(Box::new(LogConfig {
					max_files: Some(1),
					max_file_size_mb: Some(4),
				})),
				..base_task
			},
		},
		faker::job_template::request::Kind::Log(log) => GenTaskOutput {
			ports: vec![],
			meta_required: None,
			task: Task {
				name: Some(util_job::RUN_MAIN_TASK_NAME.into()),
				driver: Some("docker".into()),
				config: Some({
					let mut config = HashMap::new();
					config.insert("image".into(), json!("alpine:3.14"));
					config.insert("args".into(), json!(["sh", "${NOMAD_TASK_DIR}/run.sh"]));
					config
				}),
				templates: Some(vec![
					Template {
						dest_path: Some("local/run.sh".into()),
						embedded_tmpl: Some(
							indoc!(
								r#"
								#!/bin/sh
								cat ${NOMAD_TASK_DIR}/stdout.txt
								cat ${NOMAD_TASK_DIR}/stderr.txt > /dev/stderr
								"#
							)
							.into(),
						),
						..Template::new()
					},
					Template {
						dest_path: Some("local/stdout.txt".into()),
						embedded_tmpl: Some(log.stdout.clone()),
						..Template::new()
					},
					Template {
						dest_path: Some("local/stderr.txt".into()),
						embedded_tmpl: Some(log.stderr.clone()),
						..Template::new()
					},
				]),
				log_config: Some(Box::new(LogConfig {
					max_files: Some(1),
					max_file_size_mb: Some(4),
				})),
				..base_task
			},
		},
		faker::job_template::request::Kind::Exit(exit) => GenTaskOutput {
			ports: vec![],
			meta_required: None,
			task: Task {
				name: Some(util_job::RUN_MAIN_TASK_NAME.into()),
				driver: Some("docker".into()),
				config: Some({
					let mut config = HashMap::new();
					config.insert("image".into(), json!("alpine:3.14"));
					config.insert("args".into(), json!(["sh", "${NOMAD_TASK_DIR}/run.sh"]));
					config
				}),
				templates: Some(vec![Template {
					dest_path: Some("local/run.sh".into()),
					embedded_tmpl: Some(formatdoc!(
						r#"
						#!/bin/sh
						sleep {sleep}
						exit {exit_code}
						"#,
						sleep = exit.sleep_ms as f64 / 1000.,
						exit_code = exit.exit_code,
					)),
					..Template::new()
				}]),
				log_config: Some(Box::new(LogConfig {
					max_files: Some(1),
					max_file_size_mb: Some(4),
				})),
				..base_task
			},
		},
		faker::job_template::request::Kind::Counter(counter) => GenTaskOutput {
			ports: vec![],
			meta_required: None,
			task: Task {
				name: Some(util_job::RUN_MAIN_TASK_NAME.into()),
				driver: Some("docker".into()),
				config: Some({
					let mut config = HashMap::new();
					config.insert("image".into(), json!("alpine:3.14"));
					config.insert("args".into(), json!(["sh", "${NOMAD_TASK_DIR}/run.sh"]));
					config
				}),
				templates: Some(vec![Template {
					dest_path: Some("local/run.sh".into()),
					embedded_tmpl: Some(formatdoc!(
						r#"
						#!/bin/sh
						counter=0
						while [ true ]; do
							echo "Counter: $counter"
							let 'counter++'
							sleep {sleep}
						done
						"#,
						sleep = counter.interval_ms as f64 / 1000.,
					)),
					..Template::new()
				}]),
				log_config: Some(Box::new(LogConfig {
					max_files: Some(1),
					max_file_size_mb: Some(4),
				})),
				..base_task
			},
		},
		faker::job_template::request::Kind::Stress(stress) => GenTaskOutput {
			ports: vec![Port {
				label: Some("http".into()),
				value: None,
				to: Some(80),
			}],
			meta_required: Some(vec!["test_id".into()]),
			task: Task {
				name: Some(util_job::RUN_MAIN_TASK_NAME.into()),
				driver: Some("docker".into()),
				config: Some({
					let mut config = HashMap::new();
					config.insert("image".into(), json!("debian:12.1-slim"));
					config.insert("args".into(), json!(["sh", "${NOMAD_TASK_DIR}/run.sh"]));
					config
				}),
				templates: Some(vec![Template {
					dest_path: Some("local/run.sh".into()),
					embedded_tmpl: Some(formatdoc!(
						r#"
						#!/bin/sh
						apt update -y
						apt install -y stress-ng
						echo 'Stressing with {flags}'
						stress-ng {flags}
						"#,
						flags = stress.flags
					)),
					..Template::new()
				}]),
				log_config: Some(Box::new(LogConfig {
					max_files: Some(1),
					max_file_size_mb: Some(4),
				})),
				..base_task
			},
		},
	})
}
