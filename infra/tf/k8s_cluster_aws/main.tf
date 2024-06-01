terraform {
	required_providers {
		aws = {
			source = "hashicorp/aws"
			version = "5.52.0"
		}
		# TODO Revert to gavinbunney/kubectl once https://github.com/gavinbunney/terraform-provider-kubectl/issues/270 is resolved
		kubectl = {
			source = "alekc/kubectl"
			version = ">= 2.0.2"
		}
	}
}

data "aws_availability_zones" "available" {}
data "aws_caller_identity" "current" {}
data "aws_partition" "current" {}

locals {
	name = "rivet-${var.namespace}"
	cluster_version = "1.28"
	region = "us-east-1"

	vpc_cidr = "10.0.0.0/16"
	azs = slice(data.aws_availability_zones.available.names, 0, 3)

	account_id = data.aws_caller_identity.current.account_id
	partition  = data.aws_partition.current.partition

	tags = {
		Namespace = var.namespace
	}
}

