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

	for_each = {
		for k, v in var.pools:
		k => v
		if v.tunnels != null
	}

	name = "${var.namespace}-${each.key}"
	cloudflare_account_id = var.cloudflare_account_id
	ingress = {
		for k, v in each.value.tunnels:
		"${k}.${var.domain_main}" => {
			app_name = "${v.name} (${var.namespace})"
 			cloudflare_zone_id = var.cloudflare_zone_id_rivet_gg
			access_groups = try(v.access_groups, null)
			service_tokens = try(v.service_tokens, null)
			service = v.service
			no_app = try(v.no_app, false)
		}
	}
}

