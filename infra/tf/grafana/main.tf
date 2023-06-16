terraform {
	required_providers {
		grafana = {
			source = "grafana/grafana"
			version = "1.32.0"
		}
	}
}

module "secrets" {
	source = "../modules/secrets"

	keys = [
		"cloudflare/access/grafana_cloud/client_id",
		"cloudflare/access/grafana_cloud/client_secret",
		"clickhouse/users/grafana/password",
		"grafana/terraform/token",
	]
}
