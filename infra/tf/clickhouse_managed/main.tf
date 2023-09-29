terraform {
	required_providers {
		clickhouse = {
			source = "ClickHouse/clickhouse"
			version = "0.0.3"
		}
	}
}

module "secrets" {
	source = "../modules/secrets"

	keys = [
		"clickhouse_cloud/organization_id",
		"clickhouse_cloud/token_key",
		"clickhouse_cloud/token_secret",
		"clickhouse/users/default/password"
	]
}

resource "clickhouse_service" "main" {
	cloud_provider = "aws"
	name = "rivet-${var.namespace}"
	region = data.terraform_remote_state.k8s_cluster_aws.outputs.region

	ip_access = [
		for x in data.terraform_remote_state.k8s_cluster_aws.outputs.nat_public_ips:
		{
			source = x
			description = "AWS NAT"
		}
	]

	# TODO:
	tier = "development"

	password = module.secrets.values["clickhouse/users/default/password"]

	# Bug in ClickHouse provider for the `development` tier leads to "inconsistent result" error
	lifecycle {
		ignore_changes = [
			idle_scaling,
		]
	}
}
