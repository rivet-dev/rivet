# See https://github.com/terraform-aws-modules/terraform-aws-eks/blob/666603b6e531140d5d8fbd777cd90a7fbb8247dd/examples/eks_managed_node_group/main.tf

module "eks" {
	source = "terraform-aws-modules/eks/aws"
	version = "19.16.0"

	cluster_name = local.name
	cluster_version = local.cluster_version
	cluster_endpoint_public_access = true

	# IPV6
	cluster_ip_family = "ipv6"

	# We are using the IRSA created below for permissions
	# However, we have to deploy with the policy attached FIRST (when creating a fresh cluster)
	# and then turn this off after the cluster/node group is created. Without this initial policy,
	# the VPC CNI fails to assign IPs and nodes cannot join the cluster
	# See https://github.com/aws/containers-roadmap/issues/1666 for more context
	# TODO - remove this policy once AWS releases a managed version similar to AmazonEKS_CNI_Policy (IPv4)
	create_cni_ipv6_iam_policy = true

	cluster_addons = {
		coredns = {
			# https://docs.aws.amazon.com/eks/latest/userguide/managing-coredns.html
			addon_version = "v1.10.1-eksbuild.3"
		}
		kube-proxy = {
			# https://docs.aws.amazon.com/eks/latest/userguide/managing-kube-proxy.html
			addon_version = "v1.27.4-eksbuild.2"
		}
		vpc-cni = {
			# https://docs.aws.amazon.com/eks/latest/userguide/managing-vpc-cni.html
			addon_version = "v1.14.0-eksbuild.3"
			before_compute = true
			service_account_role_arn = module.vpc_cni_irsa.iam_role_arn
			configuration_values = jsonencode({
				env = {
					# Reference docs https://docs.aws.amazon.com/eks/latest/userguide/cni-increase-ip-addresses.html
					ENABLE_PREFIX_DELEGATION = "true"
					WARM_PREFIX_TARGET = "1"
				}
			})
		}
	}

	vpc_id = module.vpc.vpc_id
	subnet_ids = module.vpc.private_subnets
	control_plane_subnet_ids = module.vpc.intra_subnets

	# Fargate profiles use the cluster primary security group so these are not utilized
	create_cluster_security_group = false
	create_node_security_group = false

	eks_managed_node_group_defaults = {
		ami_type       = "AL2_x86_64"
		instance_types = ["m6i.large", "m5.large", "m5n.large", "m5zn.large"]

		# We are using the IRSA created below for permissions
		# However, we have to deploy with the policy attached FIRST (when creating a fresh cluster)
		# and then turn this off after the cluster/node group is created. Without this initial policy,
		# the VPC CNI fails to assign IPs and nodes cannot join the cluster
		# See https://github.com/aws/containers-roadmap/issues/1666 for more context
		iam_role_attach_cni_policy = true
	}

