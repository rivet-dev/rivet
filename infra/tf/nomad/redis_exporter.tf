# This gets ran inside of Nomad instead of inside the Redis node since we use a
# managed Redis service for production environments.

locals {
	service_redis_exporter = lookup(var.services, "redis-exporter", {
		count = 1
		resources = {
			cpu = 100
			cpu_cores = 0
			memory = 256
		}
	})
}

module "redis_secrets" {
	source = "../modules/secrets"

	keys = flatten([
		for k, v in var.redis_svcs:
		[
			"redis/${k}/username",
			"redis/${k}/password",
		]
	])
	optional = true
}

resource "nomad_job" "redis_exporter" {
	detach = false

	hcl2 {
		enabled  = true
	}

	jobspec = templatefile("${path.module}/files/redis_exporter/redis_exporter.nomad.tpl", {
		shared = local.jobspec_shared

		dc = local.primary_dc
		count = local.service_redis_exporter.count
		resources = local.service_redis_exporter.resources

		redis_svcs = {
			for k, v in var.redis_svcs:
			k => {
				endpoint = v.endpoint
				username = module.redis_secrets.values["redis/${k}/username"]
				password = module.redis_secrets.values["redis/${k}/password"]
			}
		}
	})
}

