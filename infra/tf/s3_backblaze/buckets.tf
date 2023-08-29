resource "b2_bucket" "bucket" {
	for_each = var.s3_buckets

	lifecycle {
		# TODO: Remove this. Fixes an issue with accidentally enabled file locks that can't be disabled.
		ignore_changes = ["file_lock_configuration"]
	}

	bucket_name = each.key
	bucket_type = "allPrivate"

	# Enable upload CORS policy for buckets that upload items
	dynamic "cors_rules" {
		for_each = each.value.policy == "upload" ? [0] : []

		content {
			cors_rule_name = "s3AnyOrigin"
			allowed_headers = ["content-type", "content-length"]
			allowed_operations = ["s3_put"]
			allowed_origins = each.value.cors_allowed_origins
			max_age_seconds = 3600
		}
	}
}

