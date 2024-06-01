# TODO: Wait until fargate is up
module "karpenter" {
	source = "terraform-aws-modules/eks/aws//modules/karpenter"
	version = "20.12.0"

	cluster_name = module.eks.cluster_name
	irsa_oidc_provider_arn = module.eks.oidc_provider_arn

	node_iam_role_additional_policies = {
		AmazonSSMManagedInstanceCore = "arn:aws:iam::aws:policy/AmazonSSMManagedInstanceCore"
	}

	# IRSA backwards compatability
	enable_irsa = true
	create_instance_profile = true
	create_iam_role = true
	iam_role_name = "KarpenterIRSA-${module.eks.cluster_name}"
	iam_role_description = "Karpenter IAM role for service account"
	iam_policy_name = "KarpenterIRSA-${module.eks.cluster_name}"
	iam_policy_description = "Karpenter IAM role for service account"

	tags = local.tags
}

resource "helm_release" "karpenter" {
	namespace = "karpenter"
	create_namespace = true

	name = "karpenter"
	repository = "oci://public.ecr.aws/karpenter"
	chart = "karpenter"
	version = "v0.32.10"

	values = [yamlencode({
		controller = {
			resources = {
				# Must be compatible with Fargate pod sizes:
				# 
				# https://docs.aws.amazon.com/eks/latest/userguide/fargate-pod-configuration.html
				limits = {
					cpu = "0.5"
					memory = "512M"
				}
			}
		}

		serviceAccount = {
			annotations = {
				"eks.amazonaws.com/role-arn" = module.karpenter.iam_role_arn
			}
		}

		settings = {
			clusterName = module.eks.cluster_name
			clusterEndpoint = module.eks.cluster_endpoint
			interruptionQueue = module.karpenter.queue_name
		}
	})]
}

resource "kubectl_manifest" "karpenter_node_class" {
	depends_on = [helm_release.karpenter]

	yaml_body = yamlencode({
		apiVersion = "karpenter.k8s.aws/v1beta1"
		kind = "EC2NodeClass"
		metadata = {
			name = "default"
		}
		spec = {
			amiFamily = "AL2"
			role = module.karpenter.node_iam_role_name
			subnetSelectorTerms = [
				{
					tags = {
						"karpenter.sh/discovery" = module.eks.cluster_name
					}
				}
			]
			securityGroupSelectorTerms = [
				{
					tags = {
						"karpenter.sh/discovery" = module.eks.cluster_name
					}
				}
			]
			tags = {
				"karpenter.sh/discovery" = module.eks.cluster_name
			}
		}
	})
}

resource "kubectl_manifest" "karpenter_node_pool" {
	depends_on = [helm_release.karpenter, kubectl_manifest.karpenter_node_class]

	yaml_body = yamlencode({
		apiVersion = "karpenter.sh/v1beta1"
		kind = "NodePool"
		metadata = {
			name = "default"
		}
		spec = {
			template = {
				spec = {
					nodeClassRef = {
						name = "default"
					}
					requirements = [
						# See recommended requirements:
						# https://karpenter.sh/v0.37/concepts/nodepools/#capacity-type

						{
							key = "topology.kubernetes.io/zone"
							operator = "In"
							values = local.azs
						},
						{
							key = "kubernetes.io/arch"
							operator = "In"
							values = ["amd64"]
						},
						{
							key = "kubernetes.io/os"
							operator = "In"
							values = ["linux"]
						},
						{
							key = "karpenter.sh/capacity-type"
							operator = "In"
							values = ["on-demand"]
						},
						{
							key = "karpenter.k8s.aws/instance-category"
							operator = "In"
							values   = ["c", "m", "r"]
						},
						{
							key = "karpenter.k8s.aws/instance-generation"
							operator = "Gt"
							values = ["2"]
						}
					]
				}
			}
			limits = {
				cpu = 1000
				memory = "1000Gi"
			}
			disruption = {
				# Never kill pods that are currently running
				consolidationPolicy = "WhenEmpty"
				consolidateAfter = "30s"
				# Don't kill nodes arbitrarily
				expireAfter = "Never"
				# TODO: If switching to WhenUnderutilized, add `budgets` here
			}
		}
	})
}
