terraform {
  required_providers {
    aws = {
      source = "hashicorp/aws"
      version = "5.16.0"
    }
  }
}

locals {
	name = "eks-test"
	cluster_version = "1.27"
	region = "us-east-1"

	vpc_cidr = "10.0.0.0/16"
	azs = slice(data.aws_availability_zones.available.names, 0, 3)

	tags = {
		Example = local.name
	}
}

provider "aws" {
	region = local.region
}

data "aws_availability_zones" "available" {}

module "vpc" {
	source = "terraform-aws-modules/vpc/aws"
	version = "5.1.2"

	name = local.name
	cidr = local.vpc_cidr

	azs = local.azs
	private_subnets = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 4, k)]
	public_subnets = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 48)]
	intra_subnets = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 52)]

	enable_nat_gateway = true
	single_nat_gateway = true

	public_subnet_tags = {
		"kubernetes.io/role/elb" = 1
	}

	private_subnet_tags = {
		"kubernetes.io/role/internal-elb" = 1
	}

	tags = local.tags
}

module "eks" {
	source = "terraform-aws-modules/eks/aws"
	version = "19.16.0"

	cluster_name = local.name
	cluster_version = local.cluster_version
	cluster_endpoint_public_access = true

	cluster_addons = {
		kube-proxy = {
			# https://docs.aws.amazon.com/eks/latest/userguide/managing-kube-proxy.html
			addon_version = "v1.27.4-eksbuild.2"
		}
		vpc-cni = {
			# https://docs.aws.amazon.com/eks/latest/userguide/managing-vpc-cni.html
			addon_version = "v1.14.0-eksbuild.3"
		}
		coredns = {
			# https://docs.aws.amazon.com/eks/latest/userguide/managing-coredns.html
			addon_version = "v1.10.1-eksbuild.3"
			configuration_values = jsonencode({
				computeType = "Fargate"
			})
		}
		# aws-efs-csi-driver = {
		# 	# https://github.com/kubernetes-sigs/aws-efs-csi-driver/releases
		# 	addon_version = "v1.5.8-eksbuild.1"
		# 	configuration_values = jsonencode({
		# 		computeType = "Fargate"
		# 	})
		# }
	}

	vpc_id = module.vpc.vpc_id
	subnet_ids = module.vpc.private_subnets
	control_plane_subnet_ids = module.vpc.intra_subnets

	# Fargate profiles use the cluster primary security group so these are not utilized
	create_cluster_security_group = false
	create_node_security_group = false

	fargate_profile_defaults = {
		iam_role_additional_policies = {
			additional = aws_iam_policy.additional.arn
		}
	}

	fargate_profiles = merge(
		# Create profile for each AZ for high availability
		#
		# https://stackoverflow.com/questions/75225682/whats-the-difference-between-creating-one-fargate-profile-for-each-availability
		{
			for i in range(3):
			"kube-system-${element(split("-", local.azs[i]), 2)}" => {
				selectors = [
					{ namespace = "kube-system" }
				]
				subnet_ids = [element(module.vpc.private_subnets, i)]
			}
		},
		{
			for i in range(3):
			"test-ns-${element(split("-", local.azs[i]), 2)}" => {
				selectors = [
					{ namespace = "test-ns" }
				]
				subnet_ids = [element(module.vpc.private_subnets, i)]
			}
		},
	)

	tags = local.tags
}

resource "aws_iam_policy" "additional" {
	name = "${local.name}-additional"

	policy = jsonencode({
		Version = "2012-10-17"
		Statement = [
			{
				Action = [
					"ec2:Describe*",
				]
				Effect = "Allow"
				Resource = "*"
			},
		]
	})
}
