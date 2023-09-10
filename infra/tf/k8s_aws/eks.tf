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
			"rivet-${element(split("-", local.azs[i]), 2)}" => {
				selectors = [
					{ namespace = "rivet-*" }
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

