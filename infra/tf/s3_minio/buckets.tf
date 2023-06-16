resource "aws_s3_bucket" "bucket" {
	for_each = var.s3_buckets

	bucket = each.key

	# TODO: Configure bucket type
	# TODO: Configure CORS
}

