# Troubleshooting

## If trying to diagnose a Minio-specific issue...

1. `nix-shell -p minio-client`
2. `mc alias set local http://127.0.0.1:9200 root`
3. `mc admin trace -e local`

See [here](https://min.io/docs/minio/linux/reference/minio-mc-admin/mc-admin-trace.html#description) for more
options.
