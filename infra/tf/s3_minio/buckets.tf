resource "null_resource" "check_minio" {
	depends_on = [helm_release.minio, kubectl_manifest.minio_ingress_route]

	provisioner "local-exec" {
		command = <<EOL
		until curl -sf https://minio.${var.domain_main}/minio/health/ready; do
			echo "Waiting for Minio to become reachable..."
			sleep 1
		done
		EOL
	}
}

resource "aws_s3_bucket" "bucket" {
	depends_on = [null_resource.check_minio]

	for_each = var.s3_buckets

	bucket = each.key

	# TODO: Configure bucket type
	# TODO: Configure CORS
}

