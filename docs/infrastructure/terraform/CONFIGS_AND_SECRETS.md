# Configs & Secrets

## Configs

To pass a configuration from Bolt -> Terraform plan, it goes through these steps:

1. [Generate tfvars](/lib/bolt/core/src/dep/terraform/gen.rs)
2. [Read the variable name in vars.tf](infra/tf/pools/vars.tf)

## Secrets

To pass a secret from Bolt -> Terraform state, it goes through these steps:

1. [Add `secrets` module](/infra/tf/pools/main.tf)
2. [Read the secret from module.secrets.values](/lib/bolt/core/src/dep/salt/cli.rs)
3. [Read the module output](/infra/tf/pools/providers.tf)
