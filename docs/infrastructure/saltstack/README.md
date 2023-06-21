# SaltStack

## Apply SaltStack configurations

To apply to all minions:

```bash
bolt salt apply
```

To specify a specific [compound target](https://docs.saltproject.io/en/latest/topics/targeting/#compound-targeting), use the following:

```bash
bolt salt apply '{target}'
```

e.g. to deploy to all NATS servers, we'd run the following:

```bash
bolt salt apply 'G@roles:nats-server'
```
