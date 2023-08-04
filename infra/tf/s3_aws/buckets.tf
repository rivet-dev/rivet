resource "aws_s3_bucket" "bucket" {
	for_each = var.s3_buckets

	bucket = each.key
}

# Enable upload CORS policy for buckets that upload items
resource "aws_s3_bucket_cors_configuration" "example" {
	for_each = {
		for k, v in var.s3_buckets: 
		k => v
		if v.policy == "upload"
	}

	bucket = each.key

	cors_rule {
		id = "s3AnyOrigin"
		allowed_headers = ["content-type", "content-length"]
		allowed_methods = ["PUT"]
		allowed_origins = each.value.cors_allowed_origins
		max_age_seconds = 3600
	}
}
