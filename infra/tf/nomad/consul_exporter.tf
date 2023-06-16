locals {
	service_consul_exporter = lookup(var.services, "consul-exporter", {
		count = 1
		resources = {
			cpu = 150
			cpu_cores = 0
			memory = 256
		}
	})
}

resource "nomad_job" "consul_exporter" {
	detach = false

	hcl2 {
		enabled  = true
	}

	jobspec = templatefile("${path.module}/files/consul_exporter/consul_exporter.nomad.tpl", {
		shared = local.jobspec_shared

		dc = local.primary_dc
		ocunt = local.service_consul_exporter.count
		resources = local.service_consul_exporter.resources
	})
}

