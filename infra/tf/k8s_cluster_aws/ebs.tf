module "ebs_csi_irsa_role" {
	source = "terraform-aws-modules/iam/aws//modules/iam-role-for-service-accounts-eks"

	role_name = "${local.name}-ebs-csi"
	attach_ebs_csi_policy = true

	oidc_providers = {
		ex = {
			provider_arn = module.eks.oidc_provider_arn
			namespace_service_accounts = ["kube-system:ebs-csi-controller-sa"]
		}
	}
}

resource "kubernetes_storage_class" "ebs_sc" {
	metadata {
		name = "ebs-sc"
	}

	storage_provisioner = "ebs.csi.aws.com"
	volume_binding_mode = "WaitForFirstConsumer"
}
