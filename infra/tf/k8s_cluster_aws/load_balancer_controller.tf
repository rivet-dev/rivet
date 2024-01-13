locals {
	eks_oidc_issuer_url = replace(module.eks.cluster_oidc_issuer_url, "https://", "")
}

# See https://docs.aws.amazon.com/eks/latest/userguide/aws-load-balancer-controller.html

data "http" "iam_policy" {
	url = "https://raw.githubusercontent.com/kubernetes-sigs/aws-load-balancer-controller/v2.6.2/docs/install/iam_policy.json"
}

resource "aws_iam_policy" "load_balancer_controller_policy" {
	name = "${local.name}-AWSLoadBalancerControllerIAMPolicy"
	description = "IAM policy for AWS Load Balancer Controller"
	policy = data.http.iam_policy.body
}

resource "aws_iam_role" "eks_load_balancer_role" {
	name = "${local.name}-AmazonEKSLoadBalancerControllerRole"

	assume_role_policy = jsonencode({
		Version = "2012-10-17",
		Statement = [
			{
				Effect = "Allow",
				Principal = {
					Federated = module.eks.oidc_provider_arn
				},
				Action = "sts:AssumeRoleWithWebIdentity",
				Condition = {
					StringEquals = {
						"${local.eks_oidc_issuer_url}:aud" = "sts.amazonaws.com"
						"${local.eks_oidc_issuer_url}:sub" = "system:serviceaccount:kube-system:aws-load-balancer-controller"
					}
				}
			}
		]
	})
}

resource "aws_iam_role_policy_attachment" "eks_load_balancer_policy_attachment" {
	role = aws_iam_role.eks_load_balancer_role.name
	policy_arn = aws_iam_policy.load_balancer_controller_policy.arn
}

resource "kubernetes_service_account" "aws_load_balancer_controller" {
	metadata {
		name = "aws-load-balancer-controller"
		namespace = "kube-system"
		annotations = {
			"eks.amazonaws.com/role-arn" = aws_iam_role.eks_load_balancer_role.arn
		}
	}
}

resource "helm_release" "load_balancer_controller" {
	name = "aws-load-balancer-controller"
	repository = "https://aws.github.io/eks-charts"
	chart = "aws-load-balancer-controller"
	# repository = "oci://public.ecr.aws/eks-charts"
	# chart = "aws-load-balancer-controller"
	namespace = "kube-system"
	# Corresponds to load balancer controller version 2.6.2
	version = "v1.6.2"

	values = [yamlencode({
		clusterName = module.eks.cluster_name
		vpcId = module.vpc.vpc_id
		serviceAccount = {
			create = false
			name = kubernetes_service_account.aws_load_balancer_controller.metadata.0.name
		}
	})]
}

