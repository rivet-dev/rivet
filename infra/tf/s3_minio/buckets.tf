# Check if Minio has bootstrapped yet
resource "null_resource" "check_minio" {
	provisioner "local-exec" {
		command = <<EOL
		until curl -sf ${var.s3_providers.minio.endpoint_external}/minio/health/ready; do
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

