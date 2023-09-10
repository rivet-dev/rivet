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
			{
				"Effect": "Allow",
				"Principal": {
					"Federated": module.eks.oidc_provider_arn
				},
				"Action": "sts:AssumeRoleWithWebIdentity",
				"Condition": {
					"StringLike": {
						"${module.eks.oidc_provider}:sub": "system:serviceaccount:kube-system:efs-csi-*",
						"${module.eks.oidc_provider}:aud": "sts.amazonaws.com",
					}
				}
			}

		]
	})
}

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

