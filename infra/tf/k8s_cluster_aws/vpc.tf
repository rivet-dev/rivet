module "vpc" {
	source  = "terraform-aws-modules/vpc/aws"
	version = "5.1.2"

	name = local.name
	cidr = local.vpc_cidr

	azs = local.azs
	private_subnets = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 4, k)]
	public_subnets = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 48)]
	intra_subnets = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 52)]

	# Configure one NAT gateway per AZ
	#
	# For a cheaper configuration, see "Single NAT Gateway":
	# https://registry.terraform.io/modules/terraform-aws-modules/vpc/aws/latest#nat-gateway-scenarios
	enable_nat_gateway = true
	single_nat_gateway = false
	one_nat_gateway_per_az = true

	public_subnet_tags = {
		"kubernetes.io/role/elb" = 1
	}

	private_subnet_tags = {
		"kubernetes.io/role/internal-elb" = 1
		# Tags subnets for Karpenter auto-discovery
		"karpenter.sh/discovery" = local.name
	}

	tags = local.tags
}

