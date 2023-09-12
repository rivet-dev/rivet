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

	keys = ["cockroachdb/api_key"]
}

resource "cockroach_cluster" "main" {
	cloud_provider = "AWS"
	name = "rivet-${var.namespace}"

	regions = [{
		name = "us-east-1"
		primary = true
	}]

	serverless = {
		# TODO: spend_limit
		# TODO: uasge_limit
	}
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
