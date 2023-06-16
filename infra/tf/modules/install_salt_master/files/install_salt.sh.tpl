#!/usr/bin/env bash
set -euf

# Modified from https://repo.saltproject.io/#bootstrap
# For more options, see https://docs.saltproject.io/salt/install-guide/en/latest/topics/bootstrap.html

mkdir -p /opt/install_salt
cd /opt/install_salt

# Download
curl -fsSL https://bootstrap.saltproject.io -o bootstrap-salt.sh
curl -fsSL https://bootstrap.saltproject.io/sha256 -o bootstrap-salt.sh.sha256

# Verify file integrity
if sha256sum --check bootstrap-salt.sh.sha256; then
    echo "Success! Installing..."
    sudo sh bootstrap-salt.sh -D -M -P -U -L -i "${salt_master_name}" -A "127.0.0.1" stable ${version}

    echo "Installing self as minion..."
    salt-key --yes --accept "${salt_master_name}"
else
    echo "ERROR: This file is corrupt or has been tampered with."
    exit 1
fi
