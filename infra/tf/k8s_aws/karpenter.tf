module "karpenter" {
	source = "terraform-aws-modules/eks/aws//modules/karpenter"
	version = "19.16.0"

	cluster_name           = module.eks.cluster_name
	irsa_oidc_provider_arn = module.eks.oidc_provider_arn

	policies = {
		AmazonSSMManagedInstanceCore = "arn:aws:iam::aws:policy/AmazonSSMManagedInstanceCore"
	}

	tags = local.tags
}

resource "helm_release" "karpenter" {
	namespace        = "karpenter"
	create_namespace = true

	name                = "karpenter"
	repository          = "oci://public.ecr.aws/karpenter"
	# repository_username = data.aws_ecrpublic_authorization_token.token.user_name
	# repository_password = data.aws_ecrpublic_authorization_token.token.password
	chart               = "karpenter"
	version             = "v0.30.0"

	set {
		name  = "settings.aws.clusterName"
		value = module.eks.cluster_name
	}

	set {
		name  = "settings.aws.clusterEndpoint"
		value = module.eks.cluster_endpoint
	}

	set {
		name  = "serviceAccount.annotations.eks\\.amazonaws\\.com/role-arn"
		value = module.karpenter.irsa_arn
	}

	set {
		name  = "settings.aws.defaultInstanceProfile"
		value = module.karpenter.instance_profile_name
	}

	set {
		name  = "settings.aws.interruptionQueueName"
		value = module.karpenter.queue_name
	}
}

resource "kubectl_manifest" "karpenter_provisioner" {
	depends_on = [helm_release.karpenter]

	yaml_body = yamlencode({
		apiVersion = "karpenter.sh/v1alpha5"
		kind       = "Provisioner"
		metadata = {
			name = "default"
		}
		spec = {
			requirements = [
				{
					key      = "karpenter.sh/capacity-type"
					operator = "In"
					values   = ["spot"]
				}
			]
			limits = {
				resources = {
					cpu = 1000
				}
			}
			providerRef = {
				name = "default"
			}
			ttlSecondsAfterEmpty = 30
		}
	})
}

resource "kubectl_manifest" "karpenter_node_template" {
	depends_on = [helm_release.karpenter]

	yaml_body = yamlencode({
		apiVersion = "karpenter.k8s.aws/v1alpha1"
		kind       = "AWSNodeTemplate"
		metadata = {
			name = "default"
		}
		spec = {
			subnetSelector = {
				"karpenter.sh/discovery" = module.eks.cluster_name
			}
			securityGroupSelector = {
				"karpenter.sh/discovery" = module.eks.cluster_name
			}
			tags = {
				"karpenter.sh/discovery" = module.eks.cluster_name
			}
		}
	})
}

# Example deployment using the [pause image](https://www.ianlewis.org/en/almighty-pause-container)
# and starts with zero replicas
resource "kubectl_manifest" "karpenter_example_deployment" {
	depends_on = [helm_release.karpenter]

	yaml_body = yamlencode({
		apiVersion = "apps/v1"
		kind       = "Deployment"
		metadata = {
			name = "inflate"
		}
		spec = {
			replicas = 0
			selector = {
				matchLabels = {
					app = "inflate"
				}
			}
			template = {
				metadata = {
					labels = {
						app = "inflate"
					}
				}
				spec = {
					terminationGracePeriodSeconds = 0
					containers = [
						{
							name = "inflate"
							image = "public.ecr.aws/eks-distro/kubernetes/pause:3.7"
							resources = {
								requests = {
									cpu = 1
								}
							}
						}
					]
				}
			}
		}
	})
}

