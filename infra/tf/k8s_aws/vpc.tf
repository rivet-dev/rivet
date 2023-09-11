module "vpc" {
	source  = "terraform-aws-modules/vpc/aws"
	version = "5.1.2"

	name = local.name
	cidr = local.vpc_cidr

	azs             = local.azs
	private_subnets = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 4, k)]
	public_subnets  = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 48)]
	intra_subnets   = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 52)]

	enable_nat_gateway     = true
	single_nat_gateway     = true
	enable_ipv6            = true
	create_egress_only_igw = true

	public_subnet_ipv6_prefixes                    = [0, 1, 2]
	public_subnet_assign_ipv6_address_on_creation  = true
	private_subnet_ipv6_prefixes                   = [3, 4, 5]
	private_subnet_assign_ipv6_address_on_creation = true
	intra_subnet_ipv6_prefixes                     = [6, 7, 8]
	intra_subnet_assign_ipv6_address_on_creation   = true

	public_subnet_tags = {
		"kubernetes.io/role/elb" = 1
	}

	private_subnet_tags = {
		"kubernetes.io/role/internal-elb" = 1
	}

	tags = local.tags
}

module "vpc_cni_irsa" {
	source  = "terraform-aws-modules/iam/aws//modules/iam-role-for-service-accounts-eks"
	version = "5.30.0"

	role_name_prefix      = "VPC-CNI-IRSA"
	attach_vpc_cni_policy = true
	vpc_cni_enable_ipv6   = true

	oidc_providers = {
		main = {
			provider_arn               = module.eks.oidc_provider_arn
			namespace_service_accounts = ["kube-system:aws-node"]
		}
	}

	tags = local.tags
}

resource "aws_security_group" "remote_access" {
	name_prefix = "${local.name}-remote-access"
	description = "Allow remote SSH access"
	vpc_id      = module.vpc.vpc_id

	ingress {
		description = "SSH access"
		from_port   = 22
		to_port     = 22
		protocol    = "tcp"
		cidr_blocks = ["10.0.0.0/8"]
	}

	egress {
		from_port        = 0
		to_port          = 0
		protocol         = "-1"
		cidr_blocks      = ["0.0.0.0/0"]
		ipv6_cidr_blocks = ["::/0"]
	}

	tags = merge(local.tags, { Name = "${local.name}-remote" })
}
