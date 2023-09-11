locals {
	cloudflare_zone_ids = {
		base = var.cloudflare_zone_id_rivet_gg
		base_game = var.cloudflare_zone_id_rivet_game
		base_job = var.cloudflare_zone_id_rivet_job
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
				zone_id = var.cloudflare_zone_id_rivet_game
				name = "${var.domain_cdn}"
				proxied = true
			},
			{
				zone_id = var.cloudflare_zone_id_rivet_game
				name = "fallback.${var.domain_cdn}"
				proxied = true
			},
			{
				zone_id = var.cloudflare_zone_id_rivet_game
				name = "*.${var.domain_cdn}"
				proxied = true
			},
			{
				zone_id = var.cloudflare_zone_id_rivet_gg
				name = "cdn.${var.domain_main}"
				proxied = true
			},
			{
				zone_id = var.cloudflare_zone_id_rivet_gg
				name = "media.${var.domain_main}"
				proxied = true
			}
		],

		# Job. Matchmaker lobbies will point CNAME record at this.
		[
			for server_id, server in local.servers:
			{
				zone_id = var.cloudflare_zone_id_rivet_job
				name = "*.lobby.${server.region_id}.${var.domain_job}"
				server = server
				proxied = false
			}
			if server.pool_id == "ing-job"
		],

	])
}

# Allow CLoudflare to serve TLS requests at the edge for our wildcard
# subdomains.
resource "cloudflare_certificate_pack" "rivet_gg" {
	lifecycle {
		create_before_destroy = true
	}

	certificate_authority = "digicert"
	# The certificate must include the root domain in it.
	#
	# We convert to set then back to list to remove potential duplicates of the root zoon.
	hosts = sort(tolist(toset([
		data.cloudflare_zone.rivet_gg.name,
		var.domain_main,
		"*.${var.domain_main}",
		"*.api.${var.domain_main}",
	])))
	type = "advanced"
	validation_method = "txt"
	validity_days = 90
	zone_id = var.cloudflare_zone_id_rivet_gg
	wait_for_active_status = true
}

locals {
	# If CDN is not at the root of the zone, we need to provide a cert pack for the CDN.
	# 
	# If the CDN domain is already at the root of the zone, then Cloudflare exposes a cert back by default and we don't need to create a new one.
	needs_cdn_cert_pack = data.cloudflare_zone.rivet_game.name != var.domain_cdn
}

# Allow for Cloudflare to proxy wildcard requests to this namespace.
resource "cloudflare_certificate_pack" "rivet_game" {
	count = local.needs_cdn_cert_pack ? 1 : 0

	lifecycle {
		create_before_destroy = true
	}

	certificate_authority = "digicert"
	# The certificate must include the root domain in it.
	#
	# We convert to set then back to list to remove potential duplicates of the root zoon.
	hosts = sort(tolist(toset([
		data.cloudflare_zone.rivet_game.name,
        var.domain_cdn,
        "*.${var.domain_cdn}"
    ])))
	type = "advanced"
	validation_method = "txt"
	validity_days = 90
	zone_id = var.cloudflare_zone_id_rivet_game
	wait_for_active_status = true
}

resource "cloudflare_record" "rivet_gg" {
	for_each = {
		for record in local.records:
		"${record.zone_id}:${record.name}:${try(record.server.name, "core")}" => record
	}

	zone_id = each.value.zone_id
	name = each.value.name
    # Use local node's public IP if in local region. Otherwise, look up server's IP.
	value = try(data.terraform_remote_state.pools.outputs.servers[each.value.server.name].public_ipv4, data.terraform_remote_state.k8s_infra.outputs.traefik_external_ip)
	type = can(each.value.server.public_ipv4) ? "A" : "CNAME"
	# TODO: Increase the unproxied TTL once we have proper floating IP support on all providers
	ttl = each.value.proxied ? 1 : 60  # 1 = automatic
	proxied = each.value.proxied
}

// MARK: Zone metadata
data "cloudflare_zone" "rivet_gg" {
	zone_id = var.cloudflare_zone_id_rivet_gg
}

data "cloudflare_zone" "rivet_game" {
	zone_id = var.cloudflare_zone_id_rivet_game
}

data "cloudflare_zone" "rivet_job" {
	zone_id = var.cloudflare_zone_id_rivet_job
}
