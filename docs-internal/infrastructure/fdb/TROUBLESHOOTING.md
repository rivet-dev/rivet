# Troubleshooting

## Cannot connect to cluster

This may be caused by many reasons.

### `fdb.cluster` does not match between client and host

The client and host must be able to address the server with the same IP.

Validate that the client config (e.g. `/var/lib/rivet-client/fdb.cluster` on a Rivet client) matches the file `/var/fdb/fdb.cluster` on the FoundationDB server.

If you're using DNS to resolve the cluster, make sure that the DNS address resolves to the correct location.

