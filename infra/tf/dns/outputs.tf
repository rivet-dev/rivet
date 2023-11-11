output "cloudflare_zone_ids" {
	value = {
		main = local.cloudflare_zone_id_main
		cdn = local.cloudflare_zone_id_cdn
		job = local.cloudflare_zone_id_job
	}
}
