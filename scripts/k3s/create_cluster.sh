#!/bin/sh
set -euf

k3d cluster create \
	-v /nix/store:/nix/store \
	-v /home:/home \
	-v /local:/local \
	-p "80:80" \
	-p "443:443" \
	--k3s-arg "--disable=traefik@server:0" \
	"rivet-$(bolt output namespace)"
