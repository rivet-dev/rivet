# SaltStack

## Apply SaltStack configurations

To apply to all minions:

```bash
bin/salt/apply
```

To specify a specific [compound target](https://docs.saltproject.io/en/latest/topics/targeting/#compound-targeting), use the following:

```bash
bin/salt/apply '{target}'
```

e.g. to deploy to all NATS servers, we'd run the following:

```bash
bin/salt/apply 'G@roles:nats-server'
```

