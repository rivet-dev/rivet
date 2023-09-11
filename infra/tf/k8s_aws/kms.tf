module "ebs_kms_key" {
	source  = "terraform-aws-modules/kms/aws"
	version = "2.0.0"

	description = "Customer managed key to encrypt EKS managed node group volumes"

	# Policy
	key_administrators = [
		data.aws_caller_identity.current.arn
	]

	key_service_roles_for_autoscaling = [
		# required for the ASG to manage encrypted volumes for nodes
		"arn:aws:iam::${data.aws_caller_identity.current.account_id}:role/aws-service-role/autoscaling.amazonaws.com/AWSServiceRoleForAutoScaling",
		# required for the cluster / persistentvolume-controller to create encrypted PVCs
		module.eks.cluster_iam_role_arn,
	]

	# Aliases
	aliases = ["eks/${local.name}/ebs"]

	tags = local.tags
}

module "key_pair" {
	source  = "terraform-aws-modules/key-pair/aws"
	version = "2.0.2"

	key_name_prefix    = local.name
	create_private_key = true

	tags = local.tags
}

