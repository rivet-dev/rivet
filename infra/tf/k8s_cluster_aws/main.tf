terraform {
	required_providers {
		aws = {
			source = "hashicorp/aws"
			version = "5.16.0"
		}
		# TODO Revert to gavinbunney/kubectl once https://github.com/gavinbunney/terraform-provider-kubectl/issues/270 is resolved
		kubectl = {
			source = "alekc/kubectl"
			version = ">= 2.0.2"
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

data "aws_availability_zones" "available" {}

