locals {
	cloudflare_zone_ids = {
		main = local.cloudflare_zone_id_main
		cdn = local.cloudflare_zone_id_cdn
		job = local.cloudflare_zone_id_job
	}

    # Add fake local server if developing locally.
    servers = var.deploy_method_local ? merge(var.servers, {
        "${var.namespace}-local" = {
            region_id = "local"
            pool_id = "local"
            name = "${var.namespace}-local"
        }
    }) : var.servers
}

locals {
	records = flatten([
		# Extra DNS
		flatten([
			for record in var.extra_dns:
			{
				zone_id = local.cloudflare_zone_ids[record.zone_name]
				name = record.name
				proxied = true
			}
		]),

		# CDN
		[
			{
				zone_id = local.cloudflare_zone_id_cdn
				name = "${var.domain_cdn}"
				proxied = true
			},
			{
				zone_id = local.cloudflare_zone_id_cdn
				name = "fallback.${var.domain_cdn}"
				proxied = true
			},
			{
				zone_id = local.cloudflare_zone_id_cdn
				name = "*.${var.domain_cdn}"
				proxied = true
			},
		],

		# Job. Matchmaker lobbies will point CNAME record at this.
		[
			for server_id, server in local.servers:
			{
				zone_id = local.cloudflare_zone_id_job
				name = "*.lobby.${server.region_id}.${var.domain_job}"
				server = server
				proxied = false
			}
			if server.pool_id == "gg"
		],
		# Lobby ID in path instead of domain
		[
			for server_id, server in local.servers:
			{
				zone_id = local.cloudflare_zone_id_job
				name = "lobby.${server.region_id}.${var.domain_job}"
				server = server
				proxied = false
			}
			if server.pool_id == "gg"
		],

		# Deprecated
		var.dns_deprecated_subdomains ? [{
			zone_id = local.cloudflare_zone_id_main
			name = "media.${var.domain_main}"
			proxied = true
		}] : [],
	])
}

resource "cloudflare_record" "main" {
	for_each = {
		for record in local.records:
		"${record.zone_id}:${record.name}:${try(record.server.name, "core")}" => record
	}

	zone_id = each.value.zone_id
	name = each.value.name
    # Use local node's public IP if in local region. Otherwise, look up server's IP.
	value = try(data.terraform_remote_state.pools.outputs.servers[each.value.server.name].public_ipv4, data.terraform_remote_state.k8s_infra.outputs.traefik_external_ip)
	type = (can(each.value.server) || var.deploy_method_local) ? "A" : "CNAME"
	# TODO: Increase the unproxied TTL once we have proper floating IP support on all providers
	ttl = each.value.proxied ? 1 : 60  # 1 = automatic
	proxied = each.value.proxied
}

moved {
	from = cloudflare_record.rivet_gg
	to = cloudflare_record.main
}

// MARK: Zone metadata
data "cloudflare_zone" "main" {
	zone_id = local.cloudflare_zone_id_main
}

data "cloudflare_zone" "cdn" {
	zone_id = local.cloudflare_zone_id_cdn
}

data "cloudflare_zone" "job" {
	zone_id = local.cloudflare_zone_id_job
}
