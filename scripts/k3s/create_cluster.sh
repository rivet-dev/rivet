#!/bin/sh
set -euf

# Mount repository in to k3d so we can access the built binaries
#
# Mount the /nix/store and /local since the build binaries depend on dynamic libs from there
k3d cluster create \
	-v "$(bolt output project-root):/rivet-src" \
	-v /nix/store:/nix/store \
	-v /local:/local \
	-p "80:80" \
	-p "443:443" \
	--k3s-arg "--disable=traefik@server:0" \
	"rivet-$(bolt output namespace)"
