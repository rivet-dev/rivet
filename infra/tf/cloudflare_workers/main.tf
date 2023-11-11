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

locals {
	cf_request_meta_routes = toset(concat([
		"api.${var.domain_main}/matchmaker/lobbies/create",
		"api.${var.domain_main}/matchmaker/lobbies/find",
		"api.${var.domain_main}/matchmaker/lobbies/join",
		"api.${var.domain_main}/matchmaker/lobbies/list",
		"api.${var.domain_main}/matchmaker/regions",
	], var.dns_deprecated_subdomains ? [
		"matchmaker.api.${var.domain_main}/v1/lobbies/create",
		"matchmaker.api.${var.domain_main}/v1/lobbies/find",
		"matchmaker.api.${var.domain_main}/v1/lobbies/join",
		"matchmaker.api.${var.domain_main}/v1/lobbies/list",
		"matchmaker.api.${var.domain_main}/v1/regions",
	] : []))
}

resource "cloudflare_worker_script" "request_meta" {
	account_id = var.cloudflare_account_id
	name = "${var.namespace}-request-meta"
	content = file("${path.module}/files/request_meta.js")
}

resource "cloudflare_worker_route" "request_meta_route" {
	for_each = local.cf_request_meta_routes

	zone_id = data.terraform_remote_state.dns.outputs.cloudflare_zone_ids.main
	pattern = each.value
	script_name = cloudflare_worker_script.request_meta.name
}
