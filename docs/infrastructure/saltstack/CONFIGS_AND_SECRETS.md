# Configs & Secrets

## Configs

To pass a configuration from Bolt -> SaltStack state, it goes through these steps:

1. [Generate config](/lib/bolt/core/src/dep/salt/config.rs)
2. [Write config to /srv/salt-context/rivet/config.json](/lib/bolt/core/src/dep/salt/cli.rs)
3. [Read config from pillar](/infra/salt/pillar/rivet/init.sls)

## Secrets

To pass a secret from Bolt -> SaltStack state, it goes through these steps:

1. [Generate config](/lib/bolt/core/src/dep/salt/secrets.rs)
2. [Write config to /srv/salt-context/rivet/secrets.json](/lib/bolt/core/src/dep/salt/cli.rs)
3. [Read config from pillar](/infra/salt/pillar/clickhouse/init.sls)
