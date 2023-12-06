# Troubleshooting

## `Inconsistent dependency lock file`

This error happens when the required dependencies for a Terraform plan change.

In order to fix this, run this command:

```sh
cd infra/tf/my_tf_plan && terraform init -upgrade
```

Then try the original command again.

