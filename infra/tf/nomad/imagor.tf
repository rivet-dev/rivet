locals {
	service_imagor = lookup(var.services, "imagor", {
		count = 1
		resources = {
			cpu = 250
			cpu_cores = 0
			memory = 512
		}
	})
}

resource "nomad_job" "imagor" {
	detach = false

	hcl2 {
		enabled = true
	}

	jobspec = templatefile("${path.module}/files/imagor/imagor.nomad.tpl", {
		shared = local.jobspec_shared

		dc = local.primary_dc
		count = local.service_imagor.count
		resources = local.service_imagor.resources
		ephemeral_disk = 8000

		imagor_presets = var.imagor_presets

		result_storage_s3_endpoint = data.terraform_remote_state.s3.outputs["s3_endpoint_internal"]
		result_storage_s3_region = data.terraform_remote_state.s3.outputs["s3_region"]
		result_storage_s3_access_key_id = var.s3_persistent_access_key_id
		result_storage_s3_secret_access_key = nonsensitive(var.s3_persistent_access_key_secret)
		result_storage_s3_bucket = "${var.namespace}-bucket-imagor-result-storage"
		s3_providers = var.s3_providers
	})
}
