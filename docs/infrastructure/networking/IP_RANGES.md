# IP Ranges

See [`lib/bolt/core/src/dep/terraform/net.rs`](../../../lib/bolt/core/src/dep/terraform/net.rs)

## VLAN (Class B)

Allowed range:

```
172.16.0.0/12
```

Region netmask: 18
Allows for 64 regions

Pool netmask: 24
Allows for 64 pools
Allows for 254 hosts (since we can't allocate the network or broadcast address)

Pool netmasks can be flexible to take up multiple pools if there needs to be more than 254 nodes in a pool

## Nebula (Class A)

Allowed range:

```
10.0.0.0/8
```

### svc

Allowed range:

```
10.0.0.0/12
```

Region netmask: 18 (see above)
Pool netmask: 24 (see above)

### job

```
10.16.0.0/12
```

Allows for 1,048,576 game server nodes

We'll build an IP address allocator for each node that gets created
