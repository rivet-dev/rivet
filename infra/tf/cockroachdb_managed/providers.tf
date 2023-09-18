provider "cockroach" {
	apikey = module.secrets.values["cockroachdb_coud/api_key"]
}

provider "kubernetes" {
	host = data.terraform_remote_state.k8s_aws.outputs.eks_cluster_endpoint
	cluster_ca_certificate = base64decode(
		data.terraform_remote_state.k8s_aws.outputs.eks_ca
	)

	exec {
		api_version = "client.authentication.k8s.io/v1beta1"
		command = "aws"
		args = [
			"eks",
			"get-token",
			"--cluster-name",
			data.terraform_remote_state.k8s_aws.outputs.eks_cluster_name
		]
	}
}
