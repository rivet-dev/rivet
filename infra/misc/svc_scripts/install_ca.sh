#!/bin/sh
set -euf

# Log to file
exec >> "/var/log/install-ca.txt" 2>&1

# Merge CA certificates provided from other config maps for self-signed TLS connections to databases
#
# Overriding LD_LIBRARY_PATH prevents apt from using the OpenSSL installation from /nix/store (if mounted).
LD_LIBRARY_PATH=/lib:/usr/lib:/usr/local/lib update-ca-certificates
