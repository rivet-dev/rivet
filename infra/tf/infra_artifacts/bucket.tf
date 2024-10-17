locals {
	artifacts_bucket_name = "${var.namespace}-bucket-infra-artifacts"
}

resource "aws_s3_bucket" "bucket" {
	bucket = local.artifacts_bucket_name
}

