#!/usr/bin/env bash
set -euf -o pipefail

export IP=$(jq -r '.resources[] | select(.module=="module.server" and .type=="linode_instance" and .name=="server") | .instances[0].attributes.ip_address' terraform.tfstate)
export PASSWORD=$(jq -r '.resources[] | select(.module=="module.server" and .type=="random_string" and .name=="server_root_pass") | .instances[0].attributes.result' terraform.tfstate)
echo root@$IP
echo $PASSWORD