	eks_managed_node_groups = {
		# Default node group - as provided by AWS EKS
		default_node_group = {
			# By default, the module creates a launch template to ensure tags are propagated to instances, etc.,
			# so we need to disable it to use the default template provided by the AWS EKS managed node group service
			use_custom_launch_template = false

			disk_size = 50


			# Remote access cannot be specified with a launch template
			remote_access = {
				ec2_ssh_key               = module.key_pair.key_pair_name
				source_security_group_ids = [aws_security_group.remote_access.id]
			}
		}

		# Default node group - as provided by AWS EKS using Bottlerocket
		bottlerocket_default = {
			# By default, the module creates a launch template to ensure tags are propagated to instances, etc.,
			# so we need to disable it to use the default template provided by the AWS EKS managed node group service
			use_custom_launch_template = false

			ami_type = "BOTTLEROCKET_x86_64"
			platform = "bottlerocket"
		}

		# Adds to the AWS provided user data
		bottlerocket_add = {
			ami_type = "BOTTLEROCKET_x86_64"
			platform = "bottlerocket"

			# This will get added to what AWS provides
			bootstrap_extra_args = <<-EOT
			# extra args added

			[settings.kernel]
			lockdown = "integrity"
			EOT
		}

		# Custom AMI, using module provided bootstrap data
		bottlerocket_custom = {
			# Current bottlerocket AMI
			ami_id   = data.aws_ami.eks_default_bottlerocket.image_id
			platform = "bottlerocket"

			# Use module user data template to bootstrap
			enable_bootstrap_user_data = true
			# This will get added to the template
			bootstrap_extra_args = <<-EOT
			# The admin host container provides SSH access and runs with "superpowers".
			# It is disabled by default, but can be disabled explicitly.
			[settings.host-containers.admin]
			enabled = false

			# The control host container provides out-of-band access via SSM.
			# It is enabled by default, and can be disabled if you do not expect to use SSM.
			# This could leave you with no way to access the API and change settings on an existing node!
			[settings.host-containers.control]
			enabled = true

			# extra args added
			[settings.kernel]
			lockdown = "integrity"

			[settings.kubernetes.node-labels]
			label1 = "foo"
			label2 = "bar"

			[settings.kubernetes.node-taints]
			dedicated = "experimental:PreferNoSchedule"
			special = "true:NoSchedule"
			EOT
		}

		# Use a custom AMI
		custom_ami = {
			ami_type = "AL2_ARM_64"
			# Current default AMI used by managed node groups - pseudo "custom"
			ami_id = data.aws_ami.eks_default_arm.image_id

			# This will ensure the bootstrap user data is used to join the node
			# By default, EKS managed node groups will not append bootstrap script;
			# this adds it back in using the default template provided by the module
			# Note: this assumes the AMI provided is an EKS optimized AMI derivative
			enable_bootstrap_user_data = true

			instance_types = ["t4g.medium"]
		}

		# Complete
		complete = {
			name            = "complete-eks-mng"
			use_name_prefix = true

			subnet_ids = module.vpc.private_subnets

			min_size     = 1
			max_size     = 7
			desired_size = 1

			ami_id                     = data.aws_ami.eks_default.image_id
			enable_bootstrap_user_data = true

			pre_bootstrap_user_data = <<-EOT
			export FOO=bar
			EOT

			post_bootstrap_user_data = <<-EOT
			echo "you are free little kubelet!"
			EOT

			capacity_type        = "SPOT"
			force_update_version = true
			instance_types       = ["m6i.large", "m5.large", "m5n.large", "m5zn.large"]
			labels = {
				GithubRepo = "terraform-aws-eks"
				GithubOrg  = "terraform-aws-modules"
			}

			taints = [
				{
					key    = "dedicated"
					value  = "gpuGroup"
					effect = "NO_SCHEDULE"
				}
			]

			update_config = {
				max_unavailable_percentage = 33 # or set `max_unavailable`
			}

			description = "EKS managed node group example launch template"

			ebs_optimized           = true
			disable_api_termination = false
			enable_monitoring       = true

			block_device_mappings = {
				xvda = {
					device_name = "/dev/xvda"
					ebs = {
						volume_size           = 75
						volume_type           = "gp3"
						iops                  = 3000
						throughput            = 150
						encrypted             = true
						kms_key_id            = module.ebs_kms_key.key_arn
						delete_on_termination = true
					}
				}
			}

			metadata_options = {
				http_endpoint               = "enabled"
				http_tokens                 = "required"
				http_put_response_hop_limit = 2
				instance_metadata_tags      = "disabled"
			}

			create_iam_role          = true
			iam_role_name            = "eks-managed-node-group-complete-example"
			iam_role_use_name_prefix = false
			iam_role_description     = "EKS managed node group complete example role"
			iam_role_tags = {
				Purpose = "Protector of the kubelet"
			}
			iam_role_additional_policies = {
				AmazonEC2ContainerRegistryReadOnly = "arn:aws:iam::aws:policy/AmazonEC2ContainerRegistryReadOnly"
				additional                         = aws_iam_policy.node_additional.arn
			}

			schedules = {
				scale-up = {
					min_size     = 2
					max_size     = "-1" # Retains current max size
					desired_size = 2
					start_time   = "2023-03-05T00:00:00Z"
					end_time     = "2024-03-05T00:00:00Z"
					timezone     = "Etc/GMT+0"
					recurrence   = "0 0 * * *"
				},
				scale-down = {
					min_size     = 0
					max_size     = "-1" # Retains current max size
					desired_size = 0
					start_time   = "2023-03-05T12:00:00Z"
					end_time     = "2024-03-05T12:00:00Z"
					timezone     = "Etc/GMT+0"
					recurrence   = "0 12 * * *"
				}
			}

			tags = {
				ExtraTag = "EKS managed node group complete example"
			}
		}
	}

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

resource "aws_iam_policy" "node_additional" {
	name        = "${local.name}-additional"
	description = "Example usage of node additional policy"

	policy = jsonencode({
		Version = "2012-10-17"
		Statement = [
			{
				Action = [
					"ec2:Describe*",
				]
				Effect   = "Allow"
				Resource = "*"
			},
		]
	})

	tags = local.tags
}

data "aws_ami" "eks_default" {
	most_recent = true
	owners      = ["amazon"]

	filter {
		name   = "name"
		values = ["amazon-eks-node-${local.cluster_version}-v*"]
	}
}

data "aws_ami" "eks_default_arm" {
	most_recent = true
	owners      = ["amazon"]

	filter {
		name   = "name"
		values = ["amazon-eks-arm64-node-${local.cluster_version}-v*"]
	}
}

data "aws_ami" "eks_default_bottlerocket" {
	most_recent = true
	owners      = ["amazon"]

	filter {
		name   = "name"
		values = ["bottlerocket-aws-k8s-${local.cluster_version}-x86_64-*"]
	}
}

