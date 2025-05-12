use indoc::indoc;

pub mod fdb;
pub mod nats;
pub mod nomad;
pub mod ok_server;
pub mod pegboard;
pub mod rivet;
pub mod s3;
pub mod traefik;
pub mod traffic_server;
pub mod vector;

pub fn common() -> String {
	indoc!(
		"
		apt-get update -y
		apt-get install -y apt-transport-https ca-certificates gnupg2 software-properties-common curl jq unzip
		"
	).to_string()
}

pub mod node_exporter {
	pub fn install() -> String {
		include_str!("../files/node_exporter.sh").to_string()
	}
}

pub mod process_exporter {
	pub fn install() -> String {
		include_str!("../files/process_exporter.sh").to_string()
	}
}

pub mod otel_collector {
	use chirp_workflow::prelude::*;
	use serde_json::json;

	use super::traefik::TUNNEL_OTEL_PORT;
	use crate::types::PoolType;

	const VERSION: &str = "0.125.0";

	pub fn install(pool_type: PoolType) -> GlobalResult<String> {
		let config = json!({
			"receivers": {
				"otlp": {
					"protocols": {
						"grpc": {
							"endpoint": "0.0.0.0:4317"
						},
						"http": {
							"endpoint": "0.0.0.0:4318"
						}
					}
				}
			},
			"processors": {
				"batch": {
					"timeout": "5s",
					"send_batch_size": 10000
				},
				"tail_sampling": {
					"decision_wait": "60s",
					"num_traces": 50000,
					"expected_new_traces_per_sec": 10,
					"decision_cache": {
						"sampled_cache_size": 1000,
						"non_sampled_cache_size": 1000
					},
					"policies": if let PoolType::Guard = pool_type {
						json!([
							{
								"name": "policy-1",
								"type": "and",
								"and": {
									"and_sub_policy": [
										{
											"name": "and-policy-1",
											"type": "status_code",
											"status_code": {
												"status_codes": [
													"ERROR"
												]
											}
										},
										{
											"name": "and-policy-2",
											"type": "probabilistic",
											"probabilistic": {
												"sampling_percentage": 10
											}
										}
									]
								}
							},
							{
								"name": "policy-2",
								"type": "ottl_condition",
								"ottl_condition": {
									"span": [
										"name == \"subscribe\" and attributes[\"message\"] == \"pegboard_actor_ready\""
									]
								}
							}
						])
					} else {
						json!([
							{
								"name": "policy-1",
								"type": "status_code",
								"status_code": {
									"status_codes": [
										"ERROR"
									]
								}
							},
							{
								"name": "policy-2",
								"type": "ottl_condition",
								"ottl_condition": {
									"span": [
										"name == \"message\" and attributes[\"message\"] == \"pegboard_actor_ready\""
									]
								}
							}
						])
					}
				}
			},
			"exporters": {
				"otlp": {
					"endpoint": format!("127.0.0.1:{TUNNEL_OTEL_PORT}"),
					"tls": {
						"insecure": true
					}
				}
			},
			"service": {
				"pipelines": {
					"logs": {
						"receivers": [
							"otlp"
						],
						"processors": [
							"batch"
						],
						"exporters": [
							"otlp"
						]
					},
					"traces": {
						"receivers": [
							"otlp"
						],
						"processors": [
							"tail_sampling",
							"batch"
						],
						"exporters": [
							"otlp"
						]
					},
					"metrics": {
						"receivers": [
							"otlp"
						],
						"processors": [
							"batch"
						],
						"exporters": [
							"otlp"
						]
					}
				}
			}
		});
		let config = serde_yaml::to_string(&config)?;

		Ok(include_str!("../files/otel_collector.sh")
			.replace("__VERSION__", VERSION)
			.replace("__CONFIG__", &config))
	}
}

pub mod sysctl {
	pub fn install() -> String {
		include_str!("../files/sysctl.sh").to_string()
	}
}

pub mod docker {
	pub fn install() -> String {
		include_str!("../files/docker.sh").to_string()
	}
}

pub mod lz4 {
	use indoc::indoc;

	pub fn install() -> String {
		// Don't use apt since we need v1.10.0 for multithreading support
		indoc!(
			r#"
			echo 'Downloading lz4'
			curl -L https://releases.rivet.gg/tools/lz4/1.10.0/debian11-amd64/lz4 -o /usr/local/bin/lz4
			chmod +x /usr/local/bin/lz4
			"#
		)
		.to_string()
	}
}

pub mod skopeo {
	pub fn install() -> String {
		"apt-get install -y skopeo".to_string()
	}
}

pub mod umoci {
	use indoc::indoc;

	pub fn install() -> String {
		indoc!(
			r#"
			echo 'Downloading umoci'
			curl -Lf -o /usr/bin/umoci "https://github.com/opencontainers/umoci/releases/download/v0.4.7/umoci.amd64"
			chmod +x /usr/bin/umoci
			"#
		).to_string()
	}
}

pub mod cni {
	use indoc::indoc;

	pub fn tool() -> String {
		indoc!(
			r#"
			echo 'Downloading cnitool'
			curl -Lf -o /usr/bin/cnitool "https://github.com/rivet-gg/cni/releases/download/v1.1.2-build3/cnitool"
			chmod +x /usr/bin/cnitool
			"#
		).to_string()
	}

	pub fn plugins() -> String {
		include_str!("../files/cni_plugins.sh").to_string()
	}
}

pub mod python {
	pub fn install() -> String {
		"apt-get install -y python3 pip".to_string()
	}
}
