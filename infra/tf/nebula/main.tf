terraform {
	required_providers {
		nebula = {
			source = "TelkomIndonesia/nebula"
			version = "0.3.1"
		}
	}
}

resource "nebula_ca" "main" {
	name = "Rivet (${var.namespace})"
	duration = "17531h" # 2y
}
