terraform {
	required_providers {
		cockroach = {
			source = "cockroachdb/cockroach"
			version = "1.1.0"
		}
	}
}

module "secrets" {
	source = "../modules/secrets"

	keys = [ "crdb/username", "crdb/password", "cockroachdb_cloud/api_key" ]
}

resource "cockroach_cluster" "main" {
	cloud_provider = "AWS"
	name = "rivet-${var.namespace}"

	regions = [{
		name = data.terraform_remote_state.k8s_cluster_aws.outputs.region
		primary = true
	}]

	serverless = {
		spend_limit = var.cockroachdb_spend_limit
		uasge_limits = {
			request_unit_limit = var.cockroachdb_request_unit_limit
			storage_mib_limit = var.cockroachdb_storage_limit
		}
	}
}

# TODO: This does not delete the 0.0.0.0/0 entrypoint
resource "cockroach_allow_list" "eks" {
	for_each = data.terraform_remote_state.k8s_cluster_aws.outputs.nat_public_ips

	cluster_id = cockroach_cluster.main.id
	cidr_ip = each.value
	cidr_mask = 32
	sql = true
	ui = false
}

resource "cockroach_sql_user" "root" {
	cluster_id = cockroach_cluster.main.id
	name = module.secrets.values["crdb/username"]
	password = module.secrets.values["crdb/password"]
}


data "cockroach_cluster_cert" "main" {
	id = cockroach_cluster.main.id
}

resource "kubernetes_config_map" "crdb_ca" {
	for_each = toset(flatten([
		["rivet-service", "bolt"],
		var.prometheus_enabled ? ["grafana"] : []
	]))

	metadata {
		name = "crdb-ca"
		namespace = each.value
	}

	data = {
		"ca.crt" = data.cockroach_cluster_cert.main.cert
	}
}
