# data "aws_iam_policy_document" "efs_csi" {
# 	statement {
# 		effect = "Allow"
# 		actions = [
# 			"elasticfilesystem:DescribeAccessPoints",
# 			"elasticfilesystem:DescribeFileSystems",
# 			"elasticfilesystem:DescribeMountTargets",
# 			"ec2:DescribeAvailabilityZones"
# 		]
# 		resources = ["*"]
# 	}

# 	statement {
# 		effect = "Allow"
# 		actions = [
# 			"elasticfilesystem:CreateAccessPoint"
# 		]
# 		# TODO:
# 		resources = ["<efs-arn>"]
# 		condition {
# 			test     = "StringLike"
# 			variable = "aws:RequestTag/efs.csi.aws.com/cluster"
# 			values   = ["${local.name}"]
# 		}
# 	}

# 	statement {
# 		effect = "Allow"
# 		actions = [
# 			"elasticfilesystem:DeleteAccessPoint"
# 		]
# 		resources = ["*"]
# 		condition {
# 			test     = "StringLike"
# 			variable = "aws:ResourceTag/efs.csi.aws.com/cluster"
# 			values   = ["${local.name}"]
# 		}
# 	}

# 	statement {
# 		effect = "Allow"
# 		actions = [
# 			"elasticfilesystem:ClientRootAccess",
# 			"elasticfilesystem:ClientWrite",
# 			"elasticfilesystem:ClientMount"
# 		]
# 		# TODO:
# 		# resources = ["<efs-id>"]
# 	}
# }

# resource "aws_iam_policy" "efs_csi" {
# 	name        = "${local.name}-efs-csi-driver-policy"
# 	description = "Policy for efs-csi-driver service account"
# 	policy      = data.aws_iam_policy_document.efs_csi.json
# }

# module "ebs_csi_iam_assumable_role" {
# 	source       = "terraform-aws-modules/iam/aws//modules/iam-assumable-role-with-oidc"
# 	version      = "5.11.2"
# 	create_role  = true
# 	role_name    = "${local.name}-ebs-csi-driver-role"
# 	provider_url = local.eks_oidc_provider_schemeless_url
# 	# role_policy_arns = [
# 	# 	aws_iam_policy.efs_csi.arn
# 	# ]
# 	oidc_fully_qualified_subjects = [
# 		"sts.amazonaws.com",
# 		"system:serviceaccount:kube-system:ebs-csi-controller-sa",
# 	]
# }
