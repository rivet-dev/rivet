#!/bin/sh
set -euf

k3d cluster delete "rivet-$(bolt output namespace)"
