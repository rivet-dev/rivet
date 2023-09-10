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

provider "kubernetes" {
	host = module.eks.cluster_endpoint
	cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)

	# Dynamically access token
	exec {
		api_version = "client.authentication.k8s.io/v1beta1"
		command = "aws"
		args = ["eks", "get-token", "--cluster-name", module.eks.cluster_name]
	}
}

provider "helm" {
	kubernetes {
		host = module.eks.cluster_endpoint
		cluster_ca_certificate = base64decode(module.eks.cluster_certificate_authority_data)

		exec {
			api_version = "client.authentication.k8s.io/v1beta1"
			command = "aws"
			args = ["eks", "get-token", "--cluster-name", module.eks.cluster_name]
		}
	}
}

data "aws_availability_zones" "available" {}

# MARK: VPC
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

# MARK: EKS
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
			# Placeholder policy that can be updated later to affect the Fargate profiles
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

# MARK: EFS
module "efs" {
	source  = "terraform-aws-modules/efs/aws"
	version = "1.2.0"

	creation_token = local.name
	name = local.name

	# TODO: Do I need to create an access point? https://aws.amazon.com/blogs/containers/running-stateful-workloads-with-amazon-eks-on-aws-fargate-using-amazon-efs/#:~:text=Create%20an%20EFS%20access%20point%3A

	# Mount targets / security group
	mount_targets = {
		for k, v in zipmap(local.azs, module.vpc.private_subnets):
		k => { subnet_id = v }
	}
	security_group_description = "${local.name} EFS security group"
	security_group_vpc_id = module.vpc.vpc_id
	security_group_rules = {
		vpc = {
			# Defaults to EFS/NFS (2049/TCP + ingress)
			description = "NFS ingress from VPC private subnets"
			cidr_blocks = module.vpc.private_subnets_cidr_blocks
		}
	}

	tags = local.tags
}


resource "aws_iam_role" "efs_driver_role" {
	name = "${local.name}-efs-driver-role"

	assume_role_policy = jsonencode({
		Version = "2012-10-17",
		Statement = [
			# {
			# 	Action = "sts:AssumeRole",
			# 	Effect = "Allow",
			# 	Principal = {
			# 		Service = "eks.amazonaws.com"
			# 	}
			# }
			{
				"Effect": "Allow",
				"Principal": {
					# "Federated": "arn:aws:iam::${local.account_id}:oidc-provider/oidc.eks.${local.region}.amazonaws.com/id/${local.oidc_provider}"
					"Federated": module.eks.oidc_provider_arn
				},
				"Action": "sts:AssumeRoleWithWebIdentity",
				"Condition": {
					"StringLike": {
						"${module.eks.oidc_provider}:sub": "system:serviceaccount:kube-system:efs-csi-*",
						"${module.eks.oidc_provider}:aud": "sts.amazonaws.com",
						# "oidc.eks.${local.region}.amazonaws.com/id/${local.oidc_provider}:sub": "system:serviceaccount:kube-system:efs-csi-*",
						# "oidc.eks.${local.region}.amazonaws.com/id/${local.oidc_provider}:aud": "sts.amazonaws.com"
					}
				}
			}

		]
	})
}

# resource "aws_iam_role_policy_attachment" "efs_driver_policy_attachment" {
# 	# TODO: Is this the right name?
# 	policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonEFSCSIDriverPolicy"
# 	role = aws_iam_role.efs_driver_role.name
# }

data "aws_iam_policy_document" "efs_csi" {
	statement {
		effect = "Allow"
		actions = [
			"elasticfilesystem:DescribeAccessPoints",
			"elasticfilesystem:DescribeFileSystems",
			"elasticfilesystem:DescribeMountTargets",
			"ec2:DescribeAvailabilityZones"
		]
		resources = ["*"]
	}

	statement {
		effect = "Allow"
		actions = [
			"elasticfilesystem:CreateAccessPoint"
		]
		resources = [module.efs.arn]
		condition {
			test     = "StringLike"
			variable = "aws:RequestTag/efs.csi.aws.com/cluster"
			values   = [module.eks.cluster_name]
		}
	}

	statement {
		effect = "Allow"
		actions = [
			"elasticfilesystem:DeleteAccessPoint"
		]
		resources = ["*"]
		condition {
			test     = "StringLike"
			variable = "aws:ResourceTag/efs.csi.aws.com/cluster"
			values   = [module.eks.cluster_name]
		}
	}

	statement {
		effect = "Allow"
		actions = [
			"elasticfilesystem:ClientRootAccess",
			"elasticfilesystem:ClientWrite",
			"elasticfilesystem:ClientMount"
		]
		resources = [module.efs.arn]
	}
}

resource "aws_iam_policy" "efs_csi" {
	name        = "${local.name}-efs-csi-driver-policy"
	description = "Policy for efs-csi-driver service account"
	policy      = data.aws_iam_policy_document.efs_csi.json
}

module "efs_csi_iam_assumable_role" {
	source       = "terraform-aws-modules/iam/aws//modules/iam-assumable-role-with-oidc"
	version      = "5.11.2"
	create_role  = true
	role_name    = "${local.name}-efs-csi-driver-sa-role"
	provider_url = module.eks.cluster_oidc_issuer_url
	role_policy_arns = [
		aws_iam_policy.efs_csi.arn
	]
	oidc_fully_qualified_subjects = [
		"system:serviceaccount:kube-system:efs-csi-controller-sa",
		"system:serviceaccount:kube-system:efs-csi-node-sa"
	]
}

