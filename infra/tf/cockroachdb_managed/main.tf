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

	keys = ["cockroachdb_cloud/api_key"]
}

resource "cockroach_cluster" "main" {
	cloud_provider = "AWS"
	name = "rivet-${var.namespace}"

	regions = [{
		name = data.terraform_remote_state.k8s_aws.outputs.region
		primary = true
	}]

	serverless = {
		# TODO: spend_limit
		# TODO: uasge_limit
	}
}

resource "cockroach_allow_list" "eks" {
	for_each = data.terraform_remote_state.k8s_aws.outputs.nat_public_ips

	cluster_id = cockroach_cluster.main.id
	cidr_ip = each.value
	cidr_mask = 32
	sql = true
	ui = false
}

# Generate password
resource "random_password" "root_password" {
	length = 32
	special = false
}

resource "cockroach_sql_user" "root" {
	cluster_id = cockroach_cluster.main.id
	name = "rivet-root"
	password = random_password.root_password.result
}


data "cockroach_cluster_cert" "main" {
	id = cockroach_cluster.main.id
}
