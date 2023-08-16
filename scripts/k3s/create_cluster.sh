#!/bin/sh
set -euf

k3d cluster create \
	# --k3s-arg "--disable=traefik@server:0" \
	-v /nix/store:/nix/store \
	-v /home:/home \
	-v /local:/local \
	"rivet-$(bolt output namespace)"
