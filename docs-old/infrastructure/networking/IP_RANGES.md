# IP Ranges

See [`lib/bolt/core/src/dep/terraform/net.rs`](../../../lib/bolt/core/src/dep/terraform/net.rs)

## VLAN (Class A)

Allowed range:

| Name                          | Netmask     | Subnet count | Node count       |
| ----------------------------- | ----------- | ------------ | ---------------- |
| Entire VLAN                   | 10.0.0.0/8  | ~            | ~                |
| Region                        | 10.0.0.0/16 | 256          | ~                |
| Supporting services (GG, ATS) | 10.0.0.0/26 | 16           | 64 - 2           |
| Job                           | 10.0.4.0/16 | ~            | 65536 - 1024 - 2 |

We can't allocate the network or broadcast address, so we subtract 2 from each node count.
