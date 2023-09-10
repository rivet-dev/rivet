module "efs" {
	source  = "terraform-aws-modules/efs/aws"
	version = "1.2.0"

	creation_token = local.name
	name = local.name

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

module "efs_csi_iam_assumable_role" {
	source       = "terraform-aws-modules/iam/aws//modules/iam-assumable-role-with-oidc"
	version      = "5.11.2"
	create_role  = true
	role_name    = "${local.name}-efs-csi-driver-sa-role"
	provider_url = module.eks.cluster_oidc_issuer_url
	oidc_fully_qualified_subjects = [
		"system:serviceaccount:kube-system:efs-csi-controller-sa",
		"system:serviceaccount:kube-system:efs-csi-node-sa"
	]

	tags = local.tags
}

