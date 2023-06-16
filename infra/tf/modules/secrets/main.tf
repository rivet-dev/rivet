terraform {
	required_providers {
		external = {
			source = "hashicorp/external"
			version = "2.3.1"
		}
	}
}

data "external" "bolt_secret" {
	for_each = var.keys

	program = var.optional ? ["bolt", "secret", "get", "--format=json", "--optional", each.key] : ["bolt", "secret", "get", "--format=json", each.key]
}

