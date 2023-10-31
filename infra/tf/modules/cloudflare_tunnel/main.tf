terraform {
	required_providers {
		cloudflare = {
			source = "cloudflare/cloudflare"
			version = "4.7.1"
		}
	}
}

locals {
	cert = {
		AccountTag = var.cloudflare_account_id
		TunnelID = cloudflare_tunnel.tunnel.id
		TunnelName = cloudflare_tunnel.tunnel.name
		TunnelSecret = random_id.tunnel_secret.b64_std
	}

	ingress_with_apps = {
		for k, v in var.ingress:
		k => v
		if !try(v.no_app, false)
	}
}

# MARK: Tunnel
resource "random_id" "tunnel_secret" {
	byte_length = 64
}

resource "cloudflare_tunnel" "tunnel" {
	account_id = var.cloudflare_account_id
	name = var.name
	secret = random_id.tunnel_secret.b64_std
}

resource "cloudflare_record" "tunnel" {
	for_each = var.ingress

	zone_id = each.value.cloudflare_zone_id
	name = each.key
	value = cloudflare_tunnel.tunnel.cname
	type = "CNAME"
	proxied = true
}

# MARK: Access
resource "cloudflare_access_application" "tunnel" {
	for_each = local.ingress_with_apps

	zone_id = each.value.cloudflare_zone_id
	name = each.value.app_name
	domain = each.key
	session_duration = "1h"
	app_launcher_visible = try(each.value.app_launcher, true)
}

resource "cloudflare_access_policy" "allow" {
	for_each = {
		for k, v in local.ingress_with_apps:
		k => v
		if try(length(v.access_groups) > 0, false)
	}

	application_id = cloudflare_access_application.tunnel[each.key].id
	zone_id = each.value.cloudflare_zone_id
	name = "User"
	precedence = 10
	decision = "allow"

	include {
		group = each.value.access_groups
	}
}

# TODO: Not working
# resource "cloudflare_access_policy" "service_auth" {
# 	for_each = {
# 		for k, v in local.ingress_with_apps:
# 		k => v
# 		if try(length(v.service_tokens) > 0, false)
# 	}

# 	application_id = cloudflare_access_application.tunnel[each.key].id
# 	zone_id = each.value.cloudflare_zone_id
# 	name = "Service"
# 	precedence = 20
# 	decision = "non_identity"

# 	include {
# 		service_token = each.value.service_tokens
# 	}
# }

