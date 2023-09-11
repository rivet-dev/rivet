# # See https://github.com/terraform-aws-modules/terraform-aws-eks/blob/666603b6e531140d5d8fbd777cd90a7fbb8247dd/examples/karpenter/main.tf

module "eks" {
	source = "terraform-aws-modules/eks/aws"
	version = "19.16.0"

	cluster_name = local.name
	cluster_version = local.cluster_version
	cluster_endpoint_public_access = true

	# For the latest versions: aws eks describe-addon-versions
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
				# Ensure that we fully utilize the minimum amount of resources that are supplied by
				# Fargate https://docs.aws.amazon.com/eks/latest/userguide/fargate-pod-configuration.html
				# Fargate adds 256 MB to each pod's memory reservation for the required Kubernetes
				# components (kubelet, kube-proxy, and containerd). Fargate rounds up to the following
				# compute configuration that most closely matches the sum of vCPU and memory requests in
				# order to ensure pods always have the resources that they need to run.
				resources = {
					limits = {
						cpu = "0.25"
						# We are targeting the smallest Task size of 512Mb, so we subtract 256Mb from the
						# request/limit to ensure we can fit within that task
						memory = "256M"
					}
					requests = {
						cpu = "0.25"
						# We are targeting the smallest Task size of 512Mb, so we subtract 256Mb from the
						# request/limit to ensure we can fit within that task
						memory = "256M"
					}
				}
			})
		}
		aws-ebs-csi-driver = {
			addon_version = "v1.22.0-eksbuild.2"
			service_account_role_arn = module.ebs_csi_irsa_role.iam_role_arn
		}
	}

	vpc_id = module.vpc.vpc_id
	subnet_ids = module.vpc.private_subnets
	control_plane_subnet_ids = module.vpc.intra_subnets

	# Fargate profiles use the cluster primary security group so these are not utilized
	create_cluster_security_group = false
	create_node_security_group = false

	manage_aws_auth_configmap = true
	aws_auth_roles = [
		# We need to add in the Karpenter node IAM role for nodes launched by Karpenter
		{
			rolearn = module.karpenter.role_arn
			username = "system:node:{{EC2PrivateDNSName}}"
			groups = [
				"system:bootstrappers",
				"system:nodes",
			]
		},
	]

	fargate_profiles = {
		karpenter = {
			selectors = [
				{ namespace = "karpenter" }
			]
		}
		kube-system = {
			selectors = [
				{ namespace = "kube-system" }
			]
		}
	}

	tags = merge(local.tags, {
		# NOTE - if creating multiple security groups with this module, only tag the
		# security group that Karpenter should utilize with the following tag
		# (i.e. - at most, only one security group should have this tag in your account)
		"karpenter.sh/discovery" = local.name
	})
}

