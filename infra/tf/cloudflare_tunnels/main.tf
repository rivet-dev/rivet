terraform {
	required_providers {
		cloudflare = {
			source = "cloudflare/cloudflare"
			version = "4.7.1"
		}
	}
}

module "secrets" {
	source = "../modules/secrets"

	keys = ["cloudflare/terraform/auth_token"]
}

module "cloudflare_tunnels" {
	source = "../modules/cloudflare_tunnel"

	name = var.namespace
	cloudflare_account_id = var.cloudflare_account_id
	ingress = {
		for k, v in var.tunnels:
		"${k}.${var.domain_main}" => {
			app_name = "${v.name} (${var.namespace})"
 			cloudflare_zone_id = data.terraform_remote_state.dns.outputs.cloudflare_zone_ids.main
			access_groups = try(v.access_groups, null)
			service_tokens = try(v.service_tokens, null)
			service = v.service
			no_app = try(v.no_app, false)
		}
	}
}

