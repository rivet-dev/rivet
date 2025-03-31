sysctl --system

mkdir -p /etc/rivet-server /etc/rivet-server/tls /var/lib/rivet-server

echo 'Downloading guard'
curl -Lf -o /usr/local/bin/rivet-guard "__GUARD_BINARY_URL__"
chmod +x /usr/local/bin/rivet-guard

# For clarity
FDB_VERSION="__FDB_VERSION__"

# Shared object for fdb client
echo 'Downloading fdb shared object'
curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/${FDB_VERSION}/libfdb_c.x86_64.so"
