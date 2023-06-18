# How to deploy a new version

1. Validate `tf/env/${ns}.tfvars` is up to date
1. `bin/tf/apply tls`
1. `bin/tf/apply s3`
1. `bin/tf/apply grafana`
1. `bin/tf/apply infra`
1. `bin/salt/apply`
1. `bin/tf/apply nomad`
1. `bin/migrate/up`
1. `bolt up`
