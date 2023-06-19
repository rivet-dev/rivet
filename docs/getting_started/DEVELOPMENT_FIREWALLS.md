# Firewalls for developing locally

## Inbound

Whitelist the following ports:

| Protocol  | Port | Description       |
| --------- | ---- | ----------------- |
| TCP       | 22   | SSH               |
| TCP       | 80   | HTTP              |
| TCP       | 443  | HTTPS             |
| TCP & UDP | 4242 | Nebula lighthouse |

## Outbound

-   Allow all outbound
