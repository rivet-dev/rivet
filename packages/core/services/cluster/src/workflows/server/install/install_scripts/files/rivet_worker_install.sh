sysctl --system

mkdir -p /etc/rivet-server /var/lib/rivet-server

echo 'Downloading edge server'
curl -Lf -o /usr/local/bin/rivet-edge-server "__EDGE_SERVER_BINARY_URL__"
chmod +x /usr/local/bin/rivet-edge-server

# For clarity
FDB_VERSION="__FDB_VERSION__"

# Shared object for fdb client
echo 'Downloading fdb shared object'
curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/${FDB_VERSION}/libfdb_c.x86_64.so"
