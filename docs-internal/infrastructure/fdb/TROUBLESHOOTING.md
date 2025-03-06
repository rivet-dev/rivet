# Troubleshooting

## Cannot connect to cluster

This may be caused by many reasons.

### `fdb.cluster` does not match between client and host

The client and host must be able to address the server with the same IP.

Validate that the client config (e.g. `/var/lib/rivet-client/fdb.cluster` on a Rivet client) matches the file `/var/fdb/fdb.cluster` on the FoundationDB server.

If you're using DNS to resolve the cluster, make sure that the DNS address resolves to the correct location.

### `Illegal instruction`

This usually means you're trying to use the AVX version of FoundationDB on an
unsupported system. This also happens when attempting to run AVX FoundationDB
on an Apple Silicon chip, since Docker's x86 emulation does not support AVX.

See [here](./AVX.md) for more information.

## `FdbBindingError::BadCode`

This usually indicates that `unpack` was not correct.

If reading ranges, double check that the subspace that the `unpack` from is what you expect.

