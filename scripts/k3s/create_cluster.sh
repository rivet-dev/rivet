#!/bin/sh
set -euf

k3d cluster create \
	--k3s-arg "--disable=traefik@server:0" \
	"rivet-$(bolt output namespace)"

# k3d cluster create "rivet-$(bolt output namespace)"

