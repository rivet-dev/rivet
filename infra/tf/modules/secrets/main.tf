terraform {
	required_providers {
		external = {
			source = "hashicorp/external"
			version = "2.3.3"
		}
	}
}

data "external" "bolt_secret" {
	for_each = var.keys

	program = ["${path.module}/scripts/get_secret.sh", each.key, var.optional ? "true" : "false"]
}

