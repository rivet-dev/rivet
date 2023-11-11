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


data "cloudflare_zones" "all" {
	filter {
		account_id = var.cloudflare_account_id
		status = "active"
	}
}

locals {
	# TODO: Fix bug where "foo.bar.com" matches the zone "ar.com"
	cloudflare_zone_id_main = [
		for x in data.cloudflare_zones.all.zones:
		x.id
		if endswith(var.domain_main, x.name)
	][0]
	cloudflare_zone_id_cdn = [
		for x in data.cloudflare_zones.all.zones:
		x.id
		if endswith(var.domain_cdn, x.name)
	][0]
	cloudflare_zone_id_job = [
		for x in data.cloudflare_zones.all.zones:
		x.id
		if endswith(var.domain_job, x.name)
	][0]
}

