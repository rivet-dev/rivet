#!/usr/bin/env bash
set -euf -o pipefail

# Read server host and password
export IP=$(jq -r '.resources[] | select(.module=="module.server" and .type=="linode_instance" and .name=="server") | .instances[0].attributes.ip_address' terraform.tfstate)
export PASSWORD=$(jq -r '.resources[] | select(.module=="module.server" and .type=="random_string" and .name=="server_root_pass") | .instances[0].attributes.result' terraform.tfstate)

# Destroy the infrastructure provisioned on the remote machine
nix-shell -p sshpass --run "$(cat <<'EOF1'
sshpass -p "$PASSWORD" ssh "root@$IP" "$(cat <<'EOF2'
    source /root/.nix-profile/etc/profile.d/nix.sh
    cd /root/backend
    nix-shell --run "bolt infra destroy --yes"
EOF2
)"
EOF1
)"
