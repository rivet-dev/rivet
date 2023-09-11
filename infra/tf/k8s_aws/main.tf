terraform {
  required_providers {
    aws = {
      source = "hashicorp/aws"
      version = "5.16.0"
    }
  }
}

locals {
	name = "rivet-${var.namespace}"
	cluster_version = "1.27"
	region = "us-east-1"

	vpc_cidr = "10.0.0.0/16"
	azs = slice(data.aws_availability_zones.available.names, 0, 3)

	tags = {
		Namespace = var.namespace
	}
}

data "aws_caller_identity" "current" {}
data "aws_availability_zones" "available" {}

