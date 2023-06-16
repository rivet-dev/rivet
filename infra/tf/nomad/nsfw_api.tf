locals {
	service_nsfw_api = lookup(var.services, "nsfw-api", {
		count = 1
		resources = {
			cpu = 250
			cpu_cores = 0
			memory = 512
		}
	})
}

resource "nomad_job" "nsfw_api" {
	detach = false

	hcl2 {
		enabled  = true
	}

	jobspec = templatefile("${path.module}/files/nsfw_api/nsfw_api.nomad.tpl", {
		shared = local.jobspec_shared

		dc = local.primary_dc
		count = local.service_nsfw_api.count
		resources = local.service_nsfw_api.resources
	})
}

