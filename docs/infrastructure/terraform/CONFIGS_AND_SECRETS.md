# Configs & Secrets

## Configs

To pass a configuration from Bolt -> Terraform plan, it goes through these steps:

1. [Generate tfvars](/lib/bolt/core/src/dep/terraform/gen.rs)
2. [Read the variable name in vars.tf](/infra/tf/pools/vars.tf)

## Secrets

_TODO_
