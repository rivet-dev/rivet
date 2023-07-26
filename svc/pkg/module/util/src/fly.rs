use serde_json::json;

pub struct MachineConfig<'a> {
	pub image: &'a str,
}

impl MachineConfig<'_> {
	/// Builds the JSON config for the Fly machine.
	pub fn build_machine_config(&self) -> serde_json::Value {
		json!({
			"services": [
			{
				"protocol": "tcp",
				"internal_port": 80,
				"autostop": true,
				"autostart": true,
				"ports": [
				{
					"port": 80,
					"handlers": ["http"],
					"force_https": true
				},
				{
					"port": 443,
					"handlers": ["http", "tls"]
				}
				],
				"force_instance_key": null
			}
			],
			"checks": {
				"alive": {
					"type": "tcp",
					"port": 80,
					"interval": "15s",
					"timeout": "2s",
					"grace_period": "5s"
				},
				"health": {
					"type": "http",
					"port": 80,
					"path": "/healthz",
					"interval": "15s",
					"timeout": "2s",
					"grace_period": "5s"
				}
			},
			"image": self.image,
			"restart": {},
			"guest": {
			"cpu_kind": "shared",
			"cpus": 1,
			"memory_mb": 512
			}
		})
	}
}
