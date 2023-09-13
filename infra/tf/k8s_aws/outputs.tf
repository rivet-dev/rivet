output "region" {
	value = local.region
}

output "azs" {
	value = local.azs
}

output "vpc_id" {
	value = module.vpc.vpc_id
}

output "private_subnets" {
	value = module.vpc.private_subnets
}

output "intra_subnets" {
	value = module.vpc.intra_subnets
}

output "nat_public_ips" {
    value = toset(module.vpc.nat_public_ips)
}

