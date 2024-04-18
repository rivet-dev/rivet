use indoc::indoc;

pub mod nomad;
pub mod rivet;
pub mod s3;
pub mod traefik;
pub mod traffic_server;
pub mod vector;

pub const TUNNEL_API_INTERNAL_PORT: u16 = 5010;

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
	pub fn install() -> String {
		"apt-get install -y lz4".to_string()
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
			curl -Lf -o /usr/bin/cnitool "https://github.com/rivet-gg/cni/releases/download/v1.1.2-build3/cnitool"
			chmod +x /usr/bin/cnitool
			"#
		).to_string()
	}

	pub fn plugins() -> String {
		include_str!("../files/cni_plugins.sh").to_string()
	}
}
