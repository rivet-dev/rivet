# Troubleshooting

## Checking the health of the cluster manually...

1. `bolt ssh pool nats`
2. `nix-shell -p natscli`
3. `nats --server=10.0.44.2:4222 --user admin --password password context save default`
4. `nats context select default`
5. `nats server report connections`
